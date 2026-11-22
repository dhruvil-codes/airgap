pub mod config;
pub mod doctor;
pub mod gmi;
pub mod receipt;
pub mod setup;
pub mod tools;

/// Terminal-only header for help, setup and doctor; never sent over MCP.
pub fn banner(subtitle: &str) -> String {
    let row0 = "\x1b[38;2;56;189;248m▄▀█ █ █▀█ █▀▀ ▄▀█ █▀█\x1b[0m";
    let row1 = "\x1b[38;2;129;140;248m█▀█ █ █▀▄ █▄█ █▀█ █▀▀\x1b[0m";

    let desc = match subtitle {
        "doctor" => "doctor · GMI AgentBox v2",
        "setup" => "setup · client configuration",
        other => other,
    };

    format!(
        "\n  {}\n  {}\n  \x1b[1;32m🛡️\x1b[0m  \x1b[1mairgap\x1b[0m \x1b[90mv{} · {}\x1b[0m\n",
        row0,
        row1,
        env!("CARGO_PKG_VERSION"),
        desc
    )
}

pub const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;

pub fn check_text(text: &str) -> anyhow::Result<()> {
    anyhow::ensure!(text.len() <= MAX_TEXT_BYTES, "text exceeds 2 MiB limit");
    Ok(())
}
