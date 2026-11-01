//! Ordered socket commands enter their attachment queue before invoke returns.
//! A normal async command would enqueue only when Tauri polls its task, which
//! can reverse two arrivals. Only completion is asynchronous here.
//!
//! This handler runs inside Tauri's usual invoke-key / origin / webview ACL
//! checks. Argument decoding uses the very same CommandItem as #[command],
//! including its ACL and InvalidArgs errors. Unrecognized commands go back to
//! generate_handler unchanged.

use khor_gui_core::{
    chat::{self, Control},
    command_queue::Pending,
    term,
};
use serde::{Serialize, de::DeserializeOwned};
use tauri::{
    Runtime,
    ipc::{CommandArg, CommandItem, Invoke, InvokeError},
};

pub(crate) const COMMANDS: &[&str] = &[
    "chat_answer",
    "chat_respond",
    "chat_control",
    "chat_set_mode",
    "chat_set_option",
    "chat_stop",
    "chat_capture",
    "chat_say",
    "term_key",
    "term_resize",
    "term_ready",
    "term_forget",
    "term_drop",
    "term_drop_cancel",
];

enum Request {
    Chat {
        id: String,
        lease: String,
        command: Control,
    },
    Say {
        id: String,
        lease: String,
        text: String,
        images: Vec<(String, String)>,
        files: Vec<(String, String)>,
        token: Option<String>,
    },
    Key {
        id: String,
        lease: String,
        keys: String,
    },
    Resize {
        id: String,
        lease: String,
        cols: u16,
        rows: u16,
    },
    Ready {
        id: String,
        lease: String,
    },
    Forget {
        id: String,
        lease: String,
        redraw: bool,
    },
    Drop {
        id: String,
        lease: String,
        paths: Vec<String>,
        from: Option<String>,
        into: Option<String>,
    },
    CancelDrop {
        id: String,
        lease: String,
        staged: Vec<String>,
    },
}

fn argument<R: Runtime, T: DeserializeOwned>(
    invoke: &Invoke<R>,
    name: &'static str,
    key: &'static str,
) -> Result<T, InvokeError> {
    T::from_command(CommandItem {
        plugin: None,
        name,
        key,
        message: &invoke.message,
        acl: &invoke.acl,
    })
}

fn parse<R: Runtime>(invoke: &Invoke<R>, name: &'static str) -> Result<Request, InvokeError> {
    let id = argument(invoke, name, "id")?;
    let lease = argument(invoke, name, "lease")?;
    macro_rules! arg {
        ($key:literal) => {
            argument(invoke, name, $key)?
        };
    }
    let command = match name {
        "chat_answer" => Control::Answer {
            ask: arg!("ask"),
            option: arg!("option"),
        },
        "chat_respond" => Control::Respond {
            ask: arg!("ask"),
            response: arg!("response"),
        },
        "chat_control" => Control::Operation {
            token: arg!("token"),
            operation: arg!("operation"),
            payload: arg!("payload"),
        },
        "chat_set_mode" => Control::SetMode(arg!("mode")),
        "chat_set_option" => Control::SetOption {
            option: arg!("option"),
            value: arg!("value"),
        },
        "chat_stop" => Control::Stop,
        "chat_capture" => {
            let token: String = arg!("token");
            Control::Capture(chat::capture_token(&token).map_err(InvokeError::from)?)
        },
        "chat_say" => {
            let text = arg!("text");
            let images = argument::<R, Option<Vec<(String, String)>>>(invoke, name, "images")?
                .unwrap_or_default();
            let files = argument::<R, Option<Vec<(String, String)>>>(invoke, name, "files")?
                .unwrap_or_default();
            let token: Option<String> = argument(invoke, name, "token")?;
            chat::validate_send_token(token.as_deref()).map_err(InvokeError::from)?;
            return Ok(Request::Say {
                id,
                lease,
                text,
                images,
                files,
                token,
            });
        }
        "term_key" => {
            return Ok(Request::Key {
                id,
                lease,
                keys: arg!("keys"),
            });
        }
        "term_resize" => {
            return Ok(Request::Resize {
                id,
                lease,
                cols: arg!("cols"),
                rows: arg!("rows"),
            });
        }
        "term_ready" => return Ok(Request::Ready { id, lease }),
        "term_forget" => {
            return Ok(Request::Forget {
                id,
                lease,
                redraw: arg!("redraw"),
            });
        }
        "term_drop" => {
            return Ok(Request::Drop {
                id,
                lease,
                paths: arg!("paths"),
                from: arg!("from"),
                into: arg!("into"),
            });
        }
        "term_drop_cancel" => {
            return Ok(Request::CancelDrop {
                id,
                lease,
                staged: argument::<R, Option<Vec<String>>>(invoke, name, "staged")?
                    .unwrap_or_default(),
            });
        }
        _ => unreachable!("dispatch checked the command name"),
    };
    Ok(Request::Chat { id, lease, command })
}

