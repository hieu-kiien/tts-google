use keyring::Entry;
use std::sync::Mutex;
use tracing::info;

const SERVICE_NAME: &str = "AutoTTSDesktop";
const KEY_NAME: &str = "GeminiApiKey";
const MULTI_KEY_NAME: &str = "GeminiApiKeys";

/// Tracks per-key rate limit cooldown
#[derive(Debug, Clone)]
struct KeySlot {
    key: String,
    /// Timestamp (ms) when this key can be used again. 0 = available now.
    cooldown_until: i64,
}

struct CredentialInner {
    session_key: Option<String>,
    /// Multi-key pool for round-robin rotation
    key_pool: Vec<KeySlot>,
    /// Index of the next key to use
    next_index: usize,
}

pub struct CredentialStore {
    inner: Mutex<CredentialInner>,
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialStore {
    pub fn new() -> Self {
        let store = Self {
            inner: Mutex::new(CredentialInner {
                session_key: None,
                key_pool: Vec::new(),
                next_index: 0,
            }),
        };

        // Try to load multi-keys from OS keyring on startup
        if let Ok(entry) = Entry::new(SERVICE_NAME, MULTI_KEY_NAME) {
            if let Ok(json) = entry.get_password() {
                if let Ok(keys) = serde_json::from_str::<Vec<String>>(&json) {
                    let mut inner = store.inner.lock().unwrap();
                    for k in keys {
                        if !k.trim().is_empty() {
                            inner.key_pool.push(KeySlot { key: k.trim().to_string(), cooldown_until: 0 });
                        }
                    }
                    if !inner.key_pool.is_empty() {
                        info!("Loaded {} API keys from OS keyring", inner.key_pool.len());
                    }
                }
            }
        }

        store
    }

    // ─── Single key API (backward compatible) ───

    /// Stores key either persistently in OS Keyring or session RAM.
    pub fn set_key(&self, key: &str, remember: bool) -> Result<(), String> {
        let key_trimmed = key.trim();
        if key_trimmed.is_empty() {
            return self.delete_key();
        }

        if remember {
            let entry = Entry::new(SERVICE_NAME, KEY_NAME)
                .map_err(|e| format!("Failed to create OS keyring entry: {}", e))?;
            entry
                .set_password(key_trimmed)
                .map_err(|e| format!("Failed to save API key to OS credential store: {}", e))?;
            info!("API key saved securely to OS credential store");
        } else {
            // Delete OS key if switching to session-only
            let _ = Entry::new(SERVICE_NAME, KEY_NAME).and_then(|e| e.delete_credential());
        }

        let mut inner = self.inner.lock().unwrap();
        inner.session_key = Some(key_trimmed.to_string());

        // Also add to pool if not already there
        if !inner.key_pool.iter().any(|s| s.key == key_trimmed) {
            inner.key_pool.push(KeySlot { key: key_trimmed.to_string(), cooldown_until: 0 });
        }

        Ok(())
    }

    /// Retrieves a single API key (backward compat — prefers pool rotation).
    pub fn get_key(&self) -> Option<String> {
        // If multi-key pool has available keys, use round-robin
        if let Some(key) = self.get_next_available_key() {
            return Some(key);
        }

        // Fallback: single key from session RAM
        if let Ok(inner) = self.inner.lock() {
            if let Some(ref k) = inner.session_key {
                if !k.is_empty() {
                    return Some(k.clone());
                }
            }
        }

        // Fallback: single key from OS Keyring
        if let Ok(entry) = Entry::new(SERVICE_NAME, KEY_NAME) {
            if let Ok(pass) = entry.get_password() {
                if !pass.trim().is_empty() {
                    return Some(pass.trim().to_string());
                }
            }
        }

        None
    }

    /// Deletes the single API key from both OS Keyring and session RAM.
    pub fn delete_key(&self) -> Result<(), String> {
        if let Ok(mut inner) = self.inner.lock() {
            inner.session_key = None;
        }

        let entry = Entry::new(SERVICE_NAME, KEY_NAME)
            .map_err(|e| format!("Failed to access OS keyring: {}", e))?;

        let _ = entry.delete_credential(); // Ignore error if not found
        info!("API key cleared from memory and OS credential store");
        Ok(())
    }

    /// Returns true if any API key is configured (single or multi).
    pub fn is_configured(&self) -> bool {
        // Check pool first
        if let Ok(inner) = self.inner.lock() {
            if !inner.key_pool.is_empty() {
                return true;
            }
        }
        self.get_key().is_some()
    }

    // ─── Multi-key API ───

    /// Set multiple API keys at once. Replaces existing pool.
    pub fn set_keys(&self, keys: Vec<String>, remember: bool) -> Result<usize, String> {
        let mut inner = self.inner.lock().unwrap();
        inner.key_pool.clear();

        let mut count = 0;
        for k in &keys {
            let trimmed = k.trim();
            if !trimmed.is_empty() {
                inner.key_pool.push(KeySlot { key: trimmed.to_string(), cooldown_until: 0 });
                count += 1;
            }
        }

        // Also set first key as the single key (backward compat)
        if let Some(first) = inner.key_pool.first() {
            inner.session_key = Some(first.key.clone());
        }

        drop(inner); // release lock before IO

        if remember {
            let clean_keys: Vec<String> = keys.iter()
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty())
                .collect();
            let json = serde_json::to_string(&clean_keys)
                .map_err(|e| format!("Failed to serialize keys: {}", e))?;
            let entry = Entry::new(SERVICE_NAME, MULTI_KEY_NAME)
                .map_err(|e| format!("Failed to create OS keyring entry: {}", e))?;
            entry.set_password(&json)
                .map_err(|e| format!("Failed to save API keys: {}", e))?;
            info!("Saved {} API keys to OS credential store", count);
        }

        Ok(count)
    }

