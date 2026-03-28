use crate::action::Action;
use crate::buffer::TextBuffer;
use crate::file_io;
use crate::render::{
    char_offset_for_display_column, collect_visual_rows, line_display_column, wrap_segments,
};
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Editor {
    path: PathBuf,
    buffer: TextBuffer,
    cursor: usize,
    preferred_visual_col: Option<usize>,
    viewport_row: usize,
    status_message: String,
    pending_close: bool,
    should_quit: bool,
}

impl Editor {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let buffer = file_io::load_buffer(&path)?;
        Ok(Self {
            path,
            buffer,
            cursor: 0,
            preferred_visual_col: None,
            viewport_row: 0,
            status_message: String::from("Ctrl-S save  Ctrl-W close"),
            pending_close: false,
            should_quit: false,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn viewport_row(&self) -> usize {
        self.viewport_row
    }

    pub fn status_message(&self) -> &str {
        &self.status_message
    }

    pub fn pending_close(&self) -> bool {
        self.pending_close
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn apply(&mut self, action: Action, width: usize, height: usize) -> Result<()> {
        match action {
            Action::Insert(text) => {
                if self.pending_close {
                    self.pending_close = false;
                }
                self.buffer.insert(self.cursor, &text);
                self.cursor += text.chars().count();
                self.status_message.clear();
            }
            Action::Newline => {
                if self.pending_close {
                    self.pending_close = false;
                }
                self.buffer.insert(self.cursor, "\n");
                self.cursor += 1;
                self.status_message.clear();
            }
            Action::Save => {
                file_io::save_buffer(&self.path, &self.buffer)?;
                self.buffer.mark_clean();
                self.pending_close = false;
                self.status_message = format!("Saved {}", self.path.display());
            }
            Action::RequestClose => {
                if self.pending_close {
                    self.should_quit = true;
                } else if self.buffer.is_dirty() {
                    self.pending_close = true;
                    self.status_message =
                        String::from("Unsaved changes. Press Ctrl-W again or y to discard");
                } else {
                    self.should_quit = true;
                }
            }
            Action::ConfirmClose => {
                if self.pending_close {
                    self.should_quit = true;
                }
            }
            Action::CancelPrompt => {
                if self.pending_close {
                    self.pending_close = false;
                    self.status_message = String::from("Close cancelled");
                }
            }
            Action::MoveLeft => {
                self.pending_close = false;
                self.cursor = self.cursor.saturating_sub(1);
                self.preferred_visual_col = None;
            }
            Action::MoveRight => {
                self.pending_close = false;
                self.cursor = (self.cursor + 1).min(self.buffer.len_chars());
                self.preferred_visual_col = None;
            }
            Action::MoveLineStart => {
                self.pending_close = false;
                let line = self.buffer.line_index_of_char(self.cursor);
                self.cursor = self.buffer.line_start_char(line);
                self.preferred_visual_col = Some(0);
            }
            Action::MoveLineEnd => {
                self.pending_close = false;
                let line = self.buffer.line_index_of_char(self.cursor);
                self.cursor = self.buffer.line_end_char(line);
                self.preferred_visual_col = None;
            }
            Action::MoveFileStart => {
                self.pending_close = false;
                self.cursor = 0;
                self.preferred_visual_col = Some(0);
            }
            Action::MoveFileEnd => {
                self.pending_close = false;
                self.cursor = self.buffer.len_chars();
                self.preferred_visual_col = None;
            }
            Action::MoveWordLeft => {
                self.pending_close = false;
                self.cursor = self.buffer.prev_word_boundary(self.cursor);
                self.preferred_visual_col = None;
            }
            Action::MoveWordRight => {
                self.pending_close = false;
                self.cursor = self.buffer.next_word_boundary(self.cursor);
                self.preferred_visual_col = None;
            }
            Action::Backspace => {
                self.pending_close = false;
                self.cursor = self.buffer.delete_char_before(self.cursor);
            }
            Action::Delete => {
                self.pending_close = false;
                self.cursor = self.buffer.delete_char_after(self.cursor);
            }
            Action::DeleteWordLeft => {
                self.pending_close = false;
                self.cursor = self.buffer.delete_prev_word(self.cursor);
            }
            Action::DeleteLine => {
                self.pending_close = false;
                self.cursor = self.buffer.delete_current_line(self.cursor);
            }
            Action::MoveUp => {
                self.pending_close = false;
                self.move_vertical(-1, width.max(1));
            }
            Action::MoveDown => {
                self.pending_close = false;
                self.move_vertical(1, width.max(1));
            }
        }

        self.ensure_visible(width.max(1), height.saturating_sub(1).max(1));
        Ok(())
    }

    pub fn cursor_screen_position(&self, width: usize) -> (usize, usize) {
        let width = width.max(1);
        let rows = collect_visual_rows(&self.buffer, width);
        let current_row = self.cursor_visual_row(width, &rows);
        let line = self.buffer.line_index_of_char(self.cursor);
        let line_text = self.buffer.line_text(line);
        let line_start = self.buffer.line_start_char(line);
        let line_offset = self.cursor.saturating_sub(line_start);
        let display_col = line_display_column(&line_text, line_offset);
        let segments = wrap_segments(&line_text, width);
        let segment = segments
            .iter()
            .find(|segment| {
                display_col >= segment.start_col
                    && (display_col < segment.end_col
                        || (segment.start_col == segment.end_col
                            && display_col == segment.start_col)
                        || (display_col == segment.end_col
                            && segment.end_char == line_text.chars().count()))
            })
            .cloned()
            .unwrap_or_else(|| segments.last().cloned().unwrap());

        (
            current_row.saturating_sub(self.viewport_row),
            display_col.saturating_sub(segment.start_col),
        )
    }

    fn move_vertical(&mut self, delta: isize, width: usize) {
        let line = self.buffer.line_index_of_char(self.cursor);
        let line_text = self.buffer.line_text(line);
        let line_start = self.buffer.line_start_char(line);
        let line_offset = self.cursor.saturating_sub(line_start);
        let display_col = line_display_column(&line_text, line_offset);
        let segments = wrap_segments(&line_text, width);
        let current_segment_idx = segment_index_for_display_col(&segments, display_col);
        let local_col = display_col.saturating_sub(segments[current_segment_idx].start_col);
        let desired = self.preferred_visual_col.unwrap_or(local_col);

        let target = if delta < 0 {
            if current_segment_idx > 0 {
                Some((line, current_segment_idx - 1, desired))
            } else if line > 0 {
                let previous_text = self.buffer.line_text(line - 1);
                let previous_segments = wrap_segments(&previous_text, width);
                Some((line - 1, previous_segments.len().saturating_sub(1), desired))
            } else {
                None
            }
        } else if current_segment_idx + 1 < segments.len() {
            Some((line, current_segment_idx + 1, desired))
        } else if line + 1 < self.buffer.line_count() {
            Some((line + 1, 0, desired))
        } else {
            None
        };

        if let Some((target_line, target_segment_idx, desired_col)) = target {
            let target_text = self.buffer.line_text(target_line);
            let target_segments = wrap_segments(&target_text, width);
            let target_segment = &target_segments[target_segment_idx];
            let target_display_col = target_segment.start_col
                + desired_col.min(
                    target_segment
                        .end_col
                        .saturating_sub(target_segment.start_col),
                );
            let target_offset = char_offset_for_display_column(&target_text, target_display_col);
            let target_line_start = self.buffer.line_start_char(target_line);
            self.cursor =
                (target_line_start + target_offset).min(self.buffer.line_end_char(target_line));
            self.preferred_visual_col = Some(desired_col);
        }
    }

    fn ensure_visible(&mut self, width: usize, height: usize) {
        let rows = collect_visual_rows(&self.buffer, width.max(1));
        let cursor_row = self.cursor_visual_row(width, &rows);
        if cursor_row < self.viewport_row {
            self.viewport_row = cursor_row;
        } else if cursor_row >= self.viewport_row + height.max(1) {
            self.viewport_row = cursor_row + 1 - height.max(1);
        }
    }

    fn cursor_visual_row(&self, width: usize, rows: &[crate::render::VisualRow]) -> usize {
        let line = self.buffer.line_index_of_char(self.cursor);
        let line_start = self.buffer.line_start_char(line);
        let line_text = self.buffer.line_text(line);
        let line_offset = self.cursor.saturating_sub(line_start);
        let display_col = line_display_column(&line_text, line_offset);
        let segments = wrap_segments(&line_text, width.max(1));
        let segment_idx = segment_index_for_display_col(&segments, display_col);

        rows.iter()
            .enumerate()
            .find_map(|(idx, row)| {
                (row.line_idx == line
                    && row.segment.start_char == segments[segment_idx].start_char
                    && row.segment.end_char == segments[segment_idx].end_char)
                    .then_some(idx)
            })
            .unwrap_or(0)
    }
}

fn segment_index_for_display_col(
    segments: &[crate::render::WrapSegment],
    display_col: usize,
) -> usize {
    segments
        .iter()
        .enumerate()
        .find_map(|(idx, segment)| {
            (display_col >= segment.start_col
                && (display_col < segment.end_col
                    || (segment.start_col == segment.end_col && display_col == segment.start_col)
                    || (display_col == segment.end_col)))
                .then_some(idx)
        })
        .unwrap_or_else(|| segments.len().saturating_sub(1))
}

#[cfg(test)]
mod tests {
    use super::Editor;
    use crate::Action;

    #[test]
    fn move_down_respects_soft_wrap() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.txt");
        std::fs::write(&path, "abcdef\nxy").unwrap();

        let mut editor = Editor::open(&path).unwrap();
        editor.apply(Action::MoveRight, 4, 8).unwrap();
        editor.apply(Action::MoveRight, 4, 8).unwrap();
        editor.apply(Action::MoveRight, 4, 8).unwrap();
        editor.apply(Action::MoveDown, 4, 8).unwrap();

        assert_eq!(editor.cursor(), 6);
    }

    #[test]
    fn close_requires_confirmation_when_dirty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.txt");

        let mut editor = Editor::open(&path).unwrap();
        editor.apply(Action::Insert("x".into()), 20, 10).unwrap();
        editor.apply(Action::RequestClose, 20, 10).unwrap();
        assert!(editor.pending_close());
        assert!(!editor.should_quit());
        editor.apply(Action::RequestClose, 20, 10).unwrap();
        assert!(editor.should_quit());
    }
}