/// A handled invoke is consumed; unknown commands retain their resolver and
/// ACL for the generated handler. Submission precedes respond_async.
pub(crate) fn dispatch<R: Runtime>(invoke: Invoke<R>) -> Result<(), Invoke<R>> {
    dispatch_into(invoke, submit)
}

fn dispatch_into<R: Runtime>(
    invoke: Invoke<R>,
    accept: impl FnOnce(Invoke<R>, Request),
) -> Result<(), Invoke<R>> {
    let Some(name) = COMMANDS
        .iter()
        .copied()
        .find(|name| *name == invoke.message.command())
    else {
        return Err(invoke);
    };
    let request = match parse(&invoke, name) {
        Ok(request) => request,
        Err(why) => {
            invoke.resolver.invoke_error(why);
            return Ok(());
        }
    };
    accept(invoke, request);
    Ok(())
}

fn submit<R: Runtime>(invoke: Invoke<R>, request: Request) {
    let root = khor_node::Node::root_from_env();
    match request {
        Request::Chat { id, lease, command } => {
            reply(invoke, chat::submit_control(&root, &id, &lease, command))
        }
        Request::Say {
            id,
            lease,
            text,
            images,
            files,
            token,
        } => reply(
            invoke,
            chat::submit_say_with_token(&root, &id, &lease, text, images, files, token),
        ),
        Request::Key { id, lease, keys } => reply(
            invoke,
            term::submit_key(&root, &id, &lease, keys.into_bytes()),
        ),
        Request::Resize {
            id,
            lease,
            cols,
            rows,
        } => reply(invoke, term::submit_resize(&root, &id, &lease, cols, rows)),
        Request::Ready { id, lease } => reply(invoke, term::submit_ready(&root, &id, &lease)),
        Request::Forget { id, lease, redraw } => {
            reply(invoke, term::submit_forget(&root, &id, &lease, redraw))
        }
        Request::Drop {
            id,
            lease,
            paths,
            from,
            into,
        } => {
            let job = match term::begin_drop(&root, &id, &lease) {
                Ok(job) => job,
                Err(why) => {
                    invoke.resolver.reject(why);
                    return;
                }
            };
            invoke.resolver.respond_async(async move {
                crate::off_main(move || {
                    Ok(job.run_paths(
                        &khor_node::Node::root_from_env(),
                        &paths,
                        from.as_deref(),
                        into.as_deref(),
                    ))
                })
                .await
                .map_err(InvokeError::from)
            });
        }
        Request::CancelDrop { id, lease, staged } => {
            // Cancellation is immediate; filesystem cleanup must not make
            // the UI wait, nor cancel a later drop after scheduling a task.
            if let Err(why) = term::cancel_drop(&root, &id, &lease) {
                invoke.resolver.reject(why);
                return;
            }
            invoke.resolver.respond_async(async move {
                crate::off_main(move || {
                    term::term_drop_forget(&khor_node::Node::root_from_env(), &staged);
                    Ok(())
                })
                .await
                .map_err(InvokeError::from)
            });
        }
    }
}

