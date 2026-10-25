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
    "chat_replay",
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
        command: Control,
    },
    Say {
        id: String,
        text: String,
        images: Vec<(String, String)>,
        files: Vec<(String, String)>,
    },
    Key {
        id: String,
        keys: String,
    },
    Resize {
        id: String,
        cols: u16,
        rows: u16,
    },
    Ready {
        id: String,
    },
    Forget {
        id: String,
        redraw: bool,
    },
    Drop {
        id: String,
        paths: Vec<String>,
        from: Option<String>,
        into: Option<String>,
    },
    CancelDrop {
        id: String,
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
        "chat_replay" => Control::Replay,
        "chat_say" => {
            return Ok(Request::Say {
                id,
                text: arg!("text"),
                images: argument::<R, Option<Vec<(String, String)>>>(invoke, name, "images")?
                    .unwrap_or_default(),
                files: argument::<R, Option<Vec<(String, String)>>>(invoke, name, "files")?
                    .unwrap_or_default(),
            });
        }
        "term_key" => {
            return Ok(Request::Key {
                id,
                keys: arg!("keys"),
            });
        }
        "term_resize" => {
            return Ok(Request::Resize {
                id,
                cols: arg!("cols"),
                rows: arg!("rows"),
            });
        }
        "term_ready" => return Ok(Request::Ready { id }),
        "term_forget" => {
            return Ok(Request::Forget {
                id,
                redraw: arg!("redraw"),
            });
        }
        "term_drop" => {
            return Ok(Request::Drop {
                id,
                paths: arg!("paths"),
                from: arg!("from"),
                into: arg!("into"),
            });
        }
        "term_drop_cancel" => {
            return Ok(Request::CancelDrop {
                id,
                staged: argument::<R, Option<Vec<String>>>(invoke, name, "staged")?
                    .unwrap_or_default(),
            });
        }
        _ => unreachable!("dispatch checked the command name"),
    };
    Ok(Request::Chat { id, command })
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
    match request {
        Request::Chat { id, command } => reply(invoke, chat::submit_control(&id, command)),
        Request::Say {
            id,
            text,
            images,
            files,
        } => reply(invoke, chat::submit_say(&id, text, images, files)),
        Request::Key { id, keys } => reply(invoke, term::submit_key(&id, keys.into_bytes())),
        Request::Resize { id, cols, rows } => reply(invoke, term::submit_resize(&id, cols, rows)),
        Request::Ready { id } => reply(invoke, term::submit_ready(&id)),
        Request::Forget { id, redraw } => reply(invoke, term::submit_forget(&id, redraw)),
        Request::Drop {
            id,
            paths,
            from,
            into,
        } => {
            let job = term::begin_drop(&id);
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
        Request::CancelDrop { id, staged } => {
            // Cancellation is immediate; filesystem cleanup must not make
            // the UI wait, nor cancel a later drop after scheduling a task.
            term::cancel_drop(&id);
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
        pub fn chat_answer(id: String, ask: u64, option: Option<String>) -> bool {
            true
        }
        #[tauri::command]
        pub fn chat_respond(id: String, ask: u64, response: serde_json::Value) -> bool {
            true
        }
        #[tauri::command]
        pub fn chat_control(
            id: String,
            token: String,
            operation: String,
            payload: serde_json::Value,
        ) -> bool {
            true
        }
        #[tauri::command]
        pub fn chat_set_mode(id: String, mode: String) -> bool {
            true
        }
        #[tauri::command]
        pub fn chat_set_option(id: String, option: String, value: serde_json::Value) -> bool {
            true
        }
        #[tauri::command]
        pub fn chat_stop(id: String) -> bool {
            true
        }
        #[tauri::command]
        pub fn chat_replay(id: String) -> bool {
            true
        }
        #[tauri::command]
        pub fn chat_say(
            id: String,
            text: String,
            images: Option<Vec<(String, String)>>,
            files: Option<Vec<(String, String)>>,
        ) -> bool {
            true
        }
        #[tauri::command]
        pub fn term_key(id: String, keys: String) -> bool {
            true
        }
        #[tauri::command]
        pub fn term_resize(id: String, cols: u16, rows: u16) -> bool {
            true
        }
        #[tauri::command]
        pub fn term_ready(id: String) -> bool {
            true
        }
        #[tauri::command]
        pub fn term_forget(id: String, redraw: bool) -> bool {
            true
        }
        #[tauri::command]
        pub fn term_drop(
            id: String,
            paths: Vec<String>,
            from: Option<String>,
            into: Option<String>,
        ) -> bool {
            true
        }
        #[tauri::command]
        pub fn term_drop_cancel(id: String, staged: Option<Vec<String>>) -> bool {
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
                original::chat_replay,
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
            ("chat_answer", json!({"id":"fixture","ask":1})),
            (
                "chat_respond",
                json!({"id":"fixture","ask":1,"response":{"yes":true}}),
            ),
            (
                "chat_control",
                json!({"id":"fixture","token":"t","operation":"close","payload":{}}),
            ),
            ("chat_set_mode", json!({"id":"fixture","mode":"plan"})),
            (
                "chat_set_option",
                json!({"id":"fixture","option":"plan","value":true}),
            ),
            ("chat_stop", json!({"id":"fixture"})),
            ("chat_replay", json!({"id":"fixture"})),
            ("chat_say", json!({"id":"fixture","text":"hi"})),
            ("term_key", json!({"id":"fixture","keys":"hi"})),
            ("term_resize", json!({"id":"fixture","cols":80,"rows":24})),
            ("term_ready", json!({"id":"fixture"})),
            ("term_forget", json!({"id":"fixture","redraw":true})),
            ("term_drop", json!({"id":"fixture","paths":[]})),
            ("term_drop_cancel", json!({"id":"fixture"})),
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
        for (name, body) in [
            (
                "term_resize",
                json!({"id":"fixture","cols":65536,"rows":24}),
            ),
            (
                "chat_answer",
                json!({"id":"fixture","ask":-1,"option":null}),
            ),
            (
                "chat_say",
                json!({"id":"fixture","text":"hi","images":[["png"]]}),
            ),
            (
                "term_drop",
                json!({"id":"fixture","paths":[],"from":null,"into":null}),
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
                request("term_key", json!({"id":"fixture","keys":key}).into()),
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
            json!({"id":"fixture","keys":"forbidden"}).into(),
        );
        remote.url = "https://untrusted.invalid".parse().unwrap();
        assert!(tauri::test::get_ipc_response(&view, remote).is_err());
        assert_eq!(admitted.lock().unwrap().len(), 3);
    }
}
