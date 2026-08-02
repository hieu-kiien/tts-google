use serde::{Deserialize, Serialize};

/// Configuration options for the Vietnamese text chunker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkConfig {
    /// Minimum target audio duration in seconds (default: 30.0s)
    pub min_duration_secs: f32,
    /// Maximum target audio duration in seconds (default: 90.0s)
    pub max_duration_secs: f32,
    /// Preferred target audio duration in seconds (default: 60.0s)
    pub target_duration_secs: f32,
    /// Average character reading speed per second for Vietnamese (default: 15.0 chars/sec)
    pub chars_per_second: f32,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            min_duration_secs: 12.0,
            max_duration_secs: 35.0,
            target_duration_secs: 24.0,
            chars_per_second: 15.0,
        }
    }
}

impl ChunkConfig {
    pub fn new(min_secs: f32, max_secs: f32, target_secs: f32, chars_per_sec: f32) -> Self {
        Self {
            min_duration_secs: min_secs,
            max_duration_secs: max_secs,
            target_duration_secs: target_secs,
            chars_per_second: chars_per_sec,
        }
    }

    pub fn min_chars(&self) -> usize {
        (self.min_duration_secs * self.chars_per_second).round() as usize
    }

    pub fn max_chars(&self) -> usize {
        (self.max_duration_secs * self.chars_per_second).round() as usize
    }

    pub fn target_chars(&self) -> usize {
        (self.target_duration_secs * self.chars_per_second).round() as usize
    }

    pub fn estimate_duration(&self, text: &str) -> f32 {
        let count = text.chars().count();
        if count == 0 {
            0.0
        } else {
            count as f32 / self.chars_per_second
        }
    }
}

/// Represents a produced audio text chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextChunk {
    pub index: usize,
    pub position: i32,
    pub text: String,
    pub word_count: usize,
    pub char_count: usize,
    pub estimated_duration_secs: f32,
    pub estimated_duration_ms: i64,
}

pub fn chunk_vietnamese_text(text: &str, _options: Option<()>) -> Vec<TextChunk> {
    VietnameseChunker::with_default_config().chunk_text(text)
}

pub fn chunk_vietnamese_text_by_mode(text: &str, mode: &str) -> Vec<TextChunk> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    match mode {
        "line" => {
            let lines: Vec<&str> = trimmed
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect();

            lines
                .into_iter()
                .enumerate()
                .map(|(idx, l_text)| {
                    let chars = l_text.chars().count();
                    let words = l_text.split_whitespace().count();
                    let duration_secs = chars as f32 / 15.0;
                    TextChunk {
                        index: idx,
                        position: (idx + 1) as i32,
                        text: l_text.to_string(),
                        word_count: words,
                        char_count: chars,
                        estimated_duration_secs: duration_secs,
                        estimated_duration_ms: (duration_secs * 1000.0) as i64,
                    }
                })
                .collect()
        }
        "paragraph" => {
            let paragraphs: Vec<&str> = trimmed
                .split("\n\n")
                .map(|p| p.trim())
                .filter(|p| !p.is_empty())
                .collect();

            paragraphs
                .into_iter()
                .enumerate()
                .map(|(idx, p_text)| {
                    let chars = p_text.chars().count();
                    let words = p_text.split_whitespace().count();
                    let duration_secs = chars as f32 / 15.0;
                    TextChunk {
                        index: idx,
                        position: (idx + 1) as i32,
                        text: p_text.to_string(),
                        word_count: words,
                        char_count: chars,
                        estimated_duration_secs: duration_secs,
                        estimated_duration_ms: (duration_secs * 1000.0) as i64,
                    }
                })
                .collect()
        }
        "sentence" => {
            let mut sentences = Vec::new();
            let mut current = String::new();
            for ch in trimmed.chars() {
                current.push(ch);
                if ch == '.' || ch == '!' || ch == '?' || ch == '\n' {
                    let t = current.trim();
                    if !t.is_empty() {
                        sentences.push(t.to_string());
                    }
                    current.clear();
                }
            }
            let t = current.trim();
            if !t.is_empty() {
                sentences.push(t.to_string());
            }

            sentences
                .into_iter()
                .enumerate()
                .map(|(idx, s_text)| {
                    let chars = s_text.chars().count();
                    let words = s_text.split_whitespace().count();
                    let duration_secs = chars as f32 / 15.0;
                    TextChunk {
                        index: idx,
                        position: (idx + 1) as i32,
                        text: s_text,
                        word_count: words,
                        char_count: chars,
                        estimated_duration_secs: duration_secs,
                        estimated_duration_ms: (duration_secs * 1000.0) as i64,
                    }
                })
                .collect()
        }
        "chars_500" => {
            let mut chunks = Vec::new();
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            let mut current = String::new();
            let mut idx = 0;

            for w in words {
                if !current.is_empty() && current.chars().count() + w.chars().count() + 1 > 500 {
                    let chars = current.chars().count();
                    let w_count = current.split_whitespace().count();
                    let duration_secs = chars as f32 / 15.0;
                    chunks.push(TextChunk {
                        index: idx,
                        position: (idx + 1) as i32,
                        text: current.clone(),
                        word_count: w_count,
                        char_count: chars,
                        estimated_duration_secs: duration_secs,
                        estimated_duration_ms: (duration_secs * 1000.0) as i64,
                    });
                    idx += 1;
                    current.clear();
                }
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(w);
            }

            if !current.is_empty() {
                let chars = current.chars().count();
                let w_count = current.split_whitespace().count();
                let duration_secs = chars as f32 / 15.0;
                chunks.push(TextChunk {
                    index: idx,
                    position: (idx + 1) as i32,
                    text: current,
                    word_count: w_count,
                    char_count: chars,
                    estimated_duration_secs: duration_secs,
                    estimated_duration_ms: (duration_secs * 1000.0) as i64,
                });
            }

            chunks
        }
        _ => VietnameseChunker::with_default_config().chunk_text(text),
    }
}

