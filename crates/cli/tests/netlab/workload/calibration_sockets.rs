//! Read-only TCP snapshots owned by the same future as the transfer.
use std::future::Future;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use serde_json::json;

use super::{Model, emit};

pub(super) async fn observe<F: Future>(
    transfer: F,
    model: Model,
    side: &'static str,
    operation: &'static str,
    local: SocketAddr,
    peer: SocketAddr,
) -> F::Output {
    if !model.socket_diagnostic || operation == "echo" {
        return transfer.await;
    }
    let began = Instant::now();
    let observations = async {
        for &second in model.socket_plan {
            tokio::time::sleep_until((began + Duration::from_secs(second)).into()).await;
            let started_us = began.elapsed().as_micros();
            let mut command = tokio::process::Command::new("ss");
            command.kill_on_drop(true).args([
                "-tinm",
                "state",
                "established",
                "src",
                &local.to_string(),
                "dst",
                &peer.to_string(),
            ]);
            let captured = match tokio::time::timeout(Duration::from_secs(1), command.output())
                .await
            {
                Ok(Ok(out)) => json!({"ok":out.status.success(),"timed_out":false,
                    "stdout":String::from_utf8_lossy(&out.stdout),"stderr":String::from_utf8_lossy(&out.stderr)}),
                Ok(Err(error)) => json!({"ok":false,"timed_out":false,"error":error.to_string()}),
                Err(_) => json!({"ok":false,"timed_out":true,"error":"socket snapshot deadline"}),
            };
            emit(
                json!({"schema":1,"revision":model.revision,"event":"tcp_snapshot",
                "side":side,"operation":operation,"local":local.to_string(),"peer":peer.to_string(),
                "clock_origin":"local_payload_start","planned_after_us":second*1000000,
                "started_after_us":started_us,"ended_after_us":began.elapsed().as_micros(),
                "command":captured}),
            );
        }
    };
    tokio::pin!(transfer);
    tokio::pin!(observations);
    tokio::select! {
        result = &mut transfer => result,
        _ = &mut observations => transfer.await,
    }
}
