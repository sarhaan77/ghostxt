use crate::action::Action;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::{Duration, Instant};

const ESCAPE_FLUSH_DELAY: Duration = Duration::from_millis(25);

#[derive(Debug, Default)]
pub struct InputDecoder {
    pending_escape: Option<String>,
    pending_since: Option<Instant>,
}

impl InputDecoder {
    pub fn decode_event(&mut self, event: Event) -> Vec<Action> {
        match event {
            Event::Key(key) => self.decode_key(key),
            Event::Paste(text) => {
                self.clear_pending();
                vec![Action::Insert(text)]
            }
            _ => Vec::new(),
        }
    }

    pub fn flush_pending_if_timed_out(&mut self) -> Vec<Action> {
        if let Some(since) = self.pending_since {
            if since.elapsed() >= ESCAPE_FLUSH_DELAY {
                return self.flush_pending();
            }
        }
        Vec::new()
    }

    fn decode_key(&mut self, key: KeyEvent) -> Vec<Action> {
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return Vec::new();
        }

        if matches!(key.code, KeyCode::Esc) && key.modifiers.is_empty() {
            self.pending_escape = Some(String::from("\u{1b}"));
            self.pending_since = Some(Instant::now());
            return Vec::new();
        }

        if self.pending_escape.is_some() {
            return self.decode_pending(key);
        }

        if let Some(action) = decode_key_event(key) {
            return vec![action];
        }

        Vec::new()
    }

    fn decode_pending(&mut self, key: KeyEvent) -> Vec<Action> {
        let mut pending = self.pending_escape.take().unwrap_or_default();
        pending.push_key(key);

        if let Some(action) = decode_escaped_sequence(&pending) {
            self.clear_pending();
            return vec![action];
        }

        if is_possible_prefix(&pending) {
            self.pending_since = Some(Instant::now());
            self.pending_escape = Some(pending);
            return Vec::new();
        }

        self.clear_pending();
        let mut actions = vec![Action::CancelPrompt];
        if let Some(action) = decode_key_event(key) {
            actions.push(action);
        }
        actions
    }

    fn flush_pending(&mut self) -> Vec<Action> {
        let pending = self.pending_escape.take();
        self.pending_since = None;
        if pending.is_some() {
            vec![Action::CancelPrompt]
        } else {
            Vec::new()
        }
    }

    fn clear_pending(&mut self) {
        self.pending_escape = None;
        self.pending_since = None;
    }
}

pub fn decode_key_event(key: KeyEvent) -> Option<Action> {
    let modifiers = key.modifiers;

    if modifiers.contains(KeyModifiers::SUPER) {
        return decode_super_shortcut(key.code);
    }

    if modifiers == KeyModifiers::ALT {
        return match key.code {
            KeyCode::Left => Some(Action::MoveWordLeft),
            KeyCode::Right => Some(Action::MoveWordRight),
            KeyCode::Backspace => Some(Action::DeleteWordLeft),
            KeyCode::Char('b') => Some(Action::MoveWordLeft),
            KeyCode::Char('f') => Some(Action::MoveWordRight),
            _ => None,
        };
    }

    if modifiers == KeyModifiers::CONTROL {
        return match key.code {
            KeyCode::Char('w') | KeyCode::Char('W') => Some(Action::RequestClose),
            KeyCode::Char('s') | KeyCode::Char('S') => Some(Action::Save),
            KeyCode::Char('a') => Some(Action::MoveLineStart),
            KeyCode::Char('e') => Some(Action::MoveLineEnd),
            KeyCode::Char('u') => Some(Action::DeleteLine),
            _ => None,
        };
    }

    if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::SUPER | KeyModifiers::ALT) {
        return match key.code {
            KeyCode::Left => Some(Action::MoveLeft),
            KeyCode::Right => Some(Action::MoveRight),
            KeyCode::Up => Some(Action::MoveUp),
            KeyCode::Down => Some(Action::MoveDown),
            KeyCode::Home => Some(Action::MoveLineStart),
            KeyCode::End => Some(Action::MoveLineEnd),
            KeyCode::Enter => Some(Action::Newline),
            KeyCode::Backspace => Some(Action::Backspace),
            KeyCode::Delete => Some(Action::Delete),
            KeyCode::Tab => Some(Action::Insert("    ".to_string())),
            KeyCode::Char('y') => Some(Action::ConfirmClose),
            KeyCode::Esc => Some(Action::CancelPrompt),
            KeyCode::Char(ch) => Some(Action::Insert(ch.to_string())),
            _ => None,
        };
    }

    None
}

