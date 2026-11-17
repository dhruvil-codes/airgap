use airgap::{config::Config, tools::Airgap};
use rmcp::{transport::io::stdio_pipes, ServiceExt};

fn main() -> anyhow::Result<()> {
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
