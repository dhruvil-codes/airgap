pub mod config;
pub mod gmi;
pub mod receipt;
pub mod tools;

pub const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;

pub fn check_text(text: &str) -> anyhow::Result<()> {
    anyhow::ensure!(text.len() <= MAX_TEXT_BYTES, "text exceeds 2 MiB limit");
    Ok(())
}
