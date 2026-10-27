//! Native A2A HTTP targets. The remote machine needs an A2A agent, not khor.
//! Local registrations and credentials never join the replicated ACP registry.
use crate::{
    Node,
    delegation::{Action, Agent, AgentTarget, Answer, Record, TaskRef, protocol as p},
};
use a2a_client::{A2AClient, A2AClientFactory, Transport};
use khor_core::DeviceId;
use reqwest::{
    Client, Url,
    header::{AUTHORIZATION, HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

const PREFIX: &str = "a2a:";
const MAX_FILE: u64 = 32 * 1024 * 1024;
const MAX_TASK: usize = 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub name: String,
    pub card_url: String,
    pub card: p::AgentCard,
    pub interface: p::AgentInterface,
    credential: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Receipt {
    record: Record,
    endpoint: Endpoint,
}
#[derive(Clone, Serialize, Deserialize)]
struct Input {
    fingerprint: String,
    request: String,
    caller: DeviceId,
    message: p::Message,
    parent: Option<TaskRef>,
    endpoint: Endpoint,
    response: Option<p::SendMessageResponse>,
}
#[derive(Default, Serialize, Deserialize)]
struct Ledger {
    version: u32,
    inputs: BTreeMap<String, Input>,
    tasks: BTreeMap<String, Receipt>,
}
impl Ledger {
    fn owns_context(
        &self,
        caller: DeviceId,
        agent: &str,
        endpoint: &Endpoint,
        context: &str,
    ) -> bool {
        if endpoint.name != agent {
            return false;
        }
        let same_service =
            |saved: &Endpoint| saved.name == agent && saved.card_url == endpoint.card_url;
        if self.tasks.values().any(|receipt| {
            receipt.record.caller == caller
                && receipt.record.target.agent == agent
                && receipt.record.task.context_id == context
                && same_service(&receipt.endpoint)
        }) {
            return true;
        }
        // Inputs are durable receipts, not an expiring dedup cache. Future
        // pruning must preserve context grants before removing these responses.
        self.inputs.values().any(|input| {
            input.caller == caller
                && same_service(&input.endpoint)
                && matches!(&input.response, Some(p::SendMessageResponse::Message(message))
                    if message.context_id.as_deref() == Some(context))
        })
    }
}
struct Files {
    directory: PathBuf,
}
impl Files {
    fn new(root: &Path, agent: &str) -> Result<Self, String> {
        alias(agent)?;
        let directory = root
            .join(".khor/a2a/http")
            .join(blake3::hash(agent.as_bytes()).to_hex().as_str());
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&directory).map_err(|e| e.to_string())?;
        Ok(Self { directory })
    }
    fn read<T: DeserializeOwned>(&self, leaf: &str) -> Result<Option<T>, String> {
        let mut file = match File::open(self.directory.join(leaf)) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(MAX_FILE + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_FILE {
            return Err("A2A local record exceeds its size limit".into());
        }
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| "A2A local record is damaged; refusing to reset it".into())
    }
    fn write(&self, leaf: &str, value: &impl Serialize) -> Result<(), String> {
        let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_FILE {
            return Err("A2A local ledger is full".into());
        }
        let temporary = self.directory.join(format!("{}.tmp", p::new_task_id()));
        let result = (|| {
            let mut file = private(&temporary)?;
            file.write_all(&bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            fs::rename(&temporary, self.directory.join(leaf)).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            File::open(&self.directory)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
    fn transaction<T>(
        &self,
        action: impl FnOnce(&mut Ledger) -> Result<T, String>,
    ) -> Result<T, String> {
        let gate = private(&self.directory.join("ledger.lock"))?;
        gate.lock().map_err(|e| e.to_string())?;
        let mut ledger = self.read::<Ledger>("ledger.json")?.unwrap_or(Ledger {
            version: 1,
            ..Default::default()
        });
        if ledger.version != 1 {
            return Err("Unsupported A2A local ledger version".into());
        }
        let result = action(&mut ledger)?;
        self.write("ledger.json", &ledger)?;
        Ok(result)
    }
    fn endpoint(&self) -> Result<Endpoint, String> {
        self.read("endpoint.json")?
            .ok_or("A2A agent is not connected".into())
    }
    fn receipt(&self, caller: DeviceId, task: &str) -> Result<Receipt, String> {
        self.transaction(|l| {
            l.tasks
                .get(&task_key(caller, task))
                .cloned()
                .ok_or("No task owned by this caller for this A2A agent".into())
        })
    }
}
fn private(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(|e| e.to_string())
}
fn alias(agent: &str) -> Result<&str, String> {
    let name = agent
        .strip_prefix(PREFIX)
        .ok_or("A2A targets use a2a:<name>")?;
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("A2A name must contain 1–64 letters, numbers, hyphens or underscores".into());
    }
    Ok(name)
}
pub fn is_target(agent: &str) -> bool {
    agent.starts_with(PREFIX)
}
fn url(value: &str) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|_| "A2A URL must be absolute HTTP(S)".to_owned())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(
            "A2A URLs must use HTTP(S), with no embedded credentials, query or fragment".into(),
        );
    }
    Ok(url)
}
fn card_url(value: &str) -> Result<Url, String> {
    let mut value = url(value)?;
    if !value.path().ends_with(".json") {
        let path = format!(
            "{}/.well-known/agent-card.json",
            value.path().trim_end_matches('/')
        );
        value.set_path(&path);
    }
    Ok(value)
}
fn task_key(caller: DeviceId, id: &str) -> String {
    hash(&(caller, id))
}
pub(crate) fn wire_id(home: DeviceId, caller: DeviceId, agent: &str, request: &str) -> String {
    format!("khor-{}", hash(&(home, caller, agent, request)))
}
fn hash(value: &impl Serialize) -> String {
    fn ordered(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => serde_json::Value::Object(
                map.into_iter()
                    .map(|(k, v)| (k, ordered(v)))
                    .collect::<BTreeMap<_, _>>()
                    .into_iter()
                    .collect(),
            ),
            serde_json::Value::Array(items) => {
                serde_json::Value::Array(items.into_iter().map(ordered).collect())
            }
            other => other,
        }
    }
    blake3::hash(
        &serde_json::to_vec(&ordered(
            serde_json::to_value(value).expect("serializable A2A identity"),
        ))
        .expect("serializable A2A identity"),
    )
    .to_hex()
    .to_string()
}
fn opaque(value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        return Err("A2A ID is missing or too long".into());
    }
    Ok(())
}
fn credential(root: &Path, endpoint: &Endpoint) -> Result<Option<String>, String> {
    // A credential rotation applies to existing tasks without moving their
    // snapshotted protocol interface. Disconnect retains the prior credential.
    let current = Files::new(root, &endpoint.name)?.read::<Endpoint>("endpoint.json")?;
    let endpoint = current
        .as_ref()
        .filter(|e| e.card_url == endpoint.card_url)
        .unwrap_or(endpoint);
    endpoint
        .credential
        .as_ref()
        .map(|id| {
            if id.len() != 32 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("Invalid A2A credential reference".into());
            }
            fs::read_to_string(root.join(".khor/a2a/http/credentials").join(id))
                .map_err(|_| "A2A credential is unavailable on this machine".into())
        })
        .transpose()
}
fn no_secret(value: &impl Serialize, token: Option<&str>) -> Result<(), String> {
    if let Some(token) = token {
        let encoded = serde_json::to_string(token).map_err(|e| e.to_string())?;
        if serde_json::to_string(value)
            .map_err(|e| e.to_string())?
            .contains(&encoded[1..encoded.len() - 1])
        {
            return Err("A2A agent returned credential material; response withheld".into());
        }
    }
    Ok(())
}
fn http(token: Option<&str>, origin: &Url) -> Result<Client, String> {
    let mut headers = HeaderMap::new();
    if let Some(token) = token {
        let loopback = origin.host_str().is_some_and(|host| {
            host == "localhost"
                || host
                    .trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback())
        });
        if origin.scheme() != "https" && !loopback {
            return Err("A2A bearer credentials require HTTPS or loopback HTTP".into());
        }
        let mut value = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| "Invalid bearer token".to_owned())?;
        value.set_sensitive(true);
        headers.insert(AUTHORIZATION, value);
    }
    Client::builder()
        .default_headers(headers)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(6))
        .connect_timeout(Duration::from_secs(3))
        .build()
        .map_err(|_| "Cannot start A2A HTTP client".into())
}
async fn client(root: &Path, endpoint: &Endpoint) -> Result<A2AClient<Box<dyn Transport>>, String> {
    let origin = url(&endpoint.card_url)?;
    let selected = url(&endpoint.interface.url)?;
    if origin.origin() != selected.origin() {
        return Err("A2A interface changed origin; connect that service explicitly".into());
    }
    let token = credential(root, endpoint)?;
    let http = http(token.as_deref(), &origin)?;
    let mut card = endpoint.card.clone();
    card.supported_interfaces = vec![endpoint.interface.clone()];
    A2AClientFactory::builder()
        .no_defaults()
        .register(Arc::new(a2a_client::jsonrpc::JsonRpcTransportFactory::new(
            Some(http.clone()),
        )))
        .register(Arc::new(a2a_client::rest::RestTransportFactory::new(Some(
            http,
        ))))
        .build()
        .create_from_card(&card)
        .await
        .map_err(protocol_error)
}
fn protocol_error(error: p::A2AError) -> String {
    // Server details and HTTP bodies can echo credentials. Codes are safe and
    // retain the standard error distinction without handing secrets to Pi.
    format!(
        "A2A {} ({}); check the service and its authentication",
        p::errors::error_reason(error.code),
        error.code
    )
}
fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())
}

