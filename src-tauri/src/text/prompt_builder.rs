use serde::{Deserialize, Serialize};

/// Director notes containing voice profile, performance style, and reading instructions for TTS.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DirectorNotes {
    pub voice_name: Option<String>,
    pub tone: Option<String>,
    pub speed: Option<String>,
    pub emotion: Option<String>,
    pub accent: Option<String>,
    pub custom_instructions: Option<String>,
}

impl DirectorNotes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.voice_name.is_none()
            && self.tone.is_none()
            && self.speed.is_none()
            && self.emotion.is_none()
            && self.accent.is_none()
            && self.custom_instructions.is_none()
    }
}

/// Simple style options for Tauri commands
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptStyleOptions {
    pub style_preset: String,
    pub pacing: String,
    pub pronunciation_notes: Option<String>,
}

pub fn build_tts_prompt(text: &str, opts: &PromptStyleOptions) -> String {
    let mut builder = PromptBuilder::new()
        .with_tone(&opts.style_preset)
        .with_speed(&opts.pacing);

    if let Some(ref notes) = opts.pronunciation_notes {
        builder = builder.with_custom_instructions(notes);
    }

    builder.build_prompt(text)
}

/// Prompt formatting style for isolating Director Notes from Transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PromptStyle {
    /// Section tags style: `[DIRECTOR NOTES]` ... `[TRANSCRIPT]`
    #[default]
    Tagged,
    /// XML tags style: `<director_notes>` ... `</director_notes>` `<transcript>` ... `</transcript>`
    Xml,
    /// Markdown header style: `# DIRECTOR NOTES` ... `# TRANSCRIPT` ...
    Markdown,
}

/// Prompt Builder for constructing TTS prompts with clear separation between Director Notes and Transcript.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PromptBuilder {
    notes: DirectorNotes,
    style: PromptStyle,
}

impl PromptBuilder {
    pub fn new() -> Self {
        Self {
            notes: DirectorNotes::default(),
            style: PromptStyle::Tagged,
        }
    }

    pub fn with_notes(mut self, notes: DirectorNotes) -> Self {
        self.notes = notes;
        self
    }

    pub fn with_voice_name(mut self, voice: impl Into<String>) -> Self {
        self.notes.voice_name = Some(voice.into());
        self
    }

    pub fn with_tone(mut self, tone: impl Into<String>) -> Self {
        self.notes.tone = Some(tone.into());
        self
    }

    pub fn with_speed(mut self, speed: impl Into<String>) -> Self {
        self.notes.speed = Some(speed.into());
        self
    }

    pub fn with_emotion(mut self, emotion: impl Into<String>) -> Self {
        self.notes.emotion = Some(emotion.into());
        self
    }

    pub fn with_accent(mut self, accent: impl Into<String>) -> Self {
        self.notes.accent = Some(accent.into());
        self
    }

