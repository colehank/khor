use crate::updates::{self, ProgramInfo, UpgradeOutcome, UpgradeRequest};
use iroh::endpoint::{Connection, RecvStream, SendStream};
use khor_update::{Channel, Component, Download, Feed, SignedRelease, TrustRoot};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const FRAME_LIMIT: usize = 384 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Request {
    Describe,
    Install {
        release: SignedRelease,
        channel: Channel,
        expected: khor_update::ReleaseIdentity,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum Response {
    Description {
        program: ProgramInfo,
        current: khor_update::ReleaseIdentity,
        channel: Channel,
    },
    Upload,
    Installed {
        outcome: UpgradeOutcome,
    },
    Refused {
        why: String,
    },
}

async fn write<T: Serialize>(
    stream: &mut (impl AsyncWrite + Unpin),
    value: &T,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if bytes.len() > FRAME_LIMIT {
        return Err("update message exceeds its limit".into());
    }
    stream
        .write_u32(bytes.len() as u32)
        .await
        .map_err(|e| e.to_string())?;
    stream.write_all(&bytes).await.map_err(|e| e.to_string())
}

async fn read<T: DeserializeOwned>(stream: &mut (impl AsyncRead + Unpin)) -> Result<T, String> {
    let size = stream.read_u32().await.map_err(|e| e.to_string())? as usize;
    if size > FRAME_LIMIT {
        return Err("update message exceeds its limit".into());
    }
    let mut bytes = vec![0; size];
    stream
        .read_exact(&mut bytes)
        .await
        .map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub(crate) async fn serve(
    mut send: SendStream,
    mut recv: RecvStream,
    paired: bool,
) -> Result<(), String> {
    let result = if paired {
        answer(&mut send, &mut recv).await
    } else {
        Err("only paired, unpaused machines can manage updates".into())
    };
    if let Err(why) = result {
        write(&mut send, &Response::Refused { why }).await?;
    }
    send.finish().map_err(|e| e.to_string())
}

#[cfg(unix)]
async fn answer(send: &mut SendStream, recv: &mut RecvStream) -> Result<(), String> {
    let request = read::<Request>(recv).await?;
    let (installation, program, current, default_channel) = updates::cli_installation()?;
    match request {
        Request::Describe => {
            write(
                send,
                &Response::Description {
                    program,
                    current,
                    channel: default_channel,
                },
            )
            .await
        }
        Request::Install {
            release,
            channel,
            expected,
        } => {
            if current != expected {
                return Err("this machine changed version; check again before updating".into());
            }
            let trust = TrustRoot::bundled()?;
            let verified = trust.restore(&release)?;
            let offer = verified
                .select(
                    Component::Cli,
                    program.platform,
                    &current,
                    channel,
                    Some(&verified.manifest().identity.tag),
                )?
                .ok_or("release has no CLI for this machine")?;
            let target = offer.identity().clone();
            let download =
                match Download::fetch(offer.clone(), &trust, &installation.staging(), |_| {}).await
                {
                    Ok(download) => download,
                    Err(_) => {
                        write(send, &Response::Upload).await?;
                        Download::receive(offer, &trust, recv, &installation.staging()).await?
                    }
                };
            tokio::task::spawn_blocking(move || {
                updates::install_cli(&installation, &download, channel, Some(&expected))
            })
            .await
            .map_err(|e| e.to_string())??;
            write(
                send,
                &Response::Installed {
                    outcome: UpgradeOutcome {
                        current,
                        target: Some(target),
                        installed: true,
                    },
                },
            )
            .await
        }
    }
}

#[cfg(not(unix))]
async fn answer(_: &mut SendStream, _: &mut RecvStream) -> Result<(), String> {
    Err("this installation uses its external package manager".into())
}

pub(crate) async fn upgrade(
    connection: &Connection,
    request: UpgradeRequest,
    staging: &std::path::Path,
) -> Result<UpgradeOutcome, String> {
    let (mut send, mut recv) = connection.open_bi().await.map_err(|e| e.to_string())?;
    write(&mut send, &Request::Describe).await?;
    send.finish().map_err(|e| e.to_string())?;
    let (program, current, default_channel) = match read::<Response>(&mut recv).await? {
        Response::Description {
            program,
            current,
            channel,
        } => (program, current, channel),
        Response::Refused { why } => return Err(why),
        _ => return Err("machine returned an unexpected update description".into()),
    };
    let trust = TrustRoot::bundled()?;
    let channel = request.channel.unwrap_or(default_channel);
    let Some(release) = Feed::github(trust.clone())?
        .check(&current, channel, request.version.as_ref())
        .await?
    else {
        return Ok(UpgradeOutcome {
            current,
            target: None,
            installed: false,
        });
    };
    let Some(offer) = release.select(
        Component::Cli,
        program.platform,
        &current,
        channel,
        request.version.as_ref(),
    )?
    else {
        return Ok(UpgradeOutcome {
            current,
            target: None,
            installed: false,
        });
    };
    if request.check_only {
        return Ok(UpgradeOutcome {
            current,
            target: Some(offer.identity().clone()),
            installed: false,
        });
    }
    let (mut send, mut recv) = connection.open_bi().await.map_err(|e| e.to_string())?;
    write(
        &mut send,
        &Request::Install {
            release: release.signed().clone(),
            channel,
            expected: current.clone(),
        },
    )
    .await?;
    let response = match read::<Response>(&mut recv).await? {
        Response::Upload => {
            let download = Download::fetch(offer.clone(), &trust, staging, |_| {}).await?;
            let mut file = tokio::fs::File::open(download.archive())
                .await
                .map_err(|e| e.to_string())?;
            tokio::io::copy(&mut file, &mut send)
                .await
                .map_err(|e| e.to_string())?;
            send.finish().map_err(|e| e.to_string())?;
            read::<Response>(&mut recv).await?
        }
        response => response,
    };
    match response {
        Response::Installed { outcome }
            if outcome.installed
                && outcome.current == current
                && outcome.target.as_ref() == Some(offer.identity()) =>
        {
            Ok(outcome)
        }
        Response::Refused { why } => Err(why),
        _ => Err("machine did not confirm the requested release installation".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn oversized_update_headers_are_rejected_before_their_payload_is_read() {
        let mut bytes = ((FRAME_LIMIT + 1) as u32).to_be_bytes().as_slice().to_vec();
        assert!(
            read::<Request>(&mut bytes.as_slice())
                .await
                .err()
                .unwrap()
                .contains("exceeds")
        );
        bytes = b"\0\0\0\x02{}".to_vec();
        assert!(read::<Request>(&mut bytes.as_slice()).await.is_err());
    }
}
