use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use std::path::{Path, PathBuf};

pub fn validate_file_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("File name cannot be empty".to_string());
    }

    if name.contains('\0') {
        return Err("File name contains null byte".to_string());
    }

    if name.starts_with(r"\\?\") || name.starts_with(r"\\.\") {
        return Err("Device prefixes (\\\\?\\ or \\\\.\\) are not allowed".to_string());
    }

    if name.contains(':') {
        return Err("Alternate Data Streams (':') are not allowed".to_string());
    }

    if name.ends_with(' ') || name.ends_with('.') {
        return Err("File name cannot end with a space or dot".to_string());
    }

    // Windows invalid filename characters
    const INVALID_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    for c in name.chars() {
        if (c as u32) <= 0x1F || c == '\x7F' || INVALID_CHARS.contains(&c) {
            return Err(format!("File name contains invalid character: {:?}", c));
        }
    }

    // Windows reserved device names check (stem part before dot)
    let stem = name.split('.').next().unwrap_or(name).to_uppercase();
    let is_reserved = matches!(
        stem.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );

    if is_reserved {
        return Err(format!(
            "File name '{}' uses a reserved Windows device name",
            name
        ));
    }

    Ok(())
}

pub fn resolve_write_target(
    allowed_roots: &[&Path],
    target_path: &str,
    allowed_extensions: &[&str],
) -> Result<PathBuf, String> {
    let target = PathBuf::from(target_path);

    // 1. Extension validation (case-insensitive)
    let ext = target
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| "Target path must have a valid file extension".to_string())?
        .to_lowercase();

    let ext_allowed = allowed_extensions
        .iter()
        .any(|&a| a.to_lowercase().trim_start_matches('.') == ext);
    if !ext_allowed {
        return Err(format!(
            "File extension '.{}' is not allowed. Allowed extensions: {:?}",
            ext, allowed_extensions
        ));
    }

    // 2. Validate file name security rules
    if let Some(file_name_os) = target.file_name() {
        if let Some(file_name_str) = file_name_os.to_str() {
            validate_file_name(file_name_str)?;
        } else {
            return Err("Invalid non-UTF8 file name".to_string());
        }
    } else {
        return Err("Invalid target path: missing file name".to_string());
    }

    // 3. Disallow path traversal components
    for component in target.components() {
        if let std::path::Component::ParentDir = component {
            return Err("Path traversal ('..') is not permitted".to_string());
        }
    }

    // 4. Check parent path hierarchy against allowed roots
    let parent = target
        .parent()
        .ok_or_else(|| "Invalid target path: missing parent directory".to_string())?;

    let mut ancestor = parent;
    while !ancestor.exists() {
        if let Some(p) = ancestor.parent() {
            ancestor = p;
        } else {
            return Err("Invalid path hierarchy".to_string());
        }
    }

    let canonical_ancestor = ancestor
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize path ancestor: {}", e))?;

    let is_allowed = allowed_roots.iter().any(|root| {
        if let Ok(canonical_root) = root.canonicalize() {
            canonical_ancestor.starts_with(&canonical_root)
        } else {
            false
        }
    });

    if !is_allowed {
        return Err("Access denied: Target path parent is outside allowed directories".to_string());
    }

    // 5. Symlink security check if target file exists
    if target.exists() {
        let canonical_target = target
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize target file: {}", e))?;
        let is_allowed_target = allowed_roots.iter().any(|root| {
            if let Ok(canonical_root) = root.canonicalize() {
                canonical_target.starts_with(&canonical_root)
            } else {
                false
            }
        });
        if !is_allowed_target {
            return Err(
                "Access denied: Existing file resolves outside allowed directories".to_string(),
            );
        }
        Ok(canonical_target)
    } else {
        let file_name = target
            .file_name()
            .ok_or_else(|| "Invalid target path: missing file name".to_string())?;
        if parent.exists() {
            let canonical_parent = parent
                .canonicalize()
                .map_err(|e| format!("Failed to canonicalize parent: {}", e))?;
            Ok(canonical_parent.join(file_name))
        } else {
            Ok(parent.join(file_name))
        }
    }
}

