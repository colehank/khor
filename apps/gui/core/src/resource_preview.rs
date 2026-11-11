//! Read-only HTTP origins for file previews. Each preview owns a distinct port,
//! a capability, and all connection tasks. No app APIs/cookies live at this origin.
//! Bodies pull bounded slices through the existing authenticated node transport;
//! dropping a body stops future reads, and closing the preview aborts its sockets.
use std::{collections::HashMap, convert::Infallible, path::Path, sync::{Arc,Mutex,OnceLock}, time::{Duration,Instant}};
use bytes::Bytes;
use http_body_util::{BodyExt,Full,StreamBody,combinators::UnsyncBoxBody};
use hyper::{Request,Response,StatusCode,body::{Incoming,Frame},header};
use khor_node::Node;
use serde::Serialize;
use ts_rs::TS;

type Body=UnsyncBoxBody<Bytes,std::io::Error>;
const LIFETIME:Duration=Duration::from_secs(300);
const MOST_PREVIEWS:usize=32;
struct Lease { touched:Arc<Mutex<Instant>>, task:tokio::task::AbortHandle }
static LEASES:OnceLock<Mutex<HashMap<String,Lease>>>=OnceLock::new();
fn leases()-> &'static Mutex<HashMap<String,Lease>> { LEASES.get_or_init(Mutex::default) }
#[derive(Debug,Serialize,TS)]
pub struct ResourcePreview { pub id:String, pub url:String }
struct Context {
    node:Arc<Node>,machine:String,anchor:String,entry:String,html:Option<Bytes>,
    id:String,port:u16,touched:Arc<Mutex<Instant>>,
}