impl Node {
    /// Register a native HTTP agent locally. Secret input belongs to CLI stdin,
    /// never to replicated documents, MCP arguments or model tool output.
    pub fn connect_a2a(
        &self,
        name: &str,
        address: &str,
        bearer: Option<&str>,
    ) -> Result<Endpoint, String> {
        self.refuse_unless_hosting()?;
        let name = if is_target(name) {
            name.to_owned()
        } else {
            format!("{PREFIX}{name}")
        };
        let files = Files::new(self.root(), &name)?;
        let gate = private(&files.directory.join("connect.lock"))?;
        gate.lock().map_err(|e| e.to_string())?;
        let address = card_url(address)?;
        if let Some(prior) = files.read::<String>("service.json")? {
            if prior != address.as_str() {
                return Err("This A2A name belongs to another service; choose a new name".into());
            }
        }
        let previous = files.read::<Endpoint>("endpoint.json")?;
        let old = previous
            .as_ref()
            .map(|e| credential(self.root(), e))
            .transpose()?
            .flatten();
        let token = bearer.or(old.as_deref());
        if token.is_some_and(|t| t.is_empty() || t.len() > 8192) {
            return Err("A2A token must contain 1–8192 characters".into());
        }
        let (card, interface) = runtime()?.block_on(async {
            let http = http(token, &address)?;
            let mut response = http
                .get(address.clone())
                .header("A2A-Version", p::VERSION)
                .send()
                .await
                .map_err(|_| "A2A Agent Card could not be reached".to_owned())?;
            if !response.status().is_success() {
                return Err(format!(
                    "A2A Agent Card returned HTTP {}",
                    response.status().as_u16()
                ));
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| "Invalid Agent Card body".to_owned())?
            {
                if bytes.len() + chunk.len() > 256 * 1024 {
                    return Err("A2A Agent Card exceeds 256 KiB".into());
                }
                bytes.extend_from_slice(&chunk);
            }
            let mut card: p::AgentCard = serde_json::from_slice(&bytes)
                .map_err(|_| "Invalid A2A v1 Agent Card".to_owned())?;
            no_secret(&card, token)?;
            card.supported_interfaces.retain(|i| {
                matches!(
                    i.protocol_binding.as_str(),
                    p::TRANSPORT_PROTOCOL_JSONRPC | p::TRANSPORT_PROTOCOL_HTTP_JSON
                ) && i
                    .protocol_version
                    .split('.')
                    .next()
                    .and_then(|v| v.parse::<u64>().ok())
                    == Some(1)
                    && url(&i.url).is_ok_and(|u| u.origin() == address.origin())
            });
            let (_, interface) = A2AClientFactory::builder()
                .no_defaults()
                .register(Arc::new(a2a_client::jsonrpc::JsonRpcTransportFactory::new(
                    Some(http.clone()),
                )))
                .register(Arc::new(a2a_client::rest::RestTransportFactory::new(Some(
                    http,
                ))))
                .build()
                .create_from_card_with_interface(&card)
                .await
                .map_err(protocol_error)?;
            Ok::<_, String>((card, interface))
        })?;
        let credential = if let Some(token) = bearer {
            let dir = self.root().join(".khor/a2a/http/credentials");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let id = crate::link::fresh_hex()?;
            let mut file = private(&dir.join(&id))?;
            file.write_all(token.as_bytes())
                .map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            #[cfg(unix)]
            File::open(&dir)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
            Some(id)
        } else {
            previous.and_then(|p| p.credential)
        };
        let endpoint = Endpoint {
            name,
            card_url: address.to_string(),
            card,
            interface,
            credential,
        };
        files.write("service.json", &endpoint.card_url)?;
        files.write("endpoint.json", &endpoint)?;
        Ok(endpoint)
    }
    pub fn disconnect_a2a(&self, name: &str) -> Result<(), String> {
        self.refuse_unless_hosting()?;
        let files = Files::new(self.root(), name)?;
        let gate = private(&files.directory.join("connect.lock"))?;
        gate.lock().map_err(|e| e.to_string())?;
        match fs::remove_file(files.directory.join("endpoint.json")) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
    pub fn a2a_agents(&self) -> Result<Vec<Endpoint>, String> {
        let directory = self.root().join(".khor/a2a/http");
        if !directory.exists() {
            return Ok(vec![]);
        }
        let mut endpoints = Vec::new();
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let dir = entry.map_err(|e| e.to_string())?.path();
            if !dir.is_dir() || dir.file_name().is_none_or(|name| name == "credentials") {
                continue;
            }
            let files = Files { directory: dir };
            if let Some(endpoint) = files.read::<Endpoint>("endpoint.json")? {
                endpoints.push(endpoint)
            }
        }
        endpoints.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(endpoints)
    }
}
pub(crate) fn describe(endpoint: Endpoint) -> Agent {
    Agent {
        name: endpoint.name,
        available: true,
        why: None,
        protocol: Some(endpoint.interface.protocol_binding),
        card: Some(endpoint.card),
    }
}

