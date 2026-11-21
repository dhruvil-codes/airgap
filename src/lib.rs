pub mod config;
pub mod doctor;
pub mod gmi;
pub mod receipt;
pub mod setup;
pub mod tools;

/// Terminal-only header for help, setup and doctor; never sent over MCP.
pub fn banner(subtitle: &str) -> String {
    format!(
        "\x1b[1;36m  ▄▀█ █ █▀█ █▀▀ ▄▀█ █▀█\x1b[0m\n\
         \x1b[1;36m  █▀█ █ █▀▄ █▄█ █▀█ █▀▀\x1b[0m\n\
         \x1b[90m  🛡️  airgap {subtitle} · GMI AgentBox v2\x1b[0m\n"
    )
}

pub const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;

pub fn check_text(text: &str) -> anyhow::Result<()> {
    anyhow::ensure!(text.len() <= MAX_TEXT_BYTES, "text exceeds 2 MiB limit");
    Ok(())
}
