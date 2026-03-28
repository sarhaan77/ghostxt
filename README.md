# ghostxt

`ghostxt` is a deliberately small terminal text editor for macOS.

- normal typing
- Enter makes a new line
- Tab inserts four spaces
- simple cursor movement
- save and close without modes or command palettes

## Current Status

This is a working v1 for single-file editing.

Implemented:

- single buffer, one file per process
- soft wrap
- status bar
- UTF-8 text editing
- atomic save
- dirty-state close confirmation
- Ghostty-first input handling
- VS Code integrated terminal fallback escape sequences

Not implemented:

- syntax highlighting
- formatting
- multi-buffer support
- file picker
- config file
- plugin system

## Run It

From the project directory:

```bash
cargo run -- /tmp/test.txt
```

## Install It As A Normal Command

Build the release binary:

```bash
cargo build --release
```

Create the symlink once:

```bash
ln -sf "$(pwd)/target/release/ghostxt" ~/.local/bin/ghostxt
```

After that:

```bash
ghostxt notes.txt
ghostxt app.py
ghostxt /full/path/to/file.md
```

## Update Flow

After pulling new code, rebuild:

```bash
cargo build --release
```

Do not recreate the symlink unless one of these changed:

- the repo moved
- the binary name changed
- the symlink location changed

## Keybindings

Zero-config primary bindings:

- `Ctrl-S` save
- `Ctrl-Q` close
- `Ctrl-A` line start
- `Ctrl-E` line end
- `Ctrl-U` delete line
- `Option-Left` previous word
- `Option-Right` next word
- `Option-Backspace` delete previous word

Opportunistic aliases when the terminal forwards them:

- `Cmd-S` save
- `Cmd-W` close
- `Cmd-Left` line start
- `Cmd-Right` line end
- `Cmd-Up` file start
- `Cmd-Down` file end
- `Cmd-Backspace` delete line

Notes:

- `Ctrl-W` is intentionally unbound right now.
- `Tab` always inserts four spaces.
- close confirmation works by pressing close again, or `y`, and canceling with `Esc`.

## Ghostty Notes

Ghostty is the first-class terminal target.

Important constraint:

- the editor should not require changing Ghostty global `Cmd` behavior just to be usable

That is why `Ctrl-S` and `Ctrl-Q` exist. They are the reliable no-config path.

Keep this Ghostty setting:

```conf
macos-option-as-alt = left
```

More detail lives in [docs/ghostty.md](docs/ghostty.md).

## VS Code Notes

VS Code’s integrated terminal intercepts some `Cmd` shortcuts before the terminal app sees them.

The editor is still usable there because:

- `Ctrl-S` and `Ctrl-Q` work as the primary path
- optional `sendSequence` bindings can restore some `Cmd` parity

More detail lives in [docs/vscode.md](docs/vscode.md).

## Code Layout

- [src/main.rs](src/main.rs): terminal loop and rendering
- [src/input.rs](src/input.rs): keyboard decoding and input fallbacks
- [src/editor.rs](src/editor.rs): editor state and action handling
- [src/buffer.rs](src/buffer.rs): rope-backed text buffer
- [src/render.rs](src/render.rs): wrap math and visual rows
- [src/file_io.rs](src/file_io.rs): load/save behavior

## Design Constraints

These are intentional product choices, not accidents:

- keep it extremely simple
- single-file editing only
- no modes
- no global Ghostty config hacks required for core usability
- no config system until there is a real need
- no speculative abstraction for features that do not exist yet
- every line of code should have a reason to exist
- do not add logic for edge cases that are merely hypothetical

UI constraints:

- no line highlighting
- status bar should be bright enough to read but not harsh
- use the terminal’s existing font and theme rather than inventing a separate design system

## Notes For Future Coding Agents

Do not regress these choices without a concrete reason:

- preserve the zero-config `Ctrl-S` / `Ctrl-Q` workflow
- do not make Ghostty global key remaps a requirement
- avoid turning this into a feature-rich editor
- prefer small direct code over framework-heavy code
- avoid “just in case” code paths
- prefer deleting complexity to adding options

Good future work:

- `Ctrl-W` for delete previous word
- better status messages
- a tiny amount of syntax coloring, only if it stays simple
- formatter hooks, but only if they are explicit and low-complexity

Bad future work:

- modal editing
- command palette
- plugin architecture
- config explosion
- trying to emulate a full IDE