/// Hierarchical Vietnamese text chunker.
/// Hierarchy strategy: Heading -> Paragraph -> Sentence -> Clause -> Word
#[derive(Debug, Clone)]
pub struct VietnameseChunker {
    config: ChunkConfig,
}

impl VietnameseChunker {
    pub fn new(config: ChunkConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(ChunkConfig::default())
    }

    pub fn config(&self) -> &ChunkConfig {
        &self.config
    }

    /// Primary chunking entry point.
    /// Splits input Vietnamese text into chunks targeting 30-90s audio duration.
    /// Preserves punctuation, structural hierarchy, and text order.
    pub fn chunk_text(&self, input: &str) -> Vec<TextChunk> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        let max_chars = self.config.max_chars();
        let target_chars = self.config.target_chars();

        // 1. Hierarchical decomposition into atomic blocks/units
        let units = self.decompose_text(input, max_chars);

        if units.is_empty() {
            return Vec::new();
        }

        // 2. Assemble units into balanced chunks (30s - 90s target)
        let mut chunks = Vec::new();
        let mut current_buf = String::new();
        let mut current_char_count = 0;

        for unit in units {
            let unit_char_count = unit.chars().count();

            if current_buf.is_empty() {
                current_buf.push_str(&unit);
                current_char_count = unit_char_count;
                continue;
            }

            // Check if adding unit exceeds max_chars limit
            let potential_len = current_char_count + unit_char_count;

            if potential_len <= max_chars {
                // If adding unit keeps us within max limit, aggregate
                current_buf.push_str(&unit);
                current_char_count = potential_len;

                // Optional soft threshold: if we reached target_chars and unit was paragraph boundary
                if current_char_count >= target_chars
                    && (unit.ends_with("\n\n") || unit.ends_with("\n"))
                {
                    Self::finalize_chunk(
                        &mut chunks,
                        &mut current_buf,
                        &mut current_char_count,
                        &self.config,
                    );
                }
            } else {
                // Exceeds max_chars, finalize current chunk and start new one
                Self::finalize_chunk(
                    &mut chunks,
                    &mut current_buf,
                    &mut current_char_count,
                    &self.config,
                );
                current_buf.push_str(&unit);
                current_char_count = unit_char_count;
            }
        }

