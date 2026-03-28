use ropey::Rope;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

#[derive(Debug, Clone)]
pub struct TextBuffer {
    rope: Rope,
    line_ending: LineEnding,
    dirty: bool,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            line_ending: LineEnding::Lf,
            dirty: false,
        }
    }

    pub fn from_disk_text(text: &str) -> Self {
        let line_ending = if text.contains("\r\n") {
            LineEnding::Crlf
        } else {
            LineEnding::Lf
        };

        Self {
            rope: Rope::from_str(&text.replace("\r\n", "\n")),
            line_ending,
            dirty: false,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    pub fn line_index_of_char(&self, char_idx: usize) -> usize {
        self.rope.char_to_line(char_idx.min(self.len_chars()))
    }

    pub fn line_start_char(&self, line_idx: usize) -> usize {
        self.rope
            .line_to_char(line_idx.min(self.line_count().saturating_sub(1)))
    }

    pub fn line_end_char(&self, line_idx: usize) -> usize {
        let line = self
            .rope
            .line(line_idx.min(self.line_count().saturating_sub(1)));
        let mut len = line.len_chars();
        if len > 0 && line.char(len - 1) == '\n' {
            len -= 1;
        }
        self.line_start_char(line_idx) + len
    }

    pub fn line_text(&self, line_idx: usize) -> String {
        if self.line_count() == 0 {
            return String::new();
        }

        let line = self
            .rope
            .line(line_idx.min(self.line_count().saturating_sub(1)));
        let mut text: String = line.into();
        if text.ends_with('\n') {
            text.pop();
        }
        text
    }

    pub fn char_at(&self, char_idx: usize) -> Option<char> {
        (char_idx < self.len_chars()).then(|| self.rope.char(char_idx))
    }

    pub fn line_column_for_char(&self, char_idx: usize) -> (usize, usize) {
        let line = self.line_index_of_char(char_idx);
        let line_start = self.line_start_char(line);
        (line, char_idx.min(self.len_chars()) - line_start)
    }

    pub fn char_index_for_line_column(&self, line_idx: usize, column: usize) -> usize {
        let line_start = self.line_start_char(line_idx);
        let line_end = self.line_end_char(line_idx);
        line_start + column.min(line_end.saturating_sub(line_start))
    }

    pub fn insert(&mut self, char_idx: usize, text: &str) {
        self.rope.insert(char_idx.min(self.len_chars()), text);
        self.dirty = true;
    }

    pub fn delete_range(&mut self, range: Range<usize>) {
        let start = range.start.min(self.len_chars());
        let end = range.end.min(self.len_chars());
        if start >= end {
            return;
        }
        self.rope.remove(start..end);
        self.dirty = true;
    }

    pub fn delete_char_before(&mut self, char_idx: usize) -> usize {
        if char_idx == 0 {
            return 0;
        }
        self.delete_range((char_idx - 1)..char_idx);
        char_idx - 1
    }

    pub fn delete_char_after(&mut self, char_idx: usize) -> usize {
        if char_idx >= self.len_chars() {
            return char_idx.min(self.len_chars());
        }
        self.delete_range(char_idx..(char_idx + 1));
        char_idx.min(self.len_chars())
    }

    pub fn prev_word_boundary(&self, char_idx: usize) -> usize {
        let mut idx = char_idx.min(self.len_chars());
        while idx > 0 {
            let ch = self.rope.char(idx - 1);
            if !ch.is_whitespace() {
                break;
            }
            idx -= 1;
        }

        while idx > 0 {
            let ch = self.rope.char(idx - 1);
            if !is_word_char(ch) {
                break;
            }
            idx -= 1;
        }

        idx
    }

    pub fn next_word_boundary(&self, char_idx: usize) -> usize {
        let mut idx = char_idx.min(self.len_chars());
        while idx < self.len_chars() {
            let ch = self.rope.char(idx);
            if !is_word_char(ch) {
                break;
            }
            idx += 1;
        }

        while idx < self.len_chars() {
            let ch = self.rope.char(idx);
            if !ch.is_whitespace() {
                break;
            }
            idx += 1;
        }

        idx
    }

    pub fn delete_prev_word(&mut self, char_idx: usize) -> usize {
        let start = self.prev_word_boundary(char_idx);
        self.delete_range(start..char_idx.min(self.len_chars()));
        start
    }

    pub fn delete_current_line(&mut self, char_idx: usize) -> usize {
        if self.is_empty() {
            return 0;
        }

        let line_idx = self.line_index_of_char(char_idx);
        let start = self.line_start_char(line_idx);
        let end = if line_idx + 1 < self.line_count() {
            self.line_start_char(line_idx + 1)
        } else {
            self.line_end_char(line_idx)
        };

        self.delete_range(start..end);
        start.min(self.len_chars())
    }

    pub fn serialized_text(&self) -> String {
        let mut text: String = self.rope.to_string();
        if self.line_ending == LineEnding::Crlf {
            text = text.replace('\n', "\r\n");
        }
        text
    }
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::TextBuffer;

    #[test]
    fn preserves_crlf_style_for_save() {
        let buffer = TextBuffer::from_disk_text("a\r\nb\r\n");
        assert_eq!(buffer.serialized_text(), "a\r\nb\r\n");
    }

    #[test]
    fn deletes_previous_word() {
        let mut buffer = TextBuffer::from_disk_text("hello there");
        let cursor = buffer.delete_prev_word(11);
        assert_eq!(cursor, 6);
        assert_eq!(buffer.serialized_text(), "hello ");
    }

    #[test]
    fn deletes_current_line() {
        let mut buffer = TextBuffer::from_disk_text("a\nb\nc\n");
        let cursor = buffer.delete_current_line(2);
        assert_eq!(cursor, 2);
        assert_eq!(buffer.serialized_text(), "a\nc\n");
    }
}