pub async fn open(root:&Path,machine:&str,path:&str,html:Option<String>)->Result<ResourcePreview,String> {
    let node=Arc::new(Node::open(root.to_path_buf())?);
    let meta=node.file_resource(machine,path,"",None).await?;
    if html.as_ref().is_some_and(|s|s.len()>8*1024*1024) { return Err("HTML preview buffer is too large".into()); }
    let listener=tokio::net::TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED,0)).await.map_err(|e|e.to_string())?;
    let port=listener.local_addr().map_err(|e|e.to_string())?.port();
    let mut secret=[0u8;32];getrandom::fill(&mut secret).map_err(|e|e.to_string())?;
    let id=secret.iter().map(|b|format!("{b:02x}")).collect::<String>();
    let entry=meta.entry.split('/').map(|p|urlencoding::encode(p).into_owned()).collect::<Vec<_>>().join("/");
    let url=format!("http://127.0.0.1:{port}/_khor/{id}/{entry}");
    let touched=Arc::new(Mutex::new(Instant::now()));
    let context=Arc::new(Context { node,machine:machine.into(),anchor:path.into(),entry:meta.entry,html:html.map(|s|Bytes::from(s.into_bytes())),id:id.clone(),port,touched:touched.clone() });
    // Admission is checked while holding the same table used by close/touch.
    let mut table=leases().lock().unwrap_or_else(|p|p.into_inner());
    table.retain(|_,lease|{ let live=lease.touched.lock().unwrap_or_else(|p|p.into_inner()).elapsed()<LIFETIME; if !live {lease.task.abort();} live });
    if table.len()>=MOST_PREVIEWS { return Err("too many open file previews".into()); }
    let task=tokio::spawn(serve(listener,context));
    table.insert(id.clone(),Lease {touched,task:task.abort_handle()});
    Ok(ResourcePreview {id,url})
}
pub fn touch(id:&str)->Result<(),String> {
    let table=leases().lock().unwrap_or_else(|p|p.into_inner());
    let lease=table.get(id).ok_or("file preview has expired")?;
    if lease.task.is_finished() {return Err("file preview has expired".into());}
    *lease.touched.lock().unwrap_or_else(|p|p.into_inner())=Instant::now();Ok(())
}
pub fn close(id:&str) {
    if let Some(lease)=leases().lock().unwrap_or_else(|p|p.into_inner()).remove(id) {lease.task.abort();}
}
async fn serve(listener:tokio::net::TcpListener,context:Arc<Context>) {
    let mut clients=tokio::task::JoinSet::new();
    let mut tick=tokio::time::interval(Duration::from_secs(30));
    loop { tokio::select! {
        _=tick.tick()=>{if context.touched.lock().unwrap_or_else(|p|p.into_inner()).elapsed()>=LIFETIME {break;}},
        Some(_)=clients.join_next(),if !clients.is_empty()=>{},
        socket=listener.accept()=>{let Ok((socket,_))=socket else{break};
            if clients.len()>=64 {drop(socket);continue;}
            let context=context.clone();clients.spawn(async move {
                let service=hyper::service::service_fn(move |request|{let context=context.clone();async move { Ok::<_,Infallible>(answer(context,request).await) }});
                let _=hyper::server::conn::http1::Builder::new().header_read_timeout(Duration::from_secs(15)).timer(hyper_util::rt::TokioTimer::new())
                    .serve_connection(hyper_util::rt::TokioIo::new(socket),service).await;
            });
        }
    }}
    // Dropping the JoinSet also aborts bodies/reads if this server itself is aborted.
    clients.abort_all();
}
fn empty(code:StatusCode)->Response<Body> {Response::builder().status(code).body(Full::new(Bytes::new()).map_err(|never|match never{}).boxed_unsync()).unwrap()}
fn header_text<'a>(req:&'a Request<Incoming>,key:header::HeaderName)->Option<&'a str>{req.headers().get(key)?.to_str().ok()}
fn valid_host(host:&str,port:u16)->bool {
    let Ok(url)=url::Url::parse(&format!("http://{host}")) else{return false};
    url.port_or_known_default()==Some(port) && (url.host_str()==Some("localhost") || url.host_str().is_some_and(|s|s.parse::<std::net::IpAddr>().is_ok()))
}
/// A single RFC 9110 byte range. Multipart ranges are explicitly refused rather
/// than silently seeking to a wrong position. A missing/stale If-Range uses 200.
fn byte_range(value:Option<&str>,total:u64)->Result<Option<(u64,u64)>,()> {
    let Some(value)=value else{return Ok(None)};
    let (from,to)=value.strip_prefix("bytes=").ok_or(())?.split_once('-').ok_or(())?;
    if total==0 || value.contains(','){return Err(());}
    if from.is_empty(){let count=to.parse::<u64>().map_err(|_|())?;if count==0{return Err(());}return Ok(Some((total.saturating_sub(count),total-1)));}
    let start=from.parse::<u64>().map_err(|_|())?;
    let end=if to.is_empty(){total-1}else{to.parse::<u64>().map_err(|_|())?.min(total-1)};
    if start>=total || end<start{return Err(());}Ok(Some((start,end)))
}
async fn answer(context:Arc<Context>,req:Request<Incoming>)->Response<Body> {
    if !matches!(*req.method(),hyper::Method::GET|hyper::Method::HEAD){return empty(StatusCode::METHOD_NOT_ALLOWED);}
    if !header_text(&req,header::HOST).is_some_and(|s|valid_host(s,context.port)){return empty(StatusCode::FORBIDDEN);}
    let prefix=format!("/_khor/{}/",context.id);
    let cookie_name=format!("khor_preview_{}",&context.id[..16]);
    let cookie=format!("{cookie_name}={}",context.id);
    let capability=req.uri().path().strip_prefix(&prefix);
    let allowed=capability.is_some() || header_text(&req,header::COOKIE).is_some_and(|s|s.split(';').any(|c|c.trim()==cookie));
    if !allowed{return empty(StatusCode::NOT_FOUND);}
    *context.touched.lock().unwrap_or_else(|p|p.into_inner())=Instant::now();
    let raw=capability.unwrap_or(req.uri().path().trim_start_matches('/'));
    let Ok(decoded)=urlencoding::decode(raw) else{return empty(StatusCode::BAD_REQUEST)};
    let resource=if decoded.is_empty() || decoded.ends_with('/') {format!("{decoded}index.html")}else{decoded.into_owned()};
    let buffer=context.html.as_ref().filter(|_|resource==context.entry);
    let (total,at_ms)=if let Some(buffer)=buffer {(buffer.len() as u64,0)}else{
        match context.node.file_resource(&context.machine,&context.anchor,&resource,None).await {Ok(slice)=>(slice.total,slice.at_ms),Err(_)=>return empty(StatusCode::NOT_FOUND)}
    };
    let etag=if let Some(buffer)=buffer{format!("\"{}\"",blake3::hash(buffer))}else{format!("\"{total:x}-{at_ms:x}\"")};
    let mut reply=Response::builder().header(header::CONTENT_TYPE,media_type(&resource)).header(header::ACCEPT_RANGES,"bytes")
        .header(header::CACHE_CONTROL,"no-store").header(header::ETAG,&etag)
        .header("X-Content-Type-Options","nosniff").header("Referrer-Policy","no-referrer")
        .header(header::SET_COOKIE,format!("{cookie}; Path=/; HttpOnly; SameSite=Strict"));
    let requested=if header_text(&req,header::IF_RANGE).is_some_and(|value|value!=etag){None}else{header_text(&req,header::RANGE)};
    let range=match byte_range(requested,total){Ok(range)=>range,Err(())=>return reply.status(StatusCode::RANGE_NOT_SATISFIABLE).header(header::CONTENT_RANGE,format!("bytes */{total}")).body(empty(StatusCode::OK).into_body()).unwrap()};
    let (start,end)=range.unwrap_or((0,total.saturating_sub(1)));
    let length=if total==0{0}else{end-start+1};
    if range.is_some(){reply=reply.status(StatusCode::PARTIAL_CONTENT).header(header::CONTENT_RANGE,format!("bytes {start}-{end}/{total}"));}
    reply=reply.header(header::CONTENT_LENGTH,length);
    if req.method()==hyper::Method::HEAD || length==0 {return reply.body(empty(StatusCode::OK).into_body()).unwrap();}
    if let Some(buffer)=buffer {return reply.body(Full::new(buffer.slice(start as usize..(end+1) as usize)).map_err(|never|match never{}).boxed_unsync()).unwrap();}
    let stream=futures::stream::try_unfold((context,resource,start,length),move |(context,resource,offset,left)|async move{
        if left==0{return Ok(None);}
        let slice=context.node.file_resource(&context.machine,&context.anchor,&resource,Some(offset)).await.map_err(std::io::Error::other)?;
        if slice.total!=total || slice.at_ms!=at_ms || slice.bytes.is_empty(){return Err(std::io::Error::other("file changed during playback"));}
        let mut bytes=slice.bytes.into_vec();bytes.truncate(left.min(bytes.len() as u64) as usize);
        let count=bytes.len() as u64;
        *context.touched.lock().unwrap_or_else(|p|p.into_inner())=Instant::now();
        Ok(Some((Frame::data(Bytes::from(bytes)),(context,resource,offset+count,left-count))))
    });
    reply.body(StreamBody::new(stream).boxed_unsync()).unwrap()
}
fn media_type(path:&str)-> &'static str {
    match path.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str(){
        "html"|"htm"=>"text/html; charset=utf-8","js"|"mjs"=>"text/javascript; charset=utf-8","css"=>"text/css; charset=utf-8",
        "json"|"map"=>"application/json","wasm"=>"application/wasm","svg"=>"image/svg+xml","png"=>"image/png","jpg"|"jpeg"=>"image/jpeg","gif"=>"image/gif","webp"=>"image/webp","avif"=>"image/avif","ico"=>"image/x-icon",
        "mp4"|"m4v"=>"video/mp4","webm"=>"video/webm","mov"=>"video/quicktime","ogv"=>"video/ogg","mp3"=>"audio/mpeg","m4a"|"aac"=>"audio/mp4","wav"=>"audio/wav","ogg"|"oga"|"opus"=>"audio/ogg","flac"=>"audio/flac",
        "woff"=>"font/woff","woff2"=>"font/woff2","ttf"=>"font/ttf","otf"=>"font/otf","pdf"=>"application/pdf",_=>"application/octet-stream"
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn ranges(){assert_eq!(byte_range(Some("bytes=2-4"),10),Ok(Some((2,4))));assert_eq!(byte_range(Some("bytes=-3"),10),Ok(Some((7,9))));assert_eq!(byte_range(Some("bytes=8-"),10),Ok(Some((8,9))));assert_eq!(byte_range(Some("bytes=1-999"),10),Ok(Some((1,9))));for r in ["bytes=10-","bytes=3-2","bytes=-0","bytes=0-1,3-4","cats=0-1"]{assert!(byte_range(Some(r),10).is_err());}assert!(byte_range(Some("bytes=0-"),0).is_err());}
    #[test]fn origins(){assert!(valid_host("127.0.0.1:1234",1234));assert!(valid_host("192.168.1.3:1234",1234));assert!(!valid_host("evil.example:1234",1234));assert!(!valid_host("127.0.0.1:1235",1234));}
}
