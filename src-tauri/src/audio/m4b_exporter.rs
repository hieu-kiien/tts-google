use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterMarker {
    pub chapter_number: usize,
    pub title: String,
    pub start_time_ms: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudiobookManifest {
    pub title: String,
    pub author: String,
    pub narrator: String,
    pub publisher: String,
    pub total_duration_ms: u64,
    pub chapters: Vec<ChapterMarker>,
}

pub struct M4bExporter;

impl M4bExporter {
    pub fn export_manifest<P: AsRef<Path>>(
        manifest: &AudiobookManifest,
        output_path: P,
    ) -> Result<String, String> {
        let json_data = serde_json::to_string_pretty(manifest)
            .map_err(|e| format!("Lỗi serialize JSON Manifest M4B: {}", e))?;

        let path_ref = output_path.as_ref();
        fs::write(path_ref, json_data.as_bytes())
            .map_err(|e| format!("Lỗi ghi file Audiobook Manifest: {}", e))?;

        info!("Exported Audiobook M4B Manifest to {:?}", path_ref);
        Ok(path_ref.to_string_lossy().to_string())
    }
}
