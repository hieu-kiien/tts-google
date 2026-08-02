use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpeakerRole {
    SpeakerA,
    SpeakerB,
    Narrator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueSegment {
    pub raw_line: String,
    pub speaker_label: String,
    pub role: SpeakerRole,
    pub spoken_text: String,
}

pub struct DialogueParser;

impl DialogueParser {
    pub fn parse_dialogue(text: &str) -> Vec<DialogueSegment> {
        let mut segments = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some((speaker, content)) = Self::split_speaker_prefix(trimmed) {
                let lower_spk = speaker.to_lowercase();
                let role = if lower_spk.contains("a")
                    || lower_spk.contains("người dẫn")
                    || lower_spk.contains("mc")
                    || lower_spk.contains("host")
                {
                    SpeakerRole::SpeakerA
                } else {
                    SpeakerRole::SpeakerB
                };

                segments.push(DialogueSegment {
                    raw_line: trimmed.to_string(),
                    speaker_label: speaker,
                    role,
                    spoken_text: content,
                });
            } else {
                segments.push(DialogueSegment {
                    raw_line: trimmed.to_string(),
                    speaker_label: "Dẫn chuyện".to_string(),
                    role: SpeakerRole::Narrator,
                    spoken_text: trimmed.to_string(),
                });
            }
        }

        segments
    }

    fn split_speaker_prefix(line: &str) -> Option<(String, String)> {
        // Pattern: "Speaker A: Text" or "Người dẫn: Text" or "Nhân vật - Text"
        let separators = [": ", "：", " - "];
        for sep in separators {
            if let Some(pos) = line.find(sep) {
                let speaker = line[..pos].trim().to_string();
                let content = line[pos + sep.len()..].trim().to_string();
                if !speaker.is_empty() && speaker.len() <= 30 && !content.is_empty() {
                    return Some((speaker, content));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dialogue() {
        let script = "Người dẫn: Xin chào quý vị.\nNhân vật: Cảm ơn bạn!";
        let segments = DialogueParser::parse_dialogue(script);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].speaker_label, "Người dẫn");
        assert_eq!(segments[1].speaker_label, "Nhân vật");
    }
}
