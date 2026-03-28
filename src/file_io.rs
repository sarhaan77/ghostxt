use crate::buffer::TextBuffer;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

pub fn load_buffer(path: &Path) -> Result<TextBuffer> {
    if !path.exists() {
        return Ok(TextBuffer::new());
    }

    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let text = String::from_utf8(bytes)
        .with_context(|| format!("{} is not valid UTF-8", path.display()))?;
    Ok(TextBuffer::from_disk_text(&text))
}

pub fn save_buffer(path: &Path, buffer: &TextBuffer) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temp = NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to create temp file in {}", parent.display()))?;
    std::io::Write::write_all(&mut temp, buffer.serialized_text().as_bytes())
        .with_context(|| format!("failed to write temp file for {}", path.display()))?;
    temp.persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("failed to persist {}", path.display()))?;
    Ok(())
}

pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| PathBuf::from(path).display().to_string())
}

#[cfg(test)]
mod tests {
    use super::{load_buffer, save_buffer};
    use std::fs;

    #[test]
    fn round_trips_newline_style() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.txt");
        fs::write(&path, "a\r\nb\r\n").unwrap();

        let mut buffer = load_buffer(&path).unwrap();
        buffer.insert(buffer.len_chars(), "c");
        save_buffer(&path, &buffer).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "a\r\nb\r\nc");
    }
}
