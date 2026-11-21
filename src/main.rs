use airgap::{config::Config, doctor, setup, tools::Airgap};
use rmcp::{transport::io::stdio_pipes, ServiceExt};

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("init" | "setup") => return setup::run(&args[1..]),
        Some("doctor") if args.len() == 1 => return doctor::run(),
        Some("--help" | "-h" | "help") if args.len() == 1 => {
            let banner = airgap::banner(concat!("v", env!("CARGO_PKG_VERSION")));
            println!("{banner}\nUsage: airgap [--mcp]\n       airgap init|setup [options]\n       airgap doctor\n       airgap --version\n\nNo arguments or --mcp: serve MCP JSON-RPC over stdio.\ninit/setup: configure editors and agent routing rules.\ndoctor: check credentials, gateways, model access and editor registration.\n\nRun airgap setup --help for setup options.");
            return Ok(());
        }
        Some("--version" | "-v" | "-V") if args.len() == 1 => {
            println!("airgap {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        None => {}
        Some("--mcp") if args.len() == 1 => {}
        _ => anyhow::bail!("unknown command or arguments; run airgap --help"),
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    let result = runtime.block_on(run());
    // Blocking stdin reads on Windows cannot be cancelled by Tokio.
    runtime.shutdown_timeout(std::time::Duration::from_secs(1));
    result
}

async fn run() -> anyhow::Result<()> {
    // stdout belongs exclusively to MCP JSON-RPC.
    let server = Airgap::new(Config::discover()?)?;
    let service = server.clone().serve(stdio_pipes()).await?;
    let result = tokio::select! {
        result = service.waiting() => result.map(|_| ()),
        _ = shutdown_signal() => Ok(()),
    };
    server.shutdown().await;
    result?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        if let Ok(mut terminate) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = terminate.recv() => {},
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}
