use khor_node::updates::{self, Channel, UpgradeRequest};

pub(super) fn run(args: &[String]) -> Result<(), String> {
    let (request, json) = parse(args)?;
    let trust = updates::update_trust()?;
    let mut last = 0;
    let result = super::rt()?.block_on(updates::upgrade_local(request, trust, |progress| {
        let percent = progress.received.saturating_mul(100) / progress.total.max(1);
        if !json && percent >= last + 10 {
            last = percent;
            super::err(format_args!(
                "{}\n",
                khor_catalog::cli::upgrade_download(percent)
            ));
        }
    }))?;
    if json {
        super::out(format_args!("{}\n", super::json_output(&result)?));
        return Ok(());
    }
    let message = match (&result.target, result.installed) {
        (Some(target), true) => khor_catalog::cli::upgrade_installed(target.tag.as_str()),
        (Some(target), false) => khor_catalog::cli::upgrade_available(target.tag.as_str()),
        _ => khor_catalog::cli::upgrade_current(result.current.tag.as_str()),
    };
    super::out(format_args!("{message}\n"));
    Ok(())
}

fn parse(args: &[String]) -> Result<(UpgradeRequest, bool), String> {
    let mut request = UpgradeRequest::default();
    let mut json = false;
    let mut cursor = 0;
    while cursor < args.len() {
        match args[cursor].as_str() {
            "--check" if !request.check_only => request.check_only = true,
            "--json" if !json => json = true,
            "--channel" if request.channel.is_none() => {
                cursor += 1;
                request.channel = Some(match args.get(cursor).map(String::as_str) {
                    Some("stable") => Channel::Stable,
                    Some("beta") => Channel::Beta,
                    _ => return Err(khor_catalog::cli::upgrade_usage().into()),
                });
            }
            "--version" if request.version.is_none() => {
                cursor += 1;
                request.version = Some(
                    args.get(cursor)
                        .ok_or_else(|| khor_catalog::cli::upgrade_usage().to_owned())?
                        .clone()
                        .try_into()?,
                );
            }
            _ => return Err(khor_catalog::cli::upgrade_usage().into()),
        }
        cursor += 1;
    }
    Ok((request, json))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(value: &str) -> Vec<String> {
        value.split_whitespace().map(str::to_owned).collect()
    }
    #[test]
    fn update_choices_are_explicit_and_invalid_or_repeated_flags_do_not_silently_change_scope() {
        let (request, json) = parse(&args(
            "--check --channel beta --version v0.2.0-rc.10 --json",
        ))
        .unwrap();
        assert!(request.check_only && json);
        assert_eq!(request.channel, Some(Channel::Beta));
        assert_eq!(request.version.unwrap().as_str(), "v0.2.0-rc.10");
        for bad in [
            "--check --check",
            "--channel",
            "--channel edge",
            "--channel beta --channel stable",
            "--version latest",
            "--version",
            "--all",
            "unknown-machine",
        ] {
            assert!(parse(&args(bad)).is_err(), "{bad}");
        }
    }
}
