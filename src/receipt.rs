use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub fn fingerprint(value: &impl Serialize) -> Result<String> {
    let digest = Sha256::digest(serde_json::to_vec(value)?);
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(out, "{:02x}", b);
    }
    Ok(out)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub schema_version: u8,
    pub provider: String,
    pub created_at: String,
    pub input_sha256: String,
    pub output_sha256: String,
    pub duration_ms: u64,
    pub provider_token_usage: Option<serde_json::Value>,
    pub sha256: String,
}

impl Receipt {
    pub fn new(
        input: &impl Serialize,
        output: &impl Serialize,
        duration_ms: u64,
        usage: Option<serde_json::Value>,
    ) -> Result<Self> {
        Self::from_input_hash(fingerprint(input)?, output, duration_ms, usage)
    }

    pub fn from_input_hash(
        input_sha256: String,
        output: &impl Serialize,
        duration_ms: u64,
        usage: Option<serde_json::Value>,
    ) -> Result<Self> {
        anyhow::ensure!(
            input_sha256.len() == 64 && input_sha256.bytes().all(|b| b.is_ascii_hexdigit()),
            "invalid input fingerprint"
        );
        let mut receipt = Self {
            schema_version: 1,
            provider: "gmi".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            input_sha256,
            output_sha256: fingerprint(output)?,
            duration_ms,
            provider_token_usage: usage,
            sha256: String::new(),
        };
        receipt.sha256 = fingerprint(&receipt)?;
        Ok(receipt)
    }

    pub fn verify(&self) -> Result<bool> {
        let mut value = serde_json::to_value(self)?;
        value["sha256"] = "".into();
        // Serialize through the struct to preserve the declared field order.
        let unsigned: Self = serde_json::from_value(value)?;
        Ok(fingerprint(&unsigned)? == self.sha256)
    }
}