fn save_task(
    files: &Files,
    caller: DeviceId,
    home: DeviceId,
    agent: &str,
    endpoint: &Endpoint,
    task: p::Task,
    parent: Option<TaskRef>,
) -> Result<Record, String> {
    opaque(&task.id)?;
    opaque(&task.context_id)?;
    if task.status.state == p::TaskState::Unspecified {
        return Err("A2A agent returned an unspecified task state".into());
    }
    if serde_json::to_vec(&task).map_err(|e| e.to_string())?.len() > MAX_TASK {
        return Err("A2A task exceeds the 1 MiB receipt limit".into());
    }
    files.transaction(|l| {
        let key = task_key(caller, &task.id);
        let previous = l.tasks.get(&key);
        if let Some(previous) = previous {
            if previous.endpoint.card_url != endpoint.card_url
                || previous.record.task.context_id != task.context_id
            {
                return Err("A2A task identity conflict".into());
            }
            if previous.record.task.status.state.is_terminal()
                && previous.record.task.status.state != task.status.state
            {
                return Err("A2A agent changed a terminal task state".into());
            }
        }
        let record = Record {
            task: task.clone(),
            target: AgentTarget {
                home,
                agent: agent.into(),
            },
            caller,
            parent: previous.map(|p| p.record.parent.clone()).unwrap_or(parent),
            context_session: None,
            revision: previous
                .map(|p| p.record.revision + u64::from(p.record.task != task))
                .unwrap_or(1),
            cancellation_requested: previous.is_some_and(|p| p.record.cancellation_requested),
        };
        l.tasks.insert(
            key,
            Receipt {
                record: record.clone(),
                endpoint: endpoint.clone(),
            },
        );
        Ok(record)
    })
}
fn response(
    files: &Files,
    node: &Node,
    caller: DeviceId,
    agent: &str,
    input: &Input,
    value: p::SendMessageResponse,
) -> Result<Answer, String> {
    no_secret(&value, credential(node.root(), &input.endpoint)?.as_deref())?;
    let answer = match &value {
        p::SendMessageResponse::Task(task) => {
            if input
                .message
                .task_id
                .as_ref()
                .is_some_and(|id| id != &task.id)
                || input
                    .message
                    .context_id
                    .as_ref()
                    .is_some_and(|id| id != &task.context_id)
            {
                return Err("A2A agent returned a different task or context".into());
            }
            Answer::Task {
                record: Box::new(save_task(
                    files,
                    caller,
                    node.device(),
                    agent,
                    &input.endpoint,
                    task.clone(),
                    input.parent.clone(),
                )?),
            }
        }
        p::SendMessageResponse::Message(message) => {
            opaque(&message.message_id)?;
            if let Some(context) = &message.context_id {
                opaque(context)?;
            }
            if let Some(task) = &message.task_id {
                opaque(task)?;
            }
            if message.role != p::Role::Agent
                || message.parts.is_empty()
                || serde_json::to_vec(message)
                    .map_err(|e| e.to_string())?
                    .len()
                    > MAX_TASK
            {
                return Err("Invalid or oversized A2A response message".into());
            }
            if message
                .context_id
                .as_ref()
                .zip(input.message.context_id.as_ref())
                .is_some_and(|(a, b)| a != b)
                || message
                    .task_id
                    .as_ref()
                    .zip(input.message.task_id.as_ref())
                    .is_some_and(|(a, b)| a != b)
            {
                return Err("A2A response message changed task or context".into());
            }
            Answer::Message {
                agent: agent.into(),
                message: message.clone(),
            }
        }
    };
    let key = task_key(caller, &input.request);
    files.transaction(|l| {
        l.inputs
            .get_mut(&key)
            .ok_or("A2A input receipt disappeared")?
            .response = Some(value);
        Ok(())
    })?;
    Ok(answer)
}
async fn recover_input(root: &Path, input: &Input) -> Result<p::SendMessageResponse, String> {
    let client = client(root, &input.endpoint).await?;
    let mut page = None;
    for _ in 0..4 {
        let result = client
            .list_tasks(&p::ListTasksRequest {
                context_id: input.message.context_id.clone(),
                status: None,
                page_size: Some(50),
                page_token: page,
                history_length: Some(100),
                status_timestamp_after: None,
                include_artifacts: Some(true),
                tenant: None,
            })
            .await
            .map_err(protocol_error)?;
        for task in result.tasks {
            if task
                .history
                .as_ref()
                .is_some_and(|h| h.iter().any(|m| m.message_id == input.message.message_id))
            {
                return Ok(p::SendMessageResponse::Task(task));
            }
        }
        if result.next_page_token.is_empty() {
            break;
        }
        page = Some(result.next_page_token);
    }
    Err("A2A send outcome is unknown. The message was not replayed; inspect the remote agent before sending new work".into())
}