fn decode_super_shortcut(code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Left => Some(Action::MoveLineStart),
        KeyCode::Right => Some(Action::MoveLineEnd),
        KeyCode::Up => Some(Action::MoveFileStart),
        KeyCode::Down => Some(Action::MoveFileEnd),
        KeyCode::Backspace => Some(Action::DeleteLine),
        _ => None,
    }
}

fn decode_escaped_sequence(sequence: &str) -> Option<Action> {
    match sequence {
        "\u{1b}b" => Some(Action::MoveWordLeft),
        "\u{1b}f" => Some(Action::MoveWordRight),
        "\u{1b}[9502u" => Some(Action::MoveFileStart),
        "\u{1b}[9503u" => Some(Action::MoveFileEnd),
        "\u{1b}[9504u" => Some(Action::DeleteLine),
        "\u{1b}[9505u" => Some(Action::MoveLineStart),
        "\u{1b}[9506u" => Some(Action::MoveLineEnd),
        _ => None,
    }
}

fn is_possible_prefix(sequence: &str) -> bool {
    const KNOWN_SEQUENCES: [&str; 7] = [
        "\u{1b}b",
        "\u{1b}f",
        "\u{1b}[9502u",
        "\u{1b}[9503u",
        "\u{1b}[9504u",
        "\u{1b}[9505u",
        "\u{1b}[9506u",
    ];

    KNOWN_SEQUENCES
        .iter()
        .any(|candidate| candidate.starts_with(sequence))
}

trait PushKey {
    fn push_key(&mut self, key: KeyEvent);
}

impl PushKey for String {
    fn push_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(ch) => self.push(ch),
            KeyCode::Backspace => self.push('\u{8}'),
            KeyCode::Esc => self.push('\u{1b}'),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_key_event, InputDecoder};
    use crate::Action;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn decodes_super_navigation_shortcuts() {
        assert_eq!(
            decode_key_event(key(KeyCode::Left, KeyModifiers::SUPER)),
            Some(Action::MoveLineStart)
        );
        assert_eq!(
            decode_key_event(key(KeyCode::Backspace, KeyModifiers::SUPER)),
            Some(Action::DeleteLine)
        );
    }

    #[test]
    fn decodes_control_shortcuts() {
        assert_eq!(
            decode_key_event(key(KeyCode::Char('a'), KeyModifiers::CONTROL)),
            Some(Action::MoveLineStart)
        );
        assert_eq!(
            decode_key_event(key(KeyCode::Char('u'), KeyModifiers::CONTROL)),
            Some(Action::DeleteLine)
        );
        assert_eq!(
            decode_key_event(key(KeyCode::Char('s'), KeyModifiers::CONTROL)),
            Some(Action::Save)
        );
        assert_eq!(
            decode_key_event(key(KeyCode::Char('w'), KeyModifiers::CONTROL)),
            Some(Action::RequestClose)
        );
    }

    #[test]
    fn decodes_private_vscode_sequences() {
        let mut decoder = InputDecoder::default();
        let sequence = [
            Event::Key(key(KeyCode::Esc, KeyModifiers::NONE)),
            Event::Key(key(KeyCode::Char('['), KeyModifiers::NONE)),
            Event::Key(key(KeyCode::Char('9'), KeyModifiers::NONE)),
            Event::Key(key(KeyCode::Char('5'), KeyModifiers::NONE)),
            Event::Key(key(KeyCode::Char('0'), KeyModifiers::NONE)),
            Event::Key(key(KeyCode::Char('2'), KeyModifiers::NONE)),
            Event::Key(key(KeyCode::Char('u'), KeyModifiers::NONE)),
        ];

        let mut actions = Vec::new();
        for event in sequence {
            actions.extend(decoder.decode_event(event));
        }

        assert_eq!(actions, vec![Action::MoveFileStart]);
    }
}