        if !current_buf.is_empty() {
            Self::finalize_chunk(
                &mut chunks,
                &mut current_buf,
                &mut current_char_count,
                &self.config,
            );
        }

        // Renumber index strictly
        for (i, chunk) in chunks.iter_mut().enumerate() {
            chunk.index = i;
        }

        chunks
    }

    fn finalize_chunk(
        chunks: &mut Vec<TextChunk>,
        buf: &mut String,
        char_count: &mut usize,
        config: &ChunkConfig,
    ) {
        let chunk_text = buf.trim().to_string();
        if !chunk_text.is_empty() {
            let count_chars = chunk_text.chars().count();
            let count_words = chunk_text.split_whitespace().count();
            let est_dur = config.estimate_duration(&chunk_text);

            let pos = (chunks.len() + 1) as i32;
            let est_ms = (est_dur * 1000.0) as i64;
            chunks.push(TextChunk {
                index: chunks.len(),
                position: pos,
                text: chunk_text,
                word_count: count_words,
                char_count: count_chars,
                estimated_duration_secs: (est_dur * 10.0).round() / 10.0,
                estimated_duration_ms: est_ms,
            });
        }
        buf.clear();
        *char_count = 0;
    }

    /// Decompose text top-down: Heading -> Paragraph -> Sentence -> Clause -> Word
    fn decompose_text(&self, input: &str, max_chars: usize) -> Vec<String> {
        let blocks = self.split_headings_and_paragraphs(input);
        let mut units = Vec::new();

        for block in blocks {
            if block.chars().count() <= max_chars {
                units.push(block);
            } else {
                // Level 2: Paragraph -> Sentences
                let sentences = self.split_sentences(&block);
                for sentence in sentences {
                    if sentence.chars().count() <= max_chars {
                        units.push(sentence);
                    } else {
                        // Level 3: Sentence -> Clauses
                        let clauses = self.split_clauses(&sentence);
                        for clause in clauses {
                            if clause.chars().count() <= max_chars {
                                units.push(clause);
                            } else {
                                // Level 4: Clause -> Words
                                let word_units = self.split_words(&clause, max_chars);
                                units.extend(word_units);
                            }
                        }
                    }
                }
            }
        }

        units
    }

    /// Level 1: Split input text into Headings and Paragraph blocks while preserving punctuation and separators.
    fn split_headings_and_paragraphs(&self, input: &str) -> Vec<String> {
        let mut blocks = Vec::new();
        let lines: Vec<&str> = input.split('\n').collect();

        let mut current_block = String::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let is_heading = self.is_heading_line(line);

            if is_heading {
                // If we have an accumulated paragraph block, save it first
                if !current_block.is_empty() {
                    blocks.push(current_block.clone());
                    current_block.clear();
                }
                // Heading gets its own block with trailing newline
                let mut heading_block = line.to_string();
                if i + 1 < lines.len() {
                    heading_block.push('\n');
                }
                blocks.push(heading_block);
            } else if line.trim().is_empty() {
                // Blank line represents paragraph boundary
                if !current_block.is_empty() {
                    current_block.push('\n');
                    blocks.push(current_block.clone());
                    current_block.clear();
                }
            } else {
                if !current_block.is_empty() {
                    current_block.push('\n');
                }
                current_block.push_str(line);
            }
            i += 1;
        }

        if !current_block.is_empty() {
            blocks.push(current_block);
        }

        blocks
    }

    fn is_heading_line(&self, line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.starts_with("# ")
            || trimmed.starts_with("## ")
            || trimmed.starts_with("### ")
            || trimmed.starts_with("#### ")
            || trimmed.starts_with("##### ")
            || trimmed.starts_with("###### ")
        {
            return true;
        }

        // Vietnamese section heading prefixes like "Chương 1", "Phần I", "Bài 2"
        let lower = trimmed.to_lowercase();
        if (lower.starts_with("chương ") || lower.starts_with("phần ") || lower.starts_with("bài "))
            && trimmed.len() < 80
        {
            return true;
        }

        false
    }

    /// Level 2: Split paragraph into sentences preserving punctuation (. ! ? ... … \n)
    fn split_sentences(&self, text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut start_byte = 0;
        let mut iter = text.char_indices().peekable();

        while let Some((i, ch)) = iter.next() {
            if is_sentence_ending(ch) {
                let mut end_byte = i + ch.len_utf8();

                while let Some(&(next_i, next_ch)) = iter.peek() {
                    if is_sentence_ending(next_ch) {
                        end_byte = next_i + next_ch.len_utf8();
                        iter.next();
                    } else {
                        break;
                    }
                }

                if self.is_abbreviation_or_number(text, start_byte, end_byte) {
                    continue;
                }

                while let Some(&(next_i, next_ch)) = iter.peek() {
                    if next_ch == '"' || next_ch == '”' || next_ch == ')' || next_ch == ']' {
                        end_byte = next_i + next_ch.len_utf8();
                        iter.next();
                    } else {
                        break;
                    }
                }

                while let Some(&(next_i, next_ch)) = iter.peek() {
                    if next_ch == ' ' || next_ch == '\t' {
                        end_byte = next_i + next_ch.len_utf8();
                        iter.next();
                    } else {
                        break;
                    }
                }

                let sentence = &text[start_byte..end_byte];
                if !sentence.is_empty() {
                    sentences.push(sentence.to_string());
                }
                start_byte = end_byte;
            }
        }

        if start_byte < text.len() {
            let remaining = &text[start_byte..];
            if !remaining.is_empty() {
                sentences.push(remaining.to_string());
            }
        }

        sentences
    }

    fn is_abbreviation_or_number(
        &self,
        full_text: &str,
        start_byte: usize,
        end_byte: usize,
    ) -> bool {
        let dot_char_opt = full_text[..end_byte].chars().next_back();
        if dot_char_opt.is_none() {
            return false;
        }
        let dot_char = dot_char_opt.unwrap();
        let dot_pos = end_byte - dot_char.len_utf8();

        if dot_pos == 0 || dot_pos >= full_text.len() {
            return false;
        }

        let char_before = full_text[..dot_pos].chars().next_back();
        let char_after = full_text[end_byte..].chars().next();

        if let (Some(cb), Some(ca)) = (char_before, char_after) {
            // Decimal numbers like 1.5, 3.14
            if cb.is_ascii_digit() && ca.is_ascii_digit() {
                return true;
            }

            let start_bounded = start_byte.min(end_byte);
            let sub_text = &full_text[start_bounded..end_byte];

            // Domain names, URLs, emails, filenames without space after dot (e.g. example.com, v1.0, user.name@domain.com)
            if !cb.is_whitespace() && !ca.is_whitespace() {
                let lower = sub_text.to_lowercase();
                if lower.contains("http://")
                    || lower.contains("https://")
                    || lower.contains("www.")
                    || lower.contains("@")
                {
                    return true;
                }
                if ca.is_ascii_lowercase() || ca.is_ascii_digit() {
                    return true;
                }
            }

            let abbrevs = [
                "TP.", "Tp.", "ThS.", "TS.", "GS.", "PGS.", "BS.", "v.v.", "e.g.", "i.e.", "St.",
                "Mr.", "Mrs.", "Dr.", "NXB.", "P.", "Q.", "Th.S", "STT.", "SĐT.", "Co.", "Ltd.",
                "Inc.", "Corp.",
            ];

            for abbrev in &abbrevs {
                if sub_text.ends_with(abbrev) {
                    return true;
                }
            }
        }

        false
    }

    /// Level 3: Split sentence into clauses by punctuation (, ; : — – -)
    fn split_clauses(&self, text: &str) -> Vec<String> {
        let mut clauses = Vec::new();
        let mut start_byte = 0;
        let mut iter = text.char_indices().peekable();

        while let Some((i, ch)) = iter.next() {
            if is_clause_boundary(ch) {
                // Do not treat colon in URL scheme (e.g. http://, https://) as clause boundary
                if ch == ':' {
                    let mut clone_iter = iter.clone();
                    if let Some((_, '/')) = clone_iter.next() {
                        if let Some((_, '/')) = clone_iter.next() {
                            continue;
                        }
                    }
                }

                let mut end_byte = i + ch.len_utf8();

                while let Some(&(next_i, next_ch)) = iter.peek() {
                    if next_ch == ' ' || next_ch == '\t' {
                        end_byte = next_i + next_ch.len_utf8();
                        iter.next();
                    } else {
                        break;
                    }
                }

                let clause = &text[start_byte..end_byte];
                if !clause.is_empty() {
                    clauses.push(clause.to_string());
                }
                start_byte = end_byte;
            }
        }

        if start_byte < text.len() {
            let remaining = &text[start_byte..];
            if !remaining.is_empty() {
                clauses.push(remaining.to_string());
            }
        }

        clauses
    }

    /// Level 4: Split clause into word units if clause length still exceeds max_chars
    fn split_words(&self, text: &str, max_chars: usize) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut word_units = Vec::new();
        let mut current = String::new();

        for (i, word) in words.iter().enumerate() {
            if current.is_empty() {
                current.push_str(word);
            } else if current.chars().count() + 1 + word.chars().count() <= max_chars {
                current.push(' ');
                current.push_str(word);
            } else {
                current.push(' ');
                word_units.push(current.clone());
                current.clear();
                current.push_str(word);
            }

            if i == words.len() - 1 && !current.is_empty() {
                if !current.ends_with(' ') {
                    current.push(' ');
                }
                word_units.push(current.clone());
            }
        }

        if word_units.is_empty() && !text.is_empty() {
            word_units.push(text.to_string());
        }

        word_units
    }
}