    pub fn with_custom_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.notes.custom_instructions = Some(instructions.into());
        self
    }

    pub fn with_style(mut self, style: PromptStyle) -> Self {
        self.style = style;
        self
    }

    pub fn director_notes(&self) -> &DirectorNotes {
        &self.notes
    }

    pub fn style(&self) -> PromptStyle {
        self.style
    }

    /// Builds a full TTS prompt with Director Notes separated from the Transcript.
    pub fn build_prompt(&self, transcript: &str) -> String {
        self.build_chunk_prompt(transcript, None)
    }

    /// Builds a TTS prompt for a specific chunk, including chunk index metadata if provided.
    pub fn build_chunk_prompt(
        &self,
        transcript: &str,
        chunk_info: Option<(usize, usize)>,
    ) -> String {
        let clean_transcript = transcript.trim();

        match self.style {
            PromptStyle::Tagged => self.format_tagged(clean_transcript, chunk_info),
            PromptStyle::Xml => self.format_xml(clean_transcript, chunk_info),
            PromptStyle::Markdown => self.format_markdown(clean_transcript, chunk_info),
        }
    }

    fn format_tagged(&self, transcript: &str, chunk_info: Option<(usize, usize)>) -> String {
        let mut out = String::new();

        out.push_str("[DIRECTOR NOTES]\n");

        if let Some((idx, total)) = chunk_info {
            out.push_str(&format!("Chunk: {}/{}\n", idx + 1, total));
        }

        if let Some(ref voice) = self.notes.voice_name {
            out.push_str(&format!("Voice: {}\n", voice));
        }
        if let Some(ref tone) = self.notes.tone {
            out.push_str(&format!("Tone: {}\n", tone));
        }
        if let Some(ref speed) = self.notes.speed {
            out.push_str(&format!("Speed: {}\n", speed));
        }
        if let Some(ref accent) = self.notes.accent {
            out.push_str(&format!("Accent: {}\n", accent));
        }
        if let Some(ref emotion) = self.notes.emotion {
            out.push_str(&format!("Emotion: {}\n", emotion));
        }
        if let Some(ref inst) = self.notes.custom_instructions {
            out.push_str(&format!("Instructions: {}\n", inst));
        }

        if self.notes.is_empty() && chunk_info.is_none() {
            out.push_str("Standard VietTTS reading mode. Preserve all punctuation and pauses.\n");
        }

        out.push_str("\n[TRANSCRIPT]\n");
        out.push_str(transcript);

        out
    }

    fn format_xml(&self, transcript: &str, chunk_info: Option<(usize, usize)>) -> String {
        let mut out = String::new();

        out.push_str("<director_notes>\n");

        if let Some((idx, total)) = chunk_info {
            out.push_str(&format!("Chunk: {}/{}\n", idx + 1, total));
        }

        if let Some(ref voice) = self.notes.voice_name {
            out.push_str(&format!("Voice: {}\n", voice));
        }
        if let Some(ref tone) = self.notes.tone {
            out.push_str(&format!("Tone: {}\n", tone));
        }
        if let Some(ref speed) = self.notes.speed {
            out.push_str(&format!("Speed: {}\n", speed));
        }
        if let Some(ref accent) = self.notes.accent {
            out.push_str(&format!("Accent: {}\n", accent));
        }
        if let Some(ref emotion) = self.notes.emotion {
            out.push_str(&format!("Emotion: {}\n", emotion));
        }
        if let Some(ref inst) = self.notes.custom_instructions {
            out.push_str(&format!("Instructions: {}\n", inst));
        }

        if self.notes.is_empty() && chunk_info.is_none() {
            out.push_str("Standard VietTTS reading mode.\n");
        }

        out.push_str("</director_notes>\n\n<transcript>\n");
        out.push_str(transcript);
        out.push_str("\n</transcript>");

        out
    }

    fn format_markdown(&self, transcript: &str, chunk_info: Option<(usize, usize)>) -> String {
        let mut out = String::new();

        out.push_str("# Director Notes\n");

        if let Some((idx, total)) = chunk_info {
            out.push_str(&format!("- **Chunk**: {}/{}\n", idx + 1, total));
        }

        if let Some(ref voice) = self.notes.voice_name {
            out.push_str(&format!("- **Voice**: {}\n", voice));
        }
        if let Some(ref tone) = self.notes.tone {
            out.push_str(&format!("- **Tone**: {}\n", tone));
        }
        if let Some(ref speed) = self.notes.speed {
            out.push_str(&format!("- **Speed**: {}\n", speed));
        }
        if let Some(ref accent) = self.notes.accent {
            out.push_str(&format!("- **Accent**: {}\n", accent));
        }
        if let Some(ref emotion) = self.notes.emotion {
            out.push_str(&format!("- **Emotion**: {}\n", emotion));
        }
        if let Some(ref inst) = self.notes.custom_instructions {
            out.push_str(&format!("- **Instructions**: {}\n", inst));
        }

        if self.notes.is_empty() && chunk_info.is_none() {
            out.push_str("- Standard VietTTS reading mode.\n");
        }

        out.push_str("\n# Transcript\n");
        out.push_str(transcript);

        out
    }
}

use std::sync::LazyLock;

static RE_SPEAK: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)</?speak>").unwrap());
static RE_BREAK: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r#"(?i)<break\s+time=["']([^"']+)["']\s*/?>"#).unwrap());
static RE_EMP: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(?i)<emphasis[^>]*>(.*?)</emphasis>").unwrap());
static RE_PROSODY: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r#"(?i)<prosody\s+([^>]+)>(.*?)</prosody>"#).unwrap());