fn endpoint_digest(endpoint: &Endpoint) -> String {
    // URL parsing rejects embedded credentials; the digest never discloses URLs.
    hash(&(&endpoint.card_url, &endpoint.interface))
}
fn check_binding(endpoint: &Endpoint, expected: Option<&str>) -> Result<(), String> {
    if expected.is_some_and(|expected| endpoint_digest(endpoint) != expected) {
        return Err("Delegation endpoint binding changed".into());
    }
    Ok(())
}
pub(crate) fn resolve_binding(
    node: &Node,
    caller: DeviceId,
    agent: &str,
    task: Option<&str>,
) -> Result<crate::delegation::BindingProtocol, String> {
    let files = Files::new(node.root(), agent)?;
    let endpoint = match task {
        Some(task) => files.receipt(caller, task)?.endpoint,
        None => files.endpoint()?,
    };
    Ok(crate::delegation::BindingProtocol::Http {
        endpoint_digest: endpoint_digest(&endpoint),
    })
}
pub(crate) fn handle_bound(
    node: &Node,
    caller: DeviceId,
    agent: &str,
    action: crate::delegation::BoundAction,
    digest: &str,
) -> Result<Answer, String> {
    runtime()?.block_on(async {
        tokio::time::timeout(
            Duration::from_secs(12),
            handle_async(
                node,
                caller,
                action.into_action(agent.into())?,
                Some(digest),
            ),
        )
        .await
        .map_err(|_| "A2A bound operation outcome is unknown".to_owned())?
    })
}
pub(crate) fn handle(node: &Node, caller: DeviceId, action: Action) -> Result<Answer, String> {
    runtime()?.block_on(async {
        tokio::time::timeout(Duration::from_secs(12),handle_async(node,caller,action,None)).await
            .map_err(|_|"A2A operation timed out; a send may have been accepted. Read its receipt before retrying".to_owned())?
    })
}
async fn handle_async(
    node: &Node,
    caller: DeviceId,
    action: Action,
    expected: Option<&str>,
) -> Result<Answer, String> {
    let agent = match &action {
        Action::Send { agent, .. }
        | Action::Get { agent, .. }
        | Action::Wait { agent, .. }
        | Action::List { agent, .. }
        | Action::Cancel { agent, .. }
        | Action::Answer { agent, .. } => agent.clone(),
        _ => return Err("Not an A2A task operation".into()),
    };
    let files = Files::new(node.root(), &agent)?;
    match action {
        Action::Send { mut message, parent, .. } => {
            if parent.as_ref().is_some_and(|p| p.home != caller) {
                return Err("A2A parent must belong to the calling device".into());
            }
            opaque(&message.message_id)?;
            if message.role != p::Role::User || message.parts.is_empty()
                || serde_json::to_vec(&message).map_err(|e| e.to_string())?.len() > 128 * 1024
            {
                return Err("Invalid or oversized A2A input message".into());
            }
            if message.metadata.as_ref().is_some_and(|m| m.contains_key("khor.cwd")) {
                return Err("A2A agents manage their own workspace; omit cwd".into());
            }
            let key = task_key(caller, &message.message_id);
            let fingerprint = hash(&(&message, &parent));
            let request = message.message_id.clone();
            message.message_id = wire_id(node.device(), caller, &agent, &request);
            let gate = private(&files.directory.join(format!("input-{key}.lock")))?;
            gate.try_lock().map_err(|_| "This A2A input is already being sent; query or retry the same request later".to_owned())?;
            let prior = files.transaction(|l| Ok(l.inputs.get(&key).cloned()))?;
            if let Some(input) = prior {
                check_binding(&input.endpoint, expected)?;
                if input.fingerprint != fingerprint {
                    return Err("A2A message ID already belongs to different input".into());
                }
                if let Some(value) = input.response.clone() {
                    return match value {
                        p::SendMessageResponse::Task(task) => Ok(Answer::Task {
                            record: Box::new(files.receipt(caller, &task.id)?.record),
                        }),
                        p::SendMessageResponse::Message(message) => Ok(Answer::Message { agent, message }),
                    };
                }
                // No replay after a timeout, process loss or malformed response.
                let recovered = recover_input(node.root(), &input).await?;
                return response(&files, node, caller, &agent, &input, recovered);
            }
            let _task_gate = message.task_id.as_ref()
                .map(|id| task_gate(&files, caller, id)).transpose()?;
            let endpoint = if let Some(id) = &message.task_id {
                opaque(id)?;
                let previous = files.receipt(caller, id)?;
                if !matches!(previous.record.task.status.state, p::TaskState::InputRequired | p::TaskState::AuthRequired) {
                    return Err("This A2A task is not awaiting input; use its context for a new task".into());
                }
                if message.context_id.as_ref().is_some_and(|c| c != &previous.record.task.context_id) {
                    return Err("A2A task/context mismatch".into());
                }
                previous.endpoint
            } else {
                files.endpoint()?
            };
            check_binding(&endpoint, expected)?;
            if let Some(context) = &message.context_id {
                opaque(context)?;
                let known = files.transaction(|ledger| {
                    Ok(ledger.owns_context(caller, &agent, &endpoint, context))
                })?;
                if !known {
                    return Err("A2A context does not belong to this caller and agent".into());
                }
            }
            // Validate the route before intent commits. An interruption after
            // commit is uncertain and must be reconciled rather than resent.
            let client = client(node.root(), &endpoint).await?;
            let input = Input {
                fingerprint, request, caller, message: message.clone(), parent,
                endpoint, response: None,
            };
            files.transaction(|l| { l.inputs.insert(key, input.clone()); Ok(()) })?;
            let value = client.send_message(&p::SendMessageRequest {
                message,
                configuration: Some(p::SendMessageConfiguration {
                    accepted_output_modes: None, task_push_notification_config: None,
                    history_length: Some(100), return_immediately: Some(true),
                }),
                metadata: None, tenant: None,
            }).await.map_err(protocol_error)?;
            response(&files, node, caller, &agent, &input, value)
        }
        Action::Get { task, .. } => Ok(Answer::Task {
            record: Box::new(refresh_bound(node, &files, caller, &agent, &task, expected).await?),
        }),
        Action::Wait { task, revision, .. } => {
            let until = Instant::now() + Duration::from_secs(7);
            loop {
                let record = refresh(node, &files, caller, &agent, &task).await?;
                if record.revision != revision || record.task.status.state.is_terminal()
                    || matches!(record.task.status.state, p::TaskState::InputRequired | p::TaskState::AuthRequired)
                    || Instant::now() >= until
                {
                    return Ok(Answer::Task { record: Box::new(record) });
                }
                tokio::time::sleep(Duration::from_millis(400)).await;
            }
        }
        Action::Cancel { task, .. } => {
            let _gate = task_gate(&files, caller, &task)?;
            let receipt = files.receipt(caller, &task)?;
            check_binding(&receipt.endpoint, expected)?;
            if receipt.record.task.status.state.is_terminal() {
                return Ok(Answer::Task { record: Box::new(receipt.record) });
            }
            files.transaction(|l| {
                let record = &mut l.tasks.get_mut(&task_key(caller, &task))
                    .ok_or("A2A receipt disappeared")?.record;
                if !record.cancellation_requested {
                    record.cancellation_requested = true;
                    record.revision += 1;
                }
                Ok(())
            })?;
            let value = client(node.root(), &receipt.endpoint).await?
                .cancel_task(&p::CancelTaskRequest { id: task.clone(), metadata: None, tenant: None })
                .await.map_err(protocol_error)?;
            if value.id != task { return Err("A2A agent returned a different task".into()); }
            no_secret(&value, credential(node.root(), &receipt.endpoint)?.as_deref())?;
            Ok(Answer::Task { record: Box::new(save_task(&files, caller, node.device(),
                &agent, &receipt.endpoint, value, receipt.record.parent)?) })
        }
        Action::List { context, after, .. } => {
            // Recover a few uncertain inputs so parent Stop can locate the
            // remote task by message ID. This never submits work again.
            let pending = files.transaction(|l| Ok(l.inputs.values().filter(|i|
                i.caller == caller && i.response.is_none()
                && context.as_ref().is_none_or(|c| i.message.context_id.as_ref() == Some(c)))
                .take(4).cloned().collect::<Vec<_>>()))?;
            for input in pending {
                let key = task_key(caller, &input.request);
                let gate = private(&files.directory.join(format!("input-{key}.lock")))?;
                if gate.try_lock().is_err() { continue; }
                if let Ok(value) = recover_input(node.root(), &input).await {
                    response(&files, node, caller, &agent, &input, value)?;
                }
            }
            let mut records = files.transaction(|l| Ok(l.tasks.values().filter(|r|
                r.record.caller == caller
                && context.as_ref().is_none_or(|c| c == &r.record.task.context_id)
                && after.as_ref().is_none_or(|a| r.record.task.id > *a))
                .map(|r| r.record.clone()).collect::<Vec<_>>()))?;
            records.sort_by(|a, b| a.task.id.cmp(&b.task.id));
            let mut page = Vec::new();
            let mut bytes = 0;
            let mut more = false;
            for record in records {
                let size = serde_json::to_vec(&record).map_err(|e| e.to_string())?.len();
                if page.len() == 50 || bytes + size > 2 * 1024 * 1024 { more = true; break; }
                bytes += size;
                page.push(record);
            }
            let next = if more { page.last().map(|r| r.task.id.clone()) } else { None };
            Ok(Answer::Tasks { records: page, next, cached: true })
        }
        Action::Answer { .. } => Err("A2A input is not an ACP permission. Use delegate with task, context and the user's reply; AuthRequired may need local credential setup".into()),
        _ => Err("Unsupported A2A task operation".into()),
    }
}

