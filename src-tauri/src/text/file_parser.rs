use std::fs;
use std::path::Path;

pub struct DocumentParser;

impl DocumentParser {
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<String, String> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(format!("File không tồn tại tại đường dẫn: {:?}", path));
        }

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "txt" | "md" | "json" | "srt" | "vtt" => {
                let content = fs::read_to_string(path)
                    .map_err(|e| format!("Lỗi đọc file văn bản (Yêu cầu mã hóa UTF-8): {}", e))?;
                // Strip UTF-8 BOM if present
                let clean_content = content.strip_prefix('\u{feff}').unwrap_or(&content);
                Ok(clean_content.to_string())
            }
            "docx" | "pdf" => {
                Err("Ứng dụng đã chuyển sang hỗ trợ chuẩn các file văn bản thuần (.txt, .md). Vui lòng lưu tài liệu dưới dạng file .txt hoặc .md UTF-8 để đảm bảo đọc chính xác 100%.".to_string())
            }
            _ => {
                let content = fs::read_to_string(path)
                    .map_err(|e| format!("Không hỗ trợ định dạng .{} hoặc lỗi đọc file: {}", ext, e))?;
                let clean_content = content.strip_prefix('\u{feff}').unwrap_or(&content);
                Ok(clean_content.to_string())
            }
        }
    }
}