/// Parse SSML markup into Director Notes and clean Transcript text for Gemini TTS.
pub fn parse_ssml_to_prompt(ssml_text: &str) -> (DirectorNotes, String) {
    let mut notes = DirectorNotes::default();
    let mut clean_text = ssml_text.to_string();

    // Strip <speak> outer wrapper
    clean_text = RE_SPEAK.replace_all(&clean_text, "").to_string();

    // Process <break time="..."/> -> [Pause ...]
    clean_text = RE_BREAK.replace_all(&clean_text, " [Pause $1] ").to_string();

    // Process <emphasis level="...">text</emphasis> -> [Emphasize: text]
    clean_text = RE_EMP.replace_all(&clean_text, " [Emphasize: $1] ").to_string();

    // Process <prosody rate="..." pitch="...">text</prosody>
    for cap in RE_PROSODY.captures_iter(ssml_text) {
        let attrs = &cap[1];
        if attrs.contains("slow") {
            notes.speed = Some("slow".to_string());
        } else if attrs.contains("fast") {
            notes.speed = Some("fast".to_string());
        }
    }
    clean_text = RE_PROSODY.replace_all(&clean_text, "$2").to_string();

    // Trim whitespace
    clean_text = clean_text.trim().to_string();
    (notes, clean_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ssml_tags() {
        let ssml = r#"<speak>Xin chào <break time="500ms"/> các bạn <emphasis level="strong">tiếng Việt</emphasis> rất đẹp.</speak>"#;
        let (_notes, transcript) = parse_ssml_to_prompt(ssml);

        assert!(transcript.contains("[Pause 500ms]"));
        assert!(transcript.contains("[Emphasize: tiếng Việt]"));
        assert!(!transcript.contains("<speak>"));
    }

    #[test]
    fn test_prompt_builder_tagged_style() {
        let builder = PromptBuilder::new()
            .with_voice_name("Kore")
            .with_tone("Truyền cảm")
            .with_speed("Vừa phải")
            .with_accent("Miền Nam")
            .with_emotion("Ấm áp")
            .with_custom_instructions("Ngắt nghỉ rõ ràng")
            .with_style(PromptStyle::Tagged);

        let transcript = "Xin chào quý vị khán giả.";
        let prompt = builder.build_prompt(transcript);

        assert!(prompt.contains("[DIRECTOR NOTES]"));
        assert!(prompt.contains("Voice: Kore"));
        assert!(prompt.contains("Tone: Truyền cảm"));
        assert!(prompt.contains("Speed: Vừa phải"));
        assert!(prompt.contains("Accent: Miền Nam"));
        assert!(prompt.contains("Emotion: Ấm áp"));
        assert!(prompt.contains("Instructions: Ngắt nghỉ rõ ràng"));
        assert!(prompt.contains("[TRANSCRIPT]"));
        assert!(prompt.contains("Xin chào quý vị khán giả."));
    }

    #[test]
    fn test_prompt_builder_xml_style() {
        let builder = PromptBuilder::new()
            .with_voice_name("Puck")
            .with_tone("Hào hứng")
            .with_style(PromptStyle::Xml);

        let transcript = "Chào mừng các bạn quay trở lại!";
        let prompt = builder.build_prompt(transcript);

        assert!(prompt.contains("<director_notes>"));
        assert!(prompt.contains("Voice: Puck"));
        assert!(prompt.contains("Tone: Hào hứng"));
        assert!(prompt.contains("</director_notes>"));
        assert!(prompt.contains("<transcript>"));
        assert!(prompt.contains("Chào mừng các bạn quay trở lại!"));
        assert!(prompt.contains("</transcript>"));
    }

    #[test]
    fn test_prompt_builder_markdown_style() {
        let builder = PromptBuilder::new()
            .with_tone("Trang trọng")
            .with_style(PromptStyle::Markdown);

        let transcript = "Kính chào quý đại biểu.";
        let prompt = builder.build_prompt(transcript);

        assert!(prompt.contains("# Director Notes"));
        assert!(prompt.contains("- **Tone**: Trang trọng"));
        assert!(prompt.contains("# Transcript"));
        assert!(prompt.contains("Kính chào quý đại biểu."));
    }

    #[test]
    fn test_prompt_builder_chunk_metadata() {
        let builder = PromptBuilder::new()
            .with_voice_name("Fenrir")
            .with_style(PromptStyle::Tagged);

        let transcript = "Đoạn văn bản của phần thứ nhất.";
        let prompt = builder.build_chunk_prompt(transcript, Some((0, 3)));

        assert!(prompt.contains("Chunk: 1/3"));
        assert!(prompt.contains("Voice: Fenrir"));
        assert!(prompt.contains("Đoạn văn bản của phần thứ nhất."));
    }

    #[test]
    fn test_empty_notes_default_fallback() {
        let builder = PromptBuilder::new();
        let prompt = builder.build_prompt("Nội dung đơn giản.");

        assert!(prompt.contains("[DIRECTOR NOTES]"));
        assert!(prompt.contains("Standard VietTTS reading mode"));
        assert!(prompt.contains("[TRANSCRIPT]"));
        assert!(prompt.contains("Nội dung đơn giản."));
    }
}