async fn refresh(
    node: &Node,
    files: &Files,
    caller: DeviceId,
    agent: &str,
    id: &str,
) -> Result<Record, String> {
    refresh_bound(node, files, caller, agent, id, None).await
}
async fn refresh_bound(
    node: &Node,
    files: &Files,
    caller: DeviceId,
    agent: &str,
    id: &str,
    expected: Option<&str>,
) -> Result<Record, String> {
    let _gate = task_gate(files, caller, id)?;
    let receipt = files.receipt(caller, id)?;
    check_binding(&receipt.endpoint, expected)?;
    let task = client(node.root(), &receipt.endpoint)
        .await?
        .get_task(&p::GetTaskRequest {
            id: id.into(),
            history_length: Some(100),
            tenant: None,
        })
        .await
        .map_err(protocol_error)?;
    if task.id != id {
        return Err("A2A agent returned a different task".into());
    }
    no_secret(
        &task,
        credential(node.root(), &receipt.endpoint)?.as_deref(),
    )?;
    save_task(
        files,
        caller,
        node.device(),
        agent,
        &receipt.endpoint,
        task,
        receipt.record.parent,
    )
}
fn task_gate(files: &Files, caller: DeviceId, id: &str) -> Result<File, String> {
    opaque(id)?;
    let gate = private(
        &files
            .directory
            .join(format!("task-{}.lock", task_key(caller, id))),
    )?;
    gate.try_lock()
        .map_err(|_| "This A2A task is being updated; query again shortly".to_owned())?;
    Ok(gate)
}
