use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};
use tokio::sync::Semaphore;

use crate::tools::config_tool::RuntimeConfig;

pub const DEFAULT_SLOTS_PER_TENANT: usize = 4;
pub const MAX_SLOTS_PER_TENANT: usize = 64;
const IDLE_PRUNE_AFTER: Duration = Duration::from_secs(300);
const PRUNE_THRESHOLD: usize = 128;

// Do not derive Debug: this structure may hold per-request BYOK credentials.
#[derive(Clone)]
pub struct TenantContext {
    pub tenant_id: String,
    pub agentbox_key: Option<String>,
    pub maas_key: Option<String>,
    pub github_token: Option<String>,
    pub require_byok: bool,
}

pub struct TenantHandles {
    pub agentbox_slots: Arc<Semaphore>,
    pub maas_slots: Arc<Semaphore>,
    pub runtime: Arc<RwLock<RuntimeConfig>>,
}

struct TenantState {
    agentbox_slots: Arc<Semaphore>,
    maas_slots: Arc<Semaphore>,
    runtime: Arc<RwLock<RuntimeConfig>>,
    last_seen: Instant,
}

#[derive(Clone)]
pub struct TenantRegistry {
    max_slots_per_tenant: usize,
    base_runtime: RuntimeConfig,
    tenants: Arc<Mutex<HashMap<String, TenantState>>>,
    template_locks: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

impl TenantRegistry {
    pub fn new(base_runtime: RuntimeConfig, max_slots_per_tenant: usize) -> Self {
        let clamped = max_slots_per_tenant.clamp(1, MAX_SLOTS_PER_TENANT);
        Self {
            max_slots_per_tenant: clamped,
            base_runtime,
            tenants: Arc::new(Mutex::new(HashMap::new())),
            template_locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn handles(&self, tenant_id: &str) -> TenantHandles {
        let now = Instant::now();
        let mut map = self
            .tenants
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if map.len() >= PRUNE_THRESHOLD {
            let max_slots = self.max_slots_per_tenant;
            map.retain(|key, state| {
                if key == tenant_id {
                    return true;
                }
                let idle = Arc::strong_count(&state.agentbox_slots) == 1
                    && Arc::strong_count(&state.maas_slots) == 1
                    && state.agentbox_slots.available_permits() == max_slots
                    && state.maas_slots.available_permits() == max_slots;
                !(idle && now.saturating_duration_since(state.last_seen) >= IDLE_PRUNE_AFTER)
            });
        }
        let state = map
            .entry(tenant_id.to_owned())
            .or_insert_with(|| TenantState {
                agentbox_slots: Arc::new(Semaphore::new(self.max_slots_per_tenant)),
                maas_slots: Arc::new(Semaphore::new(self.max_slots_per_tenant)),
                runtime: Arc::new(RwLock::new(self.base_runtime.clone())),
                last_seen: now,
            });
        state.last_seen = now;
        TenantHandles {
            agentbox_slots: state.agentbox_slots.clone(),
            maas_slots: state.maas_slots.clone(),
            runtime: state.runtime.clone(),
        }
    }

    pub fn template_lock(
        &self,
        agentbox_key: Option<&str>,
        is_byok: bool,
    ) -> Arc<tokio::sync::Mutex<()>> {
        let bucket = match (is_byok, agentbox_key) {
            (true, Some(key)) => hash_identifier("template-byok", key),
            _ => "managed".to_owned(),
        };
        let mut locks = self
            .template_locks
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if locks.len() >= PRUNE_THRESHOLD {
            locks.retain(|key, lock| key == &bucket || Arc::strong_count(lock) > 1);
        }
        locks
            .entry(bucket)
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}

/// Derives a fixed-length, non-secret identifier using domain-separated SHA-256.
pub fn hash_identifier(namespace: &str, value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(namespace.as_bytes());
    hasher.update([0u8]);
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(32);
    for byte in &digest[..16] {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

/// Constant-time secret equality via SHA-256 digest comparison (avoids length and timing leaks).
pub fn constant_time_eq_secret(provided: &str, expected: &str) -> bool {
    let a = Sha256::digest(provided.as_bytes());
    let b = Sha256::digest(expected.as_bytes());
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    std::hint::black_box(diff) == 0
}

/// Checks whether `provided` matches any configured secret without short-circuiting.
pub fn any_secret_matches(provided: &str, allowed: &[String]) -> bool {
    let mut matched = false;
    for candidate in allowed {
        matched |= constant_time_eq_secret(provided, candidate);
    }
    matched
}
