use rusqlite::params;
use sha2::{Digest, Sha256};
use tracing::info;

use crate::storage::db::DatabaseManager;

/// Content-addressed audio cache.
/// Same text + model + voice = same cache key = skip API call.
pub struct AudioCache;

impl AudioCache {
    /// Compute cache key: SHA256(model + voice + normalized_text)
    pub fn compute_cache_key(model: &str, voice: &str, text: &str) -> String {
        let normalized = text.trim().to_lowercase();
        let mut hasher = Sha256::new();
        hasher.update(b"v1:");
        hasher.update(model.as_bytes());
        hasher.update(b":");
        hasher.update(voice.as_bytes());
        hasher.update(b":");
        hasher.update(normalized.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Look up cached audio file path by cache key.
    /// Returns Some(file_path) if cached and file still exists on disk.
    pub fn lookup(db: &DatabaseManager, cache_key: &str) -> Option<String> {
        let result = db.with_conn(|conn| {
            let path: Option<String> = conn
                .query_row(
                    "SELECT file_path FROM audio_cache WHERE cache_key = ?1",
                    params![cache_key],
                    |row| row.get(0),
                )
                .optional()?;
            Ok(path)
        });

        match result {
            Ok(Some(path)) => {
                // Verify file still exists
                if std::path::Path::new(&path).exists() {
                    // Update last_accessed_at
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64;
                    let _ = db.with_conn_mut(|conn| {
                        conn.execute(
                            "UPDATE audio_cache SET last_accessed_at = ?1 WHERE cache_key = ?2",
                            params![now_ms, cache_key],
                        )?;
                        Ok(())
                    });
                    info!("Audio cache HIT: {}", &cache_key[..16]);
                    Some(path)
                } else {
                    // File deleted from disk, remove stale cache entry
                    let _ = db.with_conn_mut(|conn| {
                        conn.execute(
                            "DELETE FROM audio_cache WHERE cache_key = ?1",
                            params![cache_key],
                        )?;
                        Ok(())
                    });
                    info!("Audio cache STALE (file missing): {}", &cache_key[..16]);
                    None
                }
            }
            _ => None,
        }
    }

    /// Store a cache entry after successful audio generation.
    pub fn store(
        db: &DatabaseManager,
        cache_key: &str,
        model: &str,
        voice: &str,
        file_path: &str,
        duration_ms: u64,
        byte_size: u64,
    ) -> Result<(), String> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        db.with_conn_mut(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO audio_cache (
                    cache_key, model, voice, file_path,
                    duration_ms, byte_size, created_at, last_accessed_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    cache_key,
                    model,
                    voice,
                    file_path,
                    duration_ms,
                    byte_size,
                    now_ms
                ],
            )?;
            Ok(())
        })?;
        info!("Audio cache STORE: {} -> {}", &cache_key[..16], file_path);
        Ok(())
    }

    /// Get total cache size in bytes.
    pub fn total_size(db: &DatabaseManager) -> u64 {
        db.with_conn(|conn| {
            let size: i64 = conn.query_row(
                "SELECT COALESCE(SUM(byte_size), 0) FROM audio_cache",
                [],
                |row| row.get(0),
            )?;
            Ok(size as u64)
        })
        .unwrap_or(0)
    }

    /// Get number of cached entries.
    pub fn entry_count(db: &DatabaseManager) -> u64 {
        db.with_conn(|conn| {
            let count: i64 =
                conn.query_row("SELECT COUNT(*) FROM audio_cache", [], |row| row.get(0))?;
            Ok(count as u64)
        })
        .unwrap_or(0)
    }

    /// Evict least recently accessed entries until total cache is under max_bytes.
    pub fn evict_lru(db: &DatabaseManager, max_bytes: u64) -> Result<usize, String> {
        let current = Self::total_size(db);
        if current <= max_bytes {
            return Ok(0);
        }

        let entries = db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT cache_key, file_path, byte_size FROM audio_cache ORDER BY last_accessed_at ASC"
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)? as u64,
                ))
            })?.filter_map(|r| r.ok()).collect::<Vec<_>>();
            Ok(rows)
        })?;

        let mut freed: u64 = 0;
        let target_free = current - max_bytes;
        let mut evicted = 0;

        for (key, path, size) in entries {
            if freed >= target_free {
                break;
            }
            let _ = std::fs::remove_file(&path);
            let _ = db.with_conn_mut(|conn| {
                conn.execute("DELETE FROM audio_cache WHERE cache_key = ?1", params![key])?;
                Ok(())
            });
            freed += size;
            evicted += 1;
        }

        info!(
            "Audio cache LRU eviction: removed {} entries, freed {} bytes",
            evicted, freed
        );
        Ok(evicted)
    }
}

use rusqlite::OptionalExtension;