fn is_sentence_ending(c: char) -> bool {
    c == '.' || c == '!' || c == '?' || c == '…'
}

fn is_clause_boundary(c: char) -> bool {
    c == ',' || c == ';' || c == ':' || c == '—' || c == '–'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ChunkConfig::default();
        assert_eq!(config.min_duration_secs, 12.0);
        assert_eq!(config.max_duration_secs, 35.0);
        assert_eq!(config.target_duration_secs, 24.0);
        assert_eq!(config.chars_per_second, 15.0);
        assert_eq!(config.min_chars(), 180);
        assert_eq!(config.max_chars(), 525);
        assert_eq!(config.target_chars(), 360);
    }

    #[test]
    fn test_short_text_single_chunk() {
        let chunker = VietnameseChunker::with_default_config();
        let text = "Xin chào Việt Nam. Đây là thử nghiệm văn bản ngắn.";
        let chunks = chunker.chunk_text(text);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].index, 0);
        assert_eq!(chunks[0].text, text);
        assert!(chunks[0].word_count > 0);
        assert!(chunks[0].char_count > 0);
    }

    #[test]
    fn test_heading_and_paragraph_chunking() {
        let config = ChunkConfig::new(2.0, 5.0, 3.0, 10.0); // max_chars = 50
        let chunker = VietnameseChunker::new(config);

        let input = "# Tiêu đề bài viết\n\nĐoạn văn thứ nhất có nội dung vừa phải.\n\nĐoạn văn thứ hai tiếp nối nội dung bài viết.";
        let chunks = chunker.chunk_text(input);

        assert!(chunks.len() >= 2);
        for chunk in &chunks {
            assert!(chunk.char_count <= 55); // fits within target boundary
        }
        // Verify all words and punctuation are preserved
        let joined = chunks
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        assert!(joined.contains("Tiêu đề bài viết"));
        assert!(joined.contains("Đoạn văn thứ nhất"));
        assert!(joined.contains("Đoạn văn thứ hai"));
    }

    #[test]
    fn test_sentence_splitting_with_abbreviation() {
        let config = ChunkConfig::new(2.0, 4.0, 3.0, 10.0); // max_chars = 40
        let chunker = VietnameseChunker::new(config);

        let text = "GS. Nguyễn Văn A làm việc tại TP. Hồ Chí Minh. Ông có nhiều công trình nghiên cứu v.v. Rất nổi tiếng!";
        let chunks = chunker.chunk_text(text);

        assert!(chunks.len() >= 2);
        // Ensure "GS." and "TP." and "v.v." were not incorrectly split as sentence ends
        let full_reconstructed = chunks
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(full_reconstructed.contains("GS. Nguyễn Văn A"));
        assert!(full_reconstructed.contains("TP. Hồ Chí Minh"));
        assert!(full_reconstructed.contains("v.v."));
    }

    #[test]
    fn test_clause_splitting_for_long_sentences() {
        let config = ChunkConfig::new(1.0, 3.0, 2.0, 10.0); // max_chars = 30
        let chunker = VietnameseChunker::new(config);

        let text = "Câu này rất dài, gồm nhiều vế phụ, được phân tách bằng dấu phẩy; và cả dấu chấm phẩy nữa.";
        let chunks = chunker.chunk_text(text);

        assert!(chunks.len() >= 2);
        // Check that clauses preserve punctuation
        let full_text = chunks
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(full_text.contains(","));
        assert!(full_text.contains(";"));
        assert!(full_text.contains("."));
    }

    #[test]
    fn test_empty_input() {
        let chunker = VietnameseChunker::with_default_config();
        assert!(chunker.chunk_text("").is_empty());
        assert!(chunker.chunk_text("   \n\t  ").is_empty());
    }

    #[test]
    fn test_sequential_indexing() {
        let config = ChunkConfig::new(1.0, 2.0, 1.5, 10.0); // max_chars = 20
        let chunker = VietnameseChunker::new(config);

        let text = "Một hai ba bốn. Năm sáu bảy tám. Chín mười mười một.";
        let chunks = chunker.chunk_text(text);

        for (idx, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, idx);
        }
    }

    #[test]
    fn test_url_email_number_preservation() {
        let config = ChunkConfig::new(2.0, 5.0, 3.0, 10.0);
        let chunker = VietnameseChunker::new(config);

        let text = "Vui lòng truy cập https://google.com hoặc gửi email tới contact@admin.vn trước 15.50 giờ. Phiên bản v1.2.0!";
        let chunks = chunker.chunk_text(text);

        let full_output = chunks
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(full_output.contains("https://google.com"));
        assert!(full_output.contains("contact@admin.vn"));
        assert!(full_output.contains("15.50"));
        assert!(full_output.contains("v1.2.0"));
    }

    #[test]
    fn test_ellipsis_and_chapter_titles() {
        let config = ChunkConfig::new(2.0, 5.0, 3.0, 10.0); // max_chars = 50
        let chunker = VietnameseChunker::new(config);

        let text = "Chương 1: Hành Trình Mới\n\nNó đã bắt đầu... nhưng chuyện gì tiếp theo đây? Thật khó nói...";
        let chunks = chunker.chunk_text(text);

        assert!(!chunks.is_empty());
        let full_output = chunks
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(full_output.contains("Chương 1: Hành Trình Mới"));
        assert!(full_output.contains("bắt đầu..."));
        assert!(full_output.contains("nói..."));
    }

    #[test]
    fn test_url_in_first_sentence_does_not_affect_subsequent_sentences() {
        let config = ChunkConfig::new(1.0, 2.0, 1.5, 10.0); // small max chars to force chunking on sentence boundary
        let chunker = VietnameseChunker::new(config);

        let text = "Truy cập https://example.com để xem thêm. Đây là câu thứ hai có độ dài vừa phải. Đây là câu thứ ba tiếp theo.";
        let sentences = chunker.split_sentences(text);

        assert_eq!(sentences.len(), 3);
        assert!(sentences[0].contains("https://example.com"));
        assert!(sentences[1].contains("Đây là câu thứ hai"));
        assert!(sentences[2].contains("Đây là câu thứ ba"));
    }

    #[test]
    fn test_10000_segment_chunking_performance_benchmark() {
        let chunker = VietnameseChunker::with_default_config();
        let sample_paragraph = "Đoạn văn ngắn thứ nhất để kiểm thử hiệu năng phân đoạn 10.000 đoạn. Đoạn văn ngắn thứ hai có độ dài tương đương.\n\n";
        let large_text = sample_paragraph.repeat(5000); // 10,000 sentences total

        let start = std::time::Instant::now();
        let chunks = chunker.chunk_text(&large_text);
        let elapsed = start.elapsed();

        println!(
            "Chunked {} text into {} segments in {:?}",
            large_text.len(),
            chunks.len(),
            elapsed
        );
    }
}
