use rfd::FileDialog;
use serde::Serialize;

#[derive(Serialize)]
pub struct TextFileContent {
    pub file_path: String,
    pub content: String,
}

#[tauri::command]
pub fn pick_output_folder() -> Result<Option<String>, String> {
    let folder = FileDialog::new()
        .set_title("Chọn Thư Mục Lưu File Audio Master")
        .pick_folder();

    match folder {
        Some(path) => Ok(Some(path.to_string_lossy().to_string())),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn save_master_wav_dialog(default_filename: Option<String>) -> Result<Option<String>, String> {
    let default_name = default_filename.unwrap_or_else(|| "TTS_Master_Audio.wav".to_string());

    let file = FileDialog::new()
        .set_title("Lưu File Master Audio (WAV)")
        .set_file_name(&default_name)
        .add_filter("WAV Audio (*.wav)", &["wav"])
        .save_file();

    match file {
        Some(path) => Ok(Some(path.to_string_lossy().to_string())),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn save_srt_file_dialog(default_filename: Option<String>) -> Result<Option<String>, String> {
    let default_name = default_filename.unwrap_or_else(|| "TTS_Subtitle.srt".to_string());

    let file = FileDialog::new()
        .set_title("Lưu File Phụ Đề Subtitle (SRT)")
        .set_file_name(&default_name)
        .add_filter("SubRip Subtitle (*.srt)", &["srt"])
        .save_file();

    match file {
        Some(path) => Ok(Some(path.to_string_lossy().to_string())),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn save_vtt_file_dialog(default_filename: Option<String>) -> Result<Option<String>, String> {
    let default_name = default_filename.unwrap_or_else(|| "TTS_Subtitle.vtt".to_string());

    let file = FileDialog::new()
        .set_title("Lưu File Phụ Đề WebVTT (VTT)")
        .set_file_name(&default_name)
        .add_filter("WebVTT Subtitle (*.vtt)", &["vtt"])
        .save_file();

    match file {
        Some(path) => Ok(Some(path.to_string_lossy().to_string())),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn save_segment_audio_dialog(
    default_filename: Option<String>,
) -> Result<Option<String>, String> {
    let default_name = default_filename.unwrap_or_else(|| "Segment_Audio.wav".to_string());

    let file = FileDialog::new()
        .set_title("Lưu File Audio Segment Rời (WAV)")
        .set_file_name(&default_name)
        .add_filter("WAV Audio (*.wav)", &["wav"])
        .save_file();

    match file {
        Some(path) => Ok(Some(path.to_string_lossy().to_string())),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn read_text_file_dialog() -> Result<Option<TextFileContent>, String> {
    let file = FileDialog::new()
        .set_title("Chọn File Tài Liệu Văn Bản (TXT, MD)")
        .add_filter("Tài Liệu Văn Bản (*.txt, *.md)", &["txt", "md"])
        .pick_file();

    match file {
        Some(path) => {
            let content = crate::text::file_parser::DocumentParser::parse_file(&path)?;
            Ok(Some(TextFileContent {
                file_path: path.to_string_lossy().to_string(),
                content,
            }))
        }
        None => Ok(None),
    }
}