/// Resolve export target path for user-selected destinations.
/// Applies all security checks EXCEPT allowed_roots directory restriction.
/// Use ONLY when the path originates from a system save dialog.
pub fn resolve_export_target(
    target_path: &str,
    allowed_extensions: &[&str],
) -> Result<PathBuf, String> {
    let target = PathBuf::from(target_path);

    // 1. Extension validation
    let ext = target
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| "Target path must have a valid file extension".to_string())?
        .to_lowercase();

    let ext_allowed = allowed_extensions
        .iter()
        .any(|&a| a.to_lowercase().trim_start_matches('.') == ext);
    if !ext_allowed {
        return Err(format!(
            "File extension '.{}' is not allowed. Allowed extensions: {:?}",
            ext, allowed_extensions
        ));
    }

    // 2. Validate file name security
    if let Some(file_name_os) = target.file_name() {
        if let Some(file_name_str) = file_name_os.to_str() {
            validate_file_name(file_name_str)?;
        } else {
            return Err("File name contains invalid UTF-8 characters".to_string());
        }
    } else {
        return Err("Target path must include a file name".to_string());
    }

    // 3. Path traversal check
    for component in target.components() {
        if let std::path::Component::ParentDir = component {
            return Err("Path traversal ('..') is not permitted".to_string());
        }
    }

    // 4. Resolve final path (no directory restriction for exports)
    let parent = target
        .parent()
        .ok_or_else(|| "Invalid target path: missing parent directory".to_string())?;

    if !parent.exists() {
        return Err(format!(
            "Export directory does not exist: {}",
            parent.display()
        ));
    }

    let file_name = target
        .file_name()
        .ok_or_else(|| "Invalid target path: missing file name".to_string())?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize export directory: {}", e))?;
    Ok(canonical_parent.join(file_name))
}

pub fn resolve_existing_read_target(
    allowed_roots: &[&Path],
    file_path: &str,
    allowed_extensions: &[&str],
) -> Result<PathBuf, String> {
    let target = PathBuf::from(file_path);

    if !target.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    // 1. Extension validation (case-insensitive)
    if !allowed_extensions.is_empty() {
        let ext = target
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| "File path must have a valid extension".to_string())?
            .to_lowercase();
        let ext_allowed = allowed_extensions
            .iter()
            .any(|&a| a.to_lowercase().trim_start_matches('.') == ext);
        if !ext_allowed {
            return Err(format!("File extension '.{}' is not allowed", ext));
        }
    }

    // 2. Canonicalization check
    let canonical_target = target
        .canonicalize()
        .map_err(|e| format!("Invalid target path: {}", e))?;

    let is_allowed = allowed_roots.iter().any(|root| {
        if let Ok(canonical_root) = root.canonicalize() {
            canonical_target.starts_with(&canonical_root)
        } else {
            false
        }
    });

    if !is_allowed {
        return Err("Access denied: File path is outside allowed directories".to_string());
    }

    Ok(canonical_target)
}

pub fn validate_base64_payload_size(
    base64_str: &str,
    max_decoded_bytes: usize,
) -> Result<Vec<u8>, String> {
    let clean_str = if let Some(idx) = base64_str.find(',') {
        &base64_str[idx + 1..]
    } else {
        base64_str
    };

    let max_base64_len = max_decoded_bytes.div_ceil(3) * 4 + 256;
    if clean_str.len() > max_base64_len {
        return Err(format!(
            "Payload size exceeds maximum allowed limit of {} MB",
            max_decoded_bytes / (1024 * 1024)
        ));
    }

    let bytes = BASE64_STANDARD
        .decode(clean_str)
        .map_err(|e| format!("Lỗi giải mã base64: {}", e))?;

    if bytes.len() > max_decoded_bytes {
        return Err(format!(
            "Decoded payload size ({} bytes) exceeds maximum limit of {} bytes",
            bytes.len(),
            max_decoded_bytes
        ));
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_file_name_cases() {
        assert!(validate_file_name("valid_audio.wav").is_ok());
        assert!(validate_file_name("dự_án_mẫu_123.mp3").is_ok());

        assert!(validate_file_name("CON.wav").is_err());
        assert!(validate_file_name("NUL.txt").is_err());
        assert!(validate_file_name("COM1.srt").is_err());
        assert!(validate_file_name("PRN").is_err());

        assert!(validate_file_name("audio.wav ").is_err());
        assert!(validate_file_name("audio.wav.").is_err());
        assert!(validate_file_name("audio.wav:stream").is_err());
        assert!(validate_file_name("audio\0.wav").is_err());
        assert!(validate_file_name(r"\\?\C:\file.wav").is_err());
    }

    #[test]
    fn test_resolve_write_target_traversal_rejection() {
        let temp_root = std::env::temp_dir().join("autotts_test_root");
        let _ = std::fs::create_dir_all(&temp_root);

        let allowed = vec![temp_root.as_path()];
        let bad_path = temp_root
            .join("../outside.wav")
            .to_string_lossy()
            .to_string();

        assert!(resolve_write_target(&allowed, &bad_path, &["wav"]).is_err());

        let _ = std::fs::remove_dir_all(&temp_root);
    }
}
