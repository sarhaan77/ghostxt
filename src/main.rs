use anyhow::Result;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{
    poll, read, DisableBracketedPaste, EnableBracketedPaste, Event, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use crossterm::{execute, queue};
use ghostxt::file_io;
use ghostxt::input::InputDecoder;
use ghostxt::render::collect_visual_rows;
use ghostxt::Editor;
use std::env;
use std::io::{self, Stdout, Write};
use std::time::Duration;

const STATUS_MODIFIED_COLOR: Color = Color::Rgb {
    r: 0xff,
    g: 0xd7,
    b: 0x00,
};
const STATUS_SAVED_COLOR: Color = Color::Rgb {
    r: 0x78,
    g: 0xfa,
    b: 0x50,
};

fn main() -> Result<()> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: ghostxt <path>"))?;

    let mut editor = Editor::open(path)?;
    let mut terminal = TerminalGuard::enter()?;
    let mut decoder = InputDecoder::default();

    loop {
        render(&mut terminal.stdout, &editor)?;

        if editor.should_quit() {
            break;
        }

        if poll(Duration::from_millis(16))? {
            let event = read()?;
            if matches!(event, Event::Resize(_, _)) {
                continue;
            }

            let (width, height) = size()?;
            for action in decoder.decode_event(event) {
                editor.apply(action, width as usize, height as usize)?;
            }
        } else {
            let (width, height) = size()?;
            for action in decoder.flush_pending_if_timed_out() {
                editor.apply(action, width as usize, height as usize)?;
            }
        }
    }

    Ok(())
}

fn render(stdout: &mut Stdout, editor: &Editor) -> Result<()> {
    let (cols, rows) = size()?;
    let width = cols as usize;
    let height = rows as usize;
    let body_height = height.saturating_sub(1);
    let visual_rows = collect_visual_rows(editor.buffer(), width.max(1));

    queue!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;

    for screen_row in 0..body_height {
        let visual_idx = editor.viewport_row() + screen_row;
        queue!(stdout, MoveTo(0, screen_row as u16))?;
        if let Some(row) = visual_rows.get(visual_idx) {
            queue!(stdout, Print(&row.text))?;
        }
    }

    let filename = file_io::display_name(editor.path());
    let filename_color = if editor.buffer().is_dirty() {
        STATUS_MODIFIED_COLOR
    } else {
        STATUS_SAVED_COLOR
    };
    let mut used_width = 0usize;

    queue!(stdout, MoveTo(0, body_height as u16))?;
    used_width += write_status_chunk(
        stdout,
        width,
        &filename,
        Some(filename_color),
        Some(Attribute::NoUnderline),
    )?;
    if !editor.status_message().is_empty() && used_width < width {
        used_width += write_status_chunk(
            stdout,
            width.saturating_sub(used_width),
            "  ·  ",
            None,
            Some(Attribute::Dim),
        )?;
        used_width += write_status_chunk(
            stdout,
            width.saturating_sub(used_width),
            editor.status_message(),
            None,
            Some(Attribute::Dim),
        )?;
    }
    if used_width < width {
        queue!(stdout, Print(" ".repeat(width - used_width)))?;
    }
    queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;

    let (cursor_row, cursor_col) = editor.cursor_screen_position(width.max(1));
    queue!(stdout, MoveTo(cursor_col as u16, cursor_row as u16), Show)?;
    stdout.flush()?;
    Ok(())
}

fn write_status_chunk(
    stdout: &mut Stdout,
    max_width: usize,
    text: &str,
    color: Option<Color>,
    attribute: Option<Attribute>,
) -> Result<usize> {
    if max_width == 0 {
        return Ok(0);
    }

    let chunk = text.chars().take(max_width).collect::<String>();
    if let Some(color) = color {
        queue!(stdout, SetForegroundColor(color))?;
    } else {
        queue!(stdout, ResetColor)?;
    }
    if let Some(attribute) = attribute {
        queue!(stdout, SetAttribute(attribute))?;
    } else {
        queue!(stdout, SetAttribute(Attribute::Reset))?;
    }
    queue!(stdout, Print(&chunk))?;
    Ok(chunk.chars().count())
}

struct TerminalGuard {
    stdout: Stdout,
    pushed_keyboard_flags: bool,
}

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableBracketedPaste, Hide)?;

        let pushed_keyboard_flags = execute!(
            stdout,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                    | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
            )
        )
        .is_ok();

        Ok(Self {
            stdout,
            pushed_keyboard_flags,
        })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.pushed_keyboard_flags {
            let _ = execute!(self.stdout, PopKeyboardEnhancementFlags);
        }
        let _ = execute!(
            self.stdout,
            DisableBracketedPaste,
            Show,
            LeaveAlternateScreen
        );
        let _ = disable_raw_mode();
    }
}
