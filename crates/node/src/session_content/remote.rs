//! Independent read-only transport over the existing Khor connection.
use super::{ContentLocator, ReadBinding, Unavailable};
use crate::adaptor::transcript_page::{Cursor, PreparedRead, RecordChunk};
use khor_core::DeviceId;
use serde::{Deserialize, Serialize};

pub(crate) type ResponsePermit = std::sync::Arc<tokio::sync::OwnedSemaphorePermit>;

pub const METADATA_WIRE_BYTES: usize = 32 * 1024;
pub const READ_WIRE_BYTES: usize = crate::adaptor::transcript_page::WINDOW_WIRE_BYTES;
const CURSOR_BYTES: usize = 4096;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Action {
    Resolve {
        locator: ContentLocator,
    },
    Begin {
        binding: ReadBinding,
        request: String,
    },
    Window {
        binding: ReadBinding,
        request: String,
        cursor: Option<Cursor>,
        newer: bool,
    },
    Record {
        binding: ReadBinding,
        request: String,
        cursor: Cursor,
        from: u64,
    },
    Cancel {
        home: DeviceId,
        request: String,
    },
}
impl Action {
    pub fn home(&self) -> DeviceId {
        match self {
            Self::Resolve { locator } => locator.home(),
            Self::Begin { binding, .. }
            | Self::Window { binding, .. }
            | Self::Record { binding, .. } => binding.home(),
            Self::Cancel { home, .. } => *home,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Resolve { locator } => {
                locator.encode()?;
            }
            Self::Begin { binding, request } => {
                binding.validate()?;
                super::requests::validate_token(request)?;
            }
            Self::Window {
                binding,
                request,
                cursor,
                newer,
            } => {
                binding.validate()?;
                super::requests::validate_token(request)?;
                if *newer && cursor.is_none() {
                    return Err("newer history requires a cursor".into());
                }
                if let Some(cursor) = cursor {
                    validate_cursor(binding, cursor)?;
                }
            }
            Self::Record {
                binding,
                request,
                cursor,
                from,
            } => {
                binding.validate()?;
                super::requests::validate_token(request)?;
                validate_cursor(binding, cursor)?;
                let range = cursor.record_range();
                if *from < range.start || *from > range.end {
                    return Err("invalid history record range".into());
                }
            }
            Self::Cancel { request, .. } => super::requests::validate_token(request)?,
        }
        if crate::proto::encode(self)?.len() > METADATA_WIRE_BYTES {
            return Err("content read request exceeds its metadata wire budget".into());
        }
        Ok(())
    }
}
fn validate_cursor(binding: &ReadBinding, cursor: &Cursor) -> Result<(), String> {
    if !cursor.is_for_source(&binding.source_id()) {
        return Err("history cursor belongs to another content binding".into());
    }
    if serde_json::to_vec(cursor)
        .map_err(|error| error.to_string())?
        .len()
        > CURSOR_BYTES
    {
        return Err("history cursor exceeds its byte budget".into());
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capture {
    pub opening: String,
    pub gaps: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Resolution {
    Available {
        binding: ReadBinding,
        following: bool,
        capture: Option<Capture>,
    },
    Unavailable(Unavailable),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Reply {
    Resolved(Resolution),
    Accepted,
    Window { source: String, read: PreparedRead },
    Record { source: String, chunk: RecordChunk },
}
impl Reply {
    pub(crate) fn wire_limit(&self) -> usize {
        match self {
            Self::Window { .. } | Self::Record { .. } => READ_WIRE_BYTES,
            _ => METADATA_WIRE_BYTES,
        }
    }
    pub(crate) fn validate_for(&self, action: &Action) -> Result<(), String> {
        self.validate()?;
        match (self, action) {
            (Self::Accepted, Action::Begin { .. } | Action::Cancel { .. }) => Ok(()),
            (Self::Resolved(Resolution::Unavailable(_)), Action::Resolve { .. }) => Ok(()),
            (
                Self::Resolved(Resolution::Available { binding, .. }),
                Action::Resolve { locator },
            ) => {
                if binding.home() != locator.home()
                    || binding.view() != locator.view()
                    || locator
                        .reference()
                        .is_some_and(|reference| reference != binding.reference())
                {
                    return Err("content resolution belongs to another locator".into());
                }
                Ok(())
            }
            (
                Self::Window { source, read },
                Action::Window {
                    binding,
                    cursor,
                    newer,
                    ..
                },
            ) => {
                if source != &binding.source_id() {
                    return Err("history window belongs to another content binding".into());
                }
                read.validate_request(cursor.as_ref(), *newer)
            }
            (
                Self::Record { source, chunk },
                Action::Record {
                    binding,
                    cursor,
                    from,
                    ..
                },
            ) => {
                let end = chunk
                    .offset
                    .checked_add(chunk.bytes.len() as u64)
                    .ok_or("invalid history record chunk range")?;
                let range = cursor.record_range();
                let expected = range
                    .end
                    .checked_sub(*from)
                    .ok_or("invalid history record range")?
                    .min(crate::adaptor::transcript_window::INLINE_RECORD_BYTES as u64);
                if source != &binding.source_id()
                    || chunk.offset != *from
                    || chunk.bytes.len() as u64 != expected
                    || end > range.end
                    || chunk.next != (end < range.end).then_some(end)
                {
                    return Err("history record chunk does not match its request".into());
                }
                Ok(())
            }
            _ => Err("content read reply does not match its request".into()),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        let limit = match self {
            Self::Resolved(Resolution::Available {
                binding, capture, ..
            }) => {
                binding.validate()?;
                if capture
                    .as_ref()
                    .is_some_and(|capture| capture.opening.len() > 64)
                {
                    return Err("content capture metadata exceeds its byte budget".into());
                }
                METADATA_WIRE_BYTES
            }
            Self::Resolved(Resolution::Unavailable(_)) | Self::Accepted => METADATA_WIRE_BYTES,
            Self::Window { source, read } => {
                source_id(source)?;
                read.validate()?;
                READ_WIRE_BYTES
            }
            Self::Record { source, chunk } => {
                source_id(source)?;
                let end = chunk
                    .offset
                    .checked_add(chunk.bytes.len() as u64)
                    .ok_or("invalid history record chunk range")?;
                if chunk.bytes.len() > crate::adaptor::transcript_window::INLINE_RECORD_BYTES
                    || chunk
                        .next
                        .is_some_and(|next| chunk.bytes.is_empty() || next != end)
                {
                    return Err("invalid or over-budget history record chunk".into());
                }
                READ_WIRE_BYTES
            }
        };
        if crate::proto::encode(self)?.len() > limit {
            return Err("content read response exceeds its serialized wire budget".into());
        }
        Ok(())
    }
}
fn source_id(source: &str) -> Result<(), String> {
    if source.is_empty() || source.len() > 128 {
        return Err("invalid content source identity".into());
    }
    Ok(())
}

pub(crate) fn admit_here(
    home: DeviceId,
    reader: &super::requests::Reader,
    action: &Action,
) -> Result<Option<super::requests::Request>, String> {
    action.validate()?;
    if action.home() != home {
        return Err("content request belongs to another home".into());
    }
    match action {
        Action::Window {
            binding, request, ..
        }
        | Action::Record {
            binding, request, ..
        } => {
            let scope = super::requests::Scope::new(home, reader.clone());
            super::requests::claim(&scope, request, &binding.source_id()).map(Some)
        }
        _ => Ok(None),
    }
}

pub(crate) fn execute_here(
    root: &std::path::Path,
    home: DeviceId,
    reader: super::requests::Reader,
    action: Action,
    active: Option<super::requests::Request>,
) -> Result<Reply, String> {
    use super::requests::{self, Scope};
    action.validate()?;
    if action.home() != home {
        return Err("content request belongs to another home".into());
    }
    let scope = Scope::new(home, reader);
    let reply = match action {
        Action::Resolve { locator } => {
            let node = crate::Node::open(root.to_path_buf())?;
            if node.device() != home {
                return Err("content home identity changed".into());
            }
            Reply::Resolved(resolve_here(&node, &locator)?)
        }
        Action::Begin { binding, request } => {
            requests::begin(&scope, &request, &binding.source_id())?;
            Reply::Accepted
        }
        Action::Cancel { request, .. } => {
            requests::cancel(&scope, &request)?;
            Reply::Accepted
        }
        Action::Window {
            binding,
            cursor,
            newer,
            ..
        } => {
            let source_id = binding.source_id();
            let request = active
                .as_ref()
                .ok_or("content IO request was not admitted")?;
            request.check()?;
            #[cfg(test)]
            super::wire_tests::pause_io(root);
            request.check()?;
            let source = open_source(root, home, &binding)?;
            request.check()?;
            let (mut file, _) = source.open().map_err(|error| error.to_string())?;
            let read = crate::adaptor::transcript_page::prepare_read(
                &mut file,
                &source_id,
                cursor.as_ref(),
                newer,
                &|| request.flag().load(std::sync::atomic::Ordering::Acquire),
            )?;
            request.check()?;
            Reply::Window {
                source: source_id,
                read,
            }
        }
        Action::Record {
            binding,
            cursor,
            from,
            ..
        } => {
            let source_id = binding.source_id();
            let request = active
                .as_ref()
                .ok_or("content IO request was not admitted")?;
            request.check()?;
            #[cfg(test)]
            super::wire_tests::pause_io(root);
            request.check()?;
            let source = open_source(root, home, &binding)?;
            request.check()?;
            let (mut file, _) = source.open().map_err(|error| error.to_string())?;
            let chunk = crate::adaptor::transcript_page::read_record_chunk(
                &mut file,
                &source_id,
                &cursor,
                from,
                &|| request.flag().load(std::sync::atomic::Ordering::Acquire),
            )?;
            request.check()?;
            Reply::Record {
                source: source_id,
                chunk,
            }
        }
    };
    reply.validate()?;
    if let Some(request) = &active {
        request.check()?;
    }
    Ok(reply)
}

fn open_source(
    root: &std::path::Path,
    home: DeviceId,
    binding: &ReadBinding,
) -> Result<super::ContentSource, String> {
    binding
        .resolve_exact(root, home)
        .map_err(|reason| format!("content source unavailable: {reason:?}"))?
        .ok_or_else(|| "content source is no longer available".into())
}

fn resolve_here(node: &crate::Node, locator: &ContentLocator) -> Result<Resolution, String> {
    use super::{Availability, ContentReference};
    let selected = match locator.reference() {
        Some(reference) => reference.resolve(node.root(), node.device()),
        None => node.selected_content(locator.view(), true),
    };
    let source = match selected {
        Ok(Some(source)) => source,
        Ok(None) => return Ok(Resolution::Unavailable(Unavailable::NoSource)),
        Err(reason) => return Ok(Resolution::Unavailable(reason)),
    };
    match source.availability() {
        Availability::Unavailable(reason) => Ok(Resolution::Unavailable(reason)),
        Availability::Available { following, capture } => {
            let (namespace, agent, native_id) = source.observation_identity();
            let binding = ReadBinding::new(
                locator.view().clone(),
                ContentReference {
                    home: node.device(),
                    namespace,
                    agent: agent.into(),
                    native_id: native_id.into(),
                },
            )?
            .with_journal_generation(source.journal_generation())?;
            Ok(Resolution::Available {
                binding,
                following,
                capture: capture.map(|capture| Capture {
                    opening: capture.opening.into(),
                    gaps: capture.gaps,
                }),
            })
        }
    }
}

/// The GUI keeps this guard through parsing and its final bounded projection.
/// An IO/parse worker takes another Arc, so dropping an await cannot free its slot.
pub struct ClientRead {
    root: std::path::PathBuf,
    binding: ReadBinding,
    token: String,
    request: super::requests::Request,
}
impl ClientRead {
    pub fn claim(
        root: &std::path::Path,
        binding: ReadBinding,
        token: &str,
    ) -> Result<std::sync::Arc<Self>, String> {
        use super::requests::{self, Reader, Scope};
        binding.validate()?;
        let scope = Scope::new(binding.home(), Reader::Client(root.to_path_buf()));
        let request = requests::claim(&scope, token, &binding.source_id())?;
        request.check()?;
        Ok(std::sync::Arc::new(Self {
            root: root.to_path_buf(),
            binding,
            token: token.into(),
            request,
        }))
    }
    pub fn check(&self) -> Result<(), String> {
        self.request.check()
    }
    fn check_node(&self, node: &crate::Node) -> Result<(), String> {
        if self.root.as_path() != node.root().as_path() {
            return Err("content read belongs to another local client domain".into());
        }
        self.check()
    }
}

async fn begin_with<F, Fut>(
    root: &std::path::Path,
    binding: &ReadBinding,
    send: F,
) -> Result<String, String>
where
    F: Fn(Action) -> Fut,
    Fut: std::future::Future<Output = Result<Reply, String>>,
{
    use super::requests::{self, Reader, Scope};
    binding.validate()?;
    let token = crate::link::fresh_hex()?;
    let scope = Scope::new(binding.home(), Reader::Client(root.to_path_buf()));
    requests::begin(&scope, &token, &binding.source_id())?;
    let action = Action::Begin {
        binding: binding.clone(),
        request: token.clone(),
    };
    let result = send(action.clone())
        .await
        .and_then(|reply| reply.validate_for(&action));
    if let Err(error) = result {
        requests::cancel(&scope, &token)?;
        // A lost Begin ACK may have left a home-side pending lease, never IO.
        let cleanup = send(Action::Cancel {
            home: binding.home(),
            request: token,
        })
        .await;
        return Err(match cleanup {
            Ok(_) => error,
            Err(cleanup) => format!(
                "{error}; content cancellation failed: {}",
                cleanup.chars().take(512).collect::<String>()
            ),
        });
    }
    Ok(token)
}

async fn cancel_with<F, Fut>(
    root: &std::path::Path,
    binding: &ReadBinding,
    token: &str,
    send: F,
) -> Result<(), String>
where
    F: Fn(Action) -> Fut,
    Fut: std::future::Future<Output = Result<Reply, String>>,
{
    use super::requests::{self, Reader, Scope};
    binding.validate()?;
    requests::validate_token(token)?;
    requests::cancel(
        &Scope::new(binding.home(), Reader::Client(root.to_path_buf())),
        token,
    )?;
    requests::cancel(&Scope::new(binding.home(), Reader::local(root)), token)?;
    let action = Action::Cancel {
        home: binding.home(),
        request: token.into(),
    };
    send(action.clone()).await?.validate_for(&action)
}

/// Registration and cancellation use only the existing authenticated resident
/// handoff. In particular cancellation never reopens Node or scans providers.
pub async fn begin(root: &std::path::Path, binding: &ReadBinding) -> Result<String, String> {
    begin_with(root, binding, |action| {
        crate::link::content_on_resident(root, action)
    })
    .await
}
pub async fn cancel(
    root: &std::path::Path,
    binding: &ReadBinding,
    token: &str,
) -> Result<(), String> {
    cancel_with(root, binding, token, |action| {
        crate::link::content_on_resident(root, action)
    })
    .await
}

impl crate::Node {
    pub async fn content_begin(&self, binding: &ReadBinding) -> Result<String, String> {
        begin_with(self.root(), binding, |action| self.content_read(action)).await
    }
    pub async fn content_cancel(&self, binding: &ReadBinding, token: &str) -> Result<(), String> {
        cancel_with(self.root(), binding, token, |action| {
            self.content_read(action)
        })
        .await
    }
    pub async fn content_page(
        &self,
        request: &std::sync::Arc<ClientRead>,
        cursor: Option<Cursor>,
        newer: bool,
    ) -> Result<crate::adaptor::transcript_page::Page<super::ContentEvent>, String> {
        request.check_node(self)?;
        let action = Action::Window {
            binding: request.binding.clone(),
            request: request.token.clone(),
            cursor: cursor.clone(),
            newer,
        };
        let reply = self.content_read(action.clone()).await?;
        request.check()?;
        reply.validate_for(&action)?;
        let Reply::Window { source, read } = reply else {
            return Err("content reader expected a history window".into());
        };
        let parse = request.binding.reference().parser()?;
        let request = request.clone();
        tokio::task::spawn_blocking(move || {
            request.check()?;
            let page = crate::adaptor::transcript_page::parse_prepared(
                read,
                &source,
                cursor.as_ref(),
                newer,
                parse,
                &|| {
                    request
                        .request
                        .flag()
                        .load(std::sync::atomic::Ordering::Acquire)
                },
            )?;
            request.check()?;
            // Keep output and owner together if the waiting caller is dropped.
            Ok((page, request))
        })
        .await
        .map_err(|error| format!("content parse worker failed: {error}"))?
        .map(|(page, _request)| page)
    }
    pub async fn content_record(
        &self,
        request: &std::sync::Arc<ClientRead>,
        cursor: Cursor,
        from: u64,
    ) -> Result<RecordChunk, String> {
        request.check_node(self)?;
        let action = Action::Record {
            binding: request.binding.clone(),
            request: request.token.clone(),
            cursor,
            from,
        };
        let reply = self.content_read(action.clone()).await?;
        request.check()?;
        reply.validate_for(&action)?;
        match reply {
            Reply::Record { chunk, .. } => Ok(chunk),
            _ => Err("content reader expected a history record".into()),
        }
    }
}
