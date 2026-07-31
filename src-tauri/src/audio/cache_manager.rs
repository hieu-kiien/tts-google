use sha2::{Sha256, Digest};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub struct AudioCacheManager {
    cache_dir: PathBuf,
}

impl AudioCacheManager {
    pub fn new<P: AsRef<Path>>(cache_dir: P) -> Self {
        let dir = cache_dir.as_ref().to_path_buf();
        if !dir.exists() {
            let _ = fs::create_dir_all(&dir);
        }
        Self { cache_dir: dir }
    }

    pub fn compute_key(text: &str, voice: &str, speed: f32, pitch: f32, format: &str) -> String {
        let raw = format!("{}|{}|{:.2}|{:.2}|{}", text.trim(), voice.trim(), speed, pitch, format.trim());
        let mut hasher = Sha256::new();
        hasher.update(raw.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn get_cached_audio(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.cache_dir.join(format!("cache_{}.wav", key));
        if path.exists() {
            match fs::read(&path) {
                Ok(bytes) => {
                    info!("Audio Cache HIT for key [{}] ({}) bytes", &key[..8], bytes.len());
                    Some(bytes)
                }
                Err(e) => {
                    warn!("Failed to read cached audio file: {}", e);
                    None
                }
            }
        } else {
            None
        }
    }

    pub fn save_cache(&self, key: &str, audio_bytes: &[u8]) -> Result<PathBuf, std::io::Error> {
        let path = self.cache_dir.join(format!("cache_{}.wav", key));
        fs::write(&path, audio_bytes)?;
        info!("Saved audio cache entry [{}] at {:?}", &key[..8], path);
        Ok(path)
    }

    pub fn clear_cache(&self) -> Result<usize, std::io::Error> {
        let mut count = 0;
        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    fs::remove_file(entry.path())?;
                    count += 1;
                }
            }
        }
        info!("Cleared {} audio cache files", count);
        Ok(count)
    }

    pub fn get_cache_size_bytes(&self) -> u64 {
        let mut total = 0u64;
        if let Ok(entries) = fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    total += meta.len();
                }
            }
        }
        total
    }
}
