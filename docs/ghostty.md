# Ghostty Setup

Ghostxt is designed for standalone Ghostty first. The editor already understands Ghostty's default fallback sequences for:

- `Cmd-Left` via `Ctrl-A`
- `Cmd-Right` via `Ctrl-E`
- `Cmd-Backspace` via `Ctrl-U`
- `Option-Left` via `Esc b`
- `Option-Right` via `Esc f`

Zero-config primary bindings:

- `Ctrl-S` save
- `Ctrl-Q` close

Opportunistic aliases when Ghostty forwards them:

- `Cmd-S` save
- `Cmd-W` close
- `Cmd-Up` file start
- `Cmd-Down` file end

Recommended existing setting:

```conf
macos-option-as-alt = left
```

Launch Ghostxt with:

```bash
cargo run -- path/to/file.txt
```