fn reply<R: Runtime, T: Serialize + Send + 'static>(
    invoke: Invoke<R>,
    pending: Result<Pending<T>, String>,
) {
    match pending {
        Ok(pending) => invoke
            .resolver
            .respond_async(async move { pending.await.map_err(InvokeError::from) }),
        Err(why) => invoke.resolver.reject(why),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    // The original #[command] signatures are the parser oracle. Keep real
    // Tauri decoding here: a hand-written JSON mock would miss optional keys,
    // integer ranges, raw payload rejection, and InvalidArgs wording.
    #[allow(unused_variables)]
    mod original {
        #[tauri::command]
        pub fn chat_answer(id: String, lease: String, ask: u64, option: Option<String>) -> bool {
            true
        }
        #[tauri::command]
        pub fn chat_respond(
            id: String,
            lease: String,
            ask: u64,
            response: serde_json::Value,
        ) -> bool {
            true
        }
        #[tauri::command]
        pub fn chat_control(
            id: String,
            lease: String,
            token: String,
            operation: String,
            payload: serde_json::Value,
        ) -> bool {
            true
        }
        #[tauri::command]
        pub fn chat_set_mode(id: String, lease: String, mode: String) -> bool {
            true
        }
        #[tauri::command]
        pub fn chat_set_option(
            id: String,
            lease: String,
            option: String,
            value: serde_json::Value,
        ) -> bool {
            true
        }
        #[tauri::command]
        pub fn chat_stop(id: String, lease: String) -> bool {
            true
        }
        #[tauri::command]
        pub fn chat_capture(id: String, lease: String, token: String) -> bool { true }
        #[tauri::command]
        pub fn chat_say(
            id: String,
            lease: String,
            text: String,
            images: Option<Vec<(String, String)>>,
            files: Option<Vec<(String, String)>>,
            token: Option<String>,
        ) -> Result<bool, String> {
            khor_gui_core::chat::validate_send_token(token.as_deref())?;
            Ok(true)
        }
        #[tauri::command]
        pub fn term_key(id: String, lease: String, keys: String) -> bool {
            true
        }
        #[tauri::command]
        pub fn term_resize(id: String, lease: String, cols: u16, rows: u16) -> bool {
            true
        }
        #[tauri::command]
        pub fn term_ready(id: String, lease: String) -> bool {
            true
        }
        #[tauri::command]
        pub fn term_forget(id: String, lease: String, redraw: bool) -> bool {
            true
        }
        #[tauri::command]
        pub fn term_drop(
            id: String,
            lease: String,
            paths: Vec<String>,
            from: Option<String>,
            into: Option<String>,
        ) -> bool {
            true
        }
        #[tauri::command]
        pub fn term_drop_cancel(id: String, lease: String, staged: Option<Vec<String>>) -> bool {
            true
        }
    }

    fn request(name: &str, body: tauri::ipc::InvokeBody) -> tauri::webview::InvokeRequest {
        tauri::webview::InvokeRequest {
            cmd: name.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body,
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        }
    }

    #[test]
    fn native_io_arguments_match_the_original_tauri_contract() {
        let original = tauri::test::mock_builder()
            .invoke_handler(tauri::generate_handler![
                original::chat_answer,
                original::chat_respond,
                original::chat_control,
                original::chat_set_mode,
                original::chat_set_option,
                original::chat_stop,
                original::chat_capture,
                original::chat_say,
                original::term_key,
                original::term_resize,
                original::term_ready,
                original::term_forget,
                original::term_drop,
                original::term_drop_cancel,
            ])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let current = tauri::test::mock_builder()
            .invoke_handler(|invoke| {
                dispatch_into(invoke, |invoke, _| invoke.resolver.respond(Ok(true))).is_ok()
            })
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let before = tauri::WebviewWindowBuilder::new(&original, "before", Default::default())
            .build()
            .unwrap();
        let after = tauri::WebviewWindowBuilder::new(&current, "after", Default::default())
            .build()
            .unwrap();
        let cases = [
            (
                "chat_answer",
                json!({"id":"fixture","lease":"fixture-lease","ask":1}),
            ),
            (
                "chat_respond",
                json!({"id":"fixture","lease":"fixture-lease","ask":1,"response":{"yes":true}}),
            ),
            (
                "chat_control",
                json!({"id":"fixture","lease":"fixture-lease","token":"t","operation":"close","payload":{}}),
            ),
            (
                "chat_set_mode",
                json!({"id":"fixture","lease":"fixture-lease","mode":"plan"}),
            ),
            (
                "chat_set_option",
                json!({"id":"fixture","lease":"fixture-lease","option":"plan","value":true}),
            ),
            ("chat_stop", json!({"id":"fixture","lease":"fixture-lease"})),
            (
                "chat_capture",
                json!({"id":"fixture","lease":"fixture-lease","token":"00112233445566778899aabbccddeeff"}),
            ),
            (
                "chat_say",
                json!({"id":"fixture","lease":"fixture-lease","text":"hi"}),
            ),
            (
                "chat_say",
                json!({"id":"fixture","lease":"fixture-lease","text":"hi","token":"draft-1"}),
            ),
            (
                "term_key",
                json!({"id":"fixture","lease":"fixture-lease","keys":"hi"}),
            ),
            (
                "term_resize",
                json!({"id":"fixture","lease":"fixture-lease","cols":80,"rows":24}),
            ),
            (
                "term_ready",
                json!({"id":"fixture","lease":"fixture-lease"}),
            ),
            (
                "term_forget",
                json!({"id":"fixture","lease":"fixture-lease","redraw":true}),
            ),
            (
                "term_drop",
                json!({"id":"fixture","lease":"fixture-lease","paths":[]}),
            ),
            (
                "term_drop_cancel",
                json!({"id":"fixture","lease":"fixture-lease"}),
            ),
        ];
        let response = |view: &tauri::WebviewWindow<tauri::test::MockRuntime>, name, body| {
            tauri::test::get_ipc_response(view, request(name, body))
                .map(|body| body.deserialize::<Value>().unwrap())
        };
        for (name, valid) in cases {
            let mut variants = vec![
                valid.clone(),
                json!({}),
                json!({"id":3}),
                json!({"id":null}),
            ];
            for key in valid.as_object().unwrap().keys() {
                let mut missing = valid.clone();
                missing.as_object_mut().unwrap().remove(key);
                variants.push(missing);
                let mut wrong = valid.clone();
                wrong[key] = json!([]);
                variants.push(wrong);
            }
            for body in variants
                .into_iter()
                .map(tauri::ipc::InvokeBody::Json)
                .chain([tauri::ipc::InvokeBody::Raw(vec![1])])
            {
                assert_eq!(
                    response(&after, name, body.clone()),
                    response(&before, name, body),
                    "{name}"
                );
            }
        }
        for token in [
            json!(null),
            json!("draft-1"),
            json!(3),
            json!([]),
            json!(""),
            json!("x".repeat(129)),
            json!("bad token"),
            json!("秘密"),
        ] {
            let body = json!({"id":"fixture","lease":"fixture-lease","text":"hi","token":token});
            assert_eq!(
                response(&after, "chat_say", body.clone().into()),
                response(&before, "chat_say", body.into())
            );
        }
        for (name, body) in [
            (
                "term_resize",
                json!({"id":"fixture","lease":"fixture-lease","cols":65536,"rows":24}),
            ),
            (
                "chat_answer",
                json!({"id":"fixture","lease":"fixture-lease","ask":-1,"option":null}),
            ),
            (
                "chat_say",
                json!({"id":"fixture","lease":"fixture-lease","text":"hi","images":[["png"]]}),
            ),
            (
                "term_drop",
                json!({"id":"fixture","lease":"fixture-lease","paths":[],"from":null,"into":null}),
            ),
        ] {
            assert_eq!(
                response(&after, name, body.clone().into()),
                response(&before, name, body.into())
            );
        }
    }

    #[test]
    fn native_io_admits_before_handler_return_and_preserves_unknown_invokes() {
        let admitted = Arc::new(Mutex::new(Vec::new()));
        let accepted = admitted.clone();
        let fallback = Arc::new(AtomicUsize::new(0));
        let fell_back = fallback.clone();
        let app = tauri::test::mock_builder()
            .invoke_handler(move |invoke| {
                let expected = accepted.lock().unwrap().len() + 1;
                match dispatch_into(invoke, |invoke, request| {
                    let Request::Key { keys, .. } = request else {
                        panic!("unexpected fixture command")
                    };
                    accepted.lock().unwrap().push(keys);
                    invoke.resolver.respond(Ok(true));
                }) {
                    Ok(()) => assert_eq!(
                        accepted.lock().unwrap().len(),
                        expected,
                        "admission cannot wait for an async poll"
                    ),
                    Err(invoke) => {
                        assert_eq!(invoke.message.command(), "fixture_unknown");
                        fell_back.fetch_add(1, Ordering::SeqCst);
                        invoke.resolver.respond(Ok("fallback"));
                    }
                }
                true
            })
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let view = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        for key in ["a", "b", "c"] {
            tauri::test::get_ipc_response(
                &view,
                request(
                    "term_key",
                    json!({"id":"fixture","lease":"fixture-lease","keys":key}).into(),
                ),
            )
            .unwrap();
        }
        let unknown =
            tauri::test::get_ipc_response(&view, request("fixture_unknown", json!({}).into()))
                .unwrap();
        assert_eq!(unknown.deserialize::<String>().unwrap(), "fallback");
        assert_eq!(*admitted.lock().unwrap(), ["a", "b", "c"]);
        assert_eq!(fallback.load(Ordering::SeqCst), 1);
        // Remote origins still go through Tauri's ACL before our handler.
        let mut remote = request(
            "term_key",
            json!({"id":"fixture","lease":"fixture-lease","keys":"forbidden"}).into(),
        );
        remote.url = "https://untrusted.invalid".parse().unwrap();
        assert!(tauri::test::get_ipc_response(&view, remote).is_err());
        assert_eq!(admitted.lock().unwrap().len(), 3);
    }
}
