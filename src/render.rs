use crate::buffer::TextBuffer;
use unicode_width::UnicodeWidthChar;

#[derive(Debug, Clone)]
pub struct WrapSegment {
    pub start_char: usize,
    pub end_char: usize,
    pub start_col: usize,
    pub end_col: usize,
}

#[derive(Debug, Clone)]
pub struct VisualRow {
    pub line_idx: usize,
    pub segment: WrapSegment,
    pub text: String,
}

pub fn wrap_segments(text: &str, width: usize) -> Vec<WrapSegment> {
    let width = width.max(1);
    let mut result = Vec::new();
    let mut segment_start_char = 0;
    let mut segment_start_col = 0;
    let mut absolute_col = 0;
    let mut segment_col = 0;

    for (char_idx, ch) in text.chars().enumerate() {
        let ch_width = char_display_width(ch);
        if segment_col > 0 && segment_col + ch_width > width {
            result.push(WrapSegment {
                start_char: segment_start_char,
                end_char: char_idx,
                start_col: segment_start_col,
                end_col: absolute_col,
            });
            segment_start_char = char_idx;
            segment_start_col = absolute_col;
            segment_col = 0;
        }
        absolute_col += ch_width;
        segment_col += ch_width;
    }

    result.push(WrapSegment {
        start_char: segment_start_char,
        end_char: text.chars().count(),
        start_col: segment_start_col,
        end_col: absolute_col,
    });

    if result.is_empty() {
        result.push(WrapSegment {
            start_char: 0,
            end_char: 0,
            start_col: 0,
            end_col: 0,
        });
    }

    result
}

pub fn line_display_column(text: &str, char_offset: usize) -> usize {
    text.chars()
        .take(char_offset)
        .map(char_display_width)
        .sum::<usize>()
}

pub fn char_offset_for_display_column(text: &str, display_column: usize) -> usize {
    let mut col = 0;
    for (idx, ch) in text.chars().enumerate() {
        let ch_width = char_display_width(ch);
        if col + ch_width > display_column {
            return idx;
        }
        col += ch_width;
    }
    text.chars().count()
}

pub fn collect_visual_rows(buffer: &TextBuffer, width: usize) -> Vec<VisualRow> {
    let mut rows = Vec::new();
    for line_idx in 0..buffer.line_count().max(1) {
        let text = if buffer.line_count() == 0 {
            String::new()
        } else {
            buffer.line_text(line_idx)
        };
        for segment in wrap_segments(&text, width) {
            let text_segment: String = text
                .chars()
                .skip(segment.start_char)
                .take(segment.end_char.saturating_sub(segment.start_char))
                .collect();
            rows.push(VisualRow {
                line_idx,
                segment,
                text: text_segment,
            });
        }
    }
    rows
}

fn char_display_width(ch: char) -> usize {
    UnicodeWidthChar::width(ch).unwrap_or(1).max(1)
}

#[cfg(test)]
mod tests {
    use super::{char_offset_for_display_column, line_display_column, wrap_segments};

    #[test]
    fn wraps_long_lines() {
        let segments = wrap_segments("abcdef", 4);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].start_char, 0);
        assert_eq!(segments[0].end_char, 4);
        assert_eq!(segments[1].start_char, 4);
        assert_eq!(segments[1].end_char, 6);
    }

    #[test]
    fn maps_display_columns() {
        assert_eq!(line_display_column("hello", 3), 3);
        assert_eq!(char_offset_for_display_column("hello", 4), 4);
    }
}
