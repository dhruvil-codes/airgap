use std::sync::LazyLock;

use regex::Regex;
use rmcp::schemars;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RedactRequest {
    pub raw_text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Redaction {
    pub sanitized_text: String,
    pub redacted_items_count: usize,
}

static PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
    r"(?s)-----BEGIN (?:[A-Z0-9]+ )*PRIVATE KEY-----.*?(?:-----END (?:[A-Z0-9]+ )*PRIVATE KEY-----|\z)",
    r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b",
    r"\b(?:gh[pousr]_[A-Za-z0-9]{20,255}|github_pat_[A-Za-z0-9_]{20,255})\b",
    r"\b(?:sk-[A-Za-z0-9_-]{16,255}|xox[baprs]-[A-Za-z0-9-]{16,255})\b",
    r"\beyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b",
].into_iter().map(|pattern| Regex::new(pattern).expect("static regex")).collect()
});

// A sensitive keyword must be a whole segment of the key name: delimited by `_`/`-`
// (`GITHUB_TOKEN`, `x-api-key`) or a camelCase hump (`accessToken`, `secretKey`).
// `max_tokens` and `tokenizer` therefore do not match.
static ASSIGNMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r#"(?m)["']?\b(?P<key>(?:"#,
        r"(?:[A-Za-z0-9]+[_-])*(?i:api[_-]?key|access[_-]?key|secret|token|password|passwd|credentials?|authorization)",
        r"|[A-Za-z0-9]*[a-z0-9](?:Api_?Key|Access_?Key|Secret|Token|Password|Passwd|Credentials?|Authorization)",
        r")(?:[_-][A-Za-z0-9]+|[A-Z][a-z0-9]+)*)\b",
        r#"["']?[ \t]*[:=][ \t]*(?i:bearer[ \t]+)?(?:"(?P<double>(?:\\.|[^"\\\r\n])*)(?:"|$)|'(?P<single>(?:\\.|[^'\\\r\n])*)(?:'|$)|(?P<plain>[^\s,;#}\r\n]+))"#,
    ))
    .expect("static regex")
});

static BEARER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bbearer\s+(?P<value>[A-Za-z0-9._~+/-]+=*)").expect("static regex")
});
static TOKENS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z0-9_+/=-]{24,}").expect("static regex"));

pub fn entropy(value: &str) -> f64 {
    let mut counts = [0usize; 256];
    for byte in value.bytes() {
        counts[byte as usize] += 1;
    }
    if value.is_empty() {
        return 0.0;
    }
    counts
        .into_iter()
        .filter(|count| *count > 0)
        .map(|count| {
            let probability = count as f64 / value.len() as f64;
            -probability * probability.log2()
        })
        .sum()
}

pub fn redact(text: &str) -> Redaction {
    let mut spans = Vec::new();
    for pattern in PATTERNS.iter() {
        spans.extend(pattern.find_iter(text).map(|m| (m.start(), m.end())));
    }
    for captures in ASSIGNMENT.captures_iter(text) {
        if let Some(value) = captures
            .name("double")
            .or_else(|| captures.name("single"))
            .or_else(|| captures.name("plain"))
        {
            let text = value.as_str();
            let key = captures["key"].to_ascii_lowercase();
            // Passwords are often short, so any value counts. For other keys, short or
            // numeric values are settings (`retries_token_bucket = 3`), not credentials.
            let sensitive = if key.contains("password") || key.contains("passwd") {
                !text.is_empty()
            } else {
                text.len() >= 8 && !text.bytes().all(|b| b.is_ascii_digit())
            };
            if sensitive {
                spans.push((value.start(), value.end()));
            }
        }
    }
    for captures in BEARER.captures_iter(text) {
        if let Some(value) = captures.name("value") {
            spans.push((value.start(), value.end()));
        }
    }
    for token in TOKENS.find_iter(text) {
        let value = token.as_str();
        if value.len() <= 4096
            && value.bytes().any(|b| b.is_ascii_alphabetic())
            && value.bytes().any(|b| b.is_ascii_digit())
            && entropy(value) >= 4.0
        {
            spans.push((token.start(), token.end()));
        }
    }
    spans.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in spans {
        if let Some(last) = merged.last_mut() {
            if start < last.1 {
                last.1 = last.1.max(end);
                continue;
            }
        }
        merged.push((start, end));
    }
    let mut output = String::with_capacity(text.len());
    let mut cursor = 0;
    for &(start, end) in &merged {
        output.push_str(&text[cursor..start]);
        output.push_str("[REDACTED]");
        cursor = end;
    }
    output.push_str(&text[cursor..]);
    Redaction {
        sanitized_text: output,
        redacted_items_count: merged.len(),
    }
}