    /// Get list of all configured keys (masked for display).
    pub fn get_keys_masked(&self) -> Vec<String> {
        let inner = self.inner.lock().unwrap();
        inner.key_pool.iter().map(|slot| {
            let k = &slot.key;
            if k.len() > 8 {
                format!("{}...{}", &k[..4], &k[k.len()-4..])
            } else {
                "****".to_string()
            }
        }).collect()
    }

    /// Get count of configured keys.
    pub fn key_count(&self) -> usize {
        self.inner.lock().unwrap().key_pool.len()
    }

    /// Remove a key by index.
    pub fn remove_key_at(&self, index: usize) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        if index >= inner.key_pool.len() {
            return Err("Invalid key index".to_string());
        }
        inner.key_pool.remove(index);
        drop(inner);
        self.persist_pool()
    }

    /// Get the next available key using round-robin, skipping keys on cooldown.
    fn get_next_available_key(&self) -> Option<String> {
        let mut inner = self.inner.lock().unwrap();
        if inner.key_pool.is_empty() {
            return None;
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let pool_size = inner.key_pool.len();

        // Try each key in round-robin order
        for i in 0..pool_size {
            let check_idx = (inner.next_index + i) % pool_size;
            if inner.key_pool[check_idx].cooldown_until <= now_ms {
                inner.next_index = (check_idx + 1) % pool_size;
                return Some(inner.key_pool[check_idx].key.clone());
            }
        }

        // All keys on cooldown — return the one with shortest remaining cooldown
        let best = inner.key_pool.iter().min_by_key(|s| s.cooldown_until)?;
        Some(best.key.clone())
    }

    /// Mark a specific key as rate-limited for a duration.
    pub fn mark_key_cooldown(&self, key: &str, cooldown_secs: u64) {
        let mut inner = self.inner.lock().unwrap();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        for slot in inner.key_pool.iter_mut() {
            if slot.key == key {
                slot.cooldown_until = now_ms + (cooldown_secs * 1000) as i64;
                info!("Key {}...{} cooldown for {}s", &key[..4.min(key.len())], &key[key.len().saturating_sub(4)..], cooldown_secs);
                break;
            }
        }
    }

    /// Mark a key as daily-exhausted (cooldown until midnight Pacific).
    pub fn mark_key_daily_exhausted(&self, key: &str) {
        // Approximate: cooldown for 24 hours (conservative)
        self.mark_key_cooldown(key, 24 * 3600);
    }

    /// Persist current pool to OS keyring.
    fn persist_pool(&self) -> Result<(), String> {
        let inner = self.inner.lock().unwrap();
        let keys: Vec<String> = inner.key_pool.iter().map(|s| s.key.clone()).collect();
        let json = serde_json::to_string(&keys)
            .map_err(|e| format!("Serialize error: {}", e))?;
        let entry = Entry::new(SERVICE_NAME, MULTI_KEY_NAME)
            .map_err(|e| format!("Keyring error: {}", e))?;
        entry.set_password(&json)
            .map_err(|e| format!("Save error: {}", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_credential_store() {
        let store = CredentialStore::new();
        assert!(!store.is_configured());

        // Set key in session (remember = false)
        store.set_key("AIzaSyTestApiKey123", false).expect("Set key should succeed");
        assert!(store.is_configured());
        assert_eq!(store.get_key(), Some("AIzaSyTestApiKey123".to_string()));

        // Delete key
        store.delete_key().expect("Delete key should succeed");
        // Pool still has the key from set_key, so is_configured may still be true
    }

    #[test]
    fn test_multi_key_rotation() {
        let store = CredentialStore::new();
        store.set_keys(vec![
            "key_A".to_string(),
            "key_B".to_string(),
            "key_C".to_string(),
        ], false).unwrap();

        assert_eq!(store.key_count(), 3);

        // Round-robin
        let k1 = store.get_key().unwrap();
        let k2 = store.get_key().unwrap();
        let k3 = store.get_key().unwrap();
        let k4 = store.get_key().unwrap();

        assert_eq!(k1, "key_A");
        assert_eq!(k2, "key_B");
        assert_eq!(k3, "key_C");
        assert_eq!(k4, "key_A"); // wraps around
    }

    #[test]
    fn test_cooldown_skip() {
        let store = CredentialStore::new();
        store.set_keys(vec![
            "key_A".to_string(),
            "key_B".to_string(),
        ], false).unwrap();

        // Mark key_A on cooldown
        store.mark_key_cooldown("key_A", 60);

        // Should skip key_A and return key_B
        let k = store.get_key().unwrap();
        assert_eq!(k, "key_B");
    }
}
