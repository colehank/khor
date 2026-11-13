//! Native terminal entry point for the shared command engine.
fn main() {
    #[cfg(feature = "net-diagnostics")]
    if let Some(mode) = std::env::var_os("KHOR_NET_DIAGNOSTICS") {
        let lifecycle_only = mode == "lifecycle";
        let filter = if lifecycle_only {
            "off"
        } else {
            "khor_net_samples=debug"
        };
        if tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_ansi(false)
            .with_writer(std::io::stderr)
            .try_init()
            .is_ok()
        {
            eprintln!(
                "[khor-net-diagnostics] mode={} samples={}",
                if lifecycle_only { "lifecycle" } else { "full" },
                if lifecycle_only { "off" } else { "debug" },
            );
        }
    }
    if let Some(code) = khor_commands::console_main_if_requested(start_web) {
        std::process::exit(code);
    }
    let code = khor_commands::run_cli(start_web, write);
    std::process::exit(code);
}

fn start_web(root: &std::path::Path) {
    match khor_web::listen(root.to_path_buf(), khor_web::port()) {
        Ok(face) => eprintln!("{}", khor_catalog::cli::web_banner(face.addr)),
        Err(error) => eprintln!("{error}"),
    }
}

/// Native pipe handling belongs to the process entry point, never embedded requests.
fn write(stderr: bool, text: &str) {
    use std::io::Write;
    if stderr {
        let _ = std::io::stderr().lock().write_all(text.as_bytes());
        return;
    }
    if let Err(error) = std::io::stdout().lock().write_all(text.as_bytes()) {
        if error.kind() == std::io::ErrorKind::BrokenPipe {
            #[cfg(unix)]
            unsafe {
                libc::signal(libc::SIGPIPE, libc::SIG_DFL);
                libc::raise(libc::SIGPIPE);
            }
            std::process::exit(141);
        }
        panic!("failed printing to stdout: {error}");
    }
}
