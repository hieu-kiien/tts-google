pub fn validate_project_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Tên dự án không được để trống".to_string());
    }
    if trimmed.chars().count() > 255 {
        return Err("Tên dự án vượt quá giới hạn 255 ký tự".to_string());
    }
    if trimmed.chars().any(|c| c.is_control() && c != '\n' && c != '\t') {
        return Err("Tên dự án chứa ký tự không hợp lệ".to_string());
    }
    Ok(())
}

pub fn validate_source_text(text: &str) -> Result<(), String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("Văn bản nguồn không được để trống".to_string());
    }
    if trimmed.chars().count() > 500_000 {
        return Err("Văn bản nguồn quá dài (tối đa 500.000 ký tự)".to_string());
    }
    Ok(())
}

pub fn validate_voice(voice: &str) -> Result<(), String> {
    let trimmed = voice.trim();
    if trimmed.is_empty() {
        return Err("Chưa chọn giọng đọc".to_string());
    }
    if trimmed.chars().count() > 64 {
        return Err("Tên giọng đọc không hợp lệ".to_string());
    }
    Ok(())
}

pub fn validate_preset(preset: &str) -> Result<(), String> {
    let trimmed = preset.trim();
    if trimmed.chars().count() > 64 {
        return Err("Tên preset không hợp lệ".to_string());
    }
    Ok(())
}

pub fn validate_chunk_mode(mode: Option<&str>) -> Result<(), String> {
    if let Some(m) = mode {
        match m {
            "auto" | "paragraph" | "sentence" | "clause" => Ok(()),
            _ => Err("Chế độ phân đoạn (chunk_mode) không hợp lệ".to_string()),
        }
    } else {
        Ok(())
    }
}

pub fn validate_api_key(key: &str) -> Result<(), String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("API Key không được để trống".to_string());
    }
    if trimmed.chars().count() > 256 {
        return Err("API Key quá dài (tối đa 256 ký tự)".to_string());
    }
    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("API Key chứa ký tự trắng hoặc ký tự điều khiển không hợp lệ".to_string());
    }
    Ok(())
}
