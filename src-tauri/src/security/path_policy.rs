use std::path::{Path, PathBuf};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

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

    // 2. Disallow path traversal components
    for component in target.components() {
        if let std::path::Component::ParentDir = component {
            return Err("Path traversal ('..') is not permitted".to_string());
        }
    }

    // 3. Check parent path hierarchy against allowed roots
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

    // 4. Symlink security check if target file exists
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
            return Err("Access denied: Existing file resolves outside allowed directories".to_string());
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

    let max_base64_len = ((max_decoded_bytes + 2) / 3) * 4 + 256;
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
