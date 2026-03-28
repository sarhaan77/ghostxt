# VS Code Integrated Terminal

Ghostxt works best in Ghostty. In VS Code, some `Cmd` shortcuts are intercepted by the workbench before the terminal process sees them. The cleanest v1 setup is:

Zero-config primary bindings:

- `Ctrl-S` save
- `Ctrl-Q` close

If you want `Cmd` parity in VS Code too, add the forwarding/snippet below.

1. Enable terminal keybinding forwarding:

```json
{
  "terminal.integrated.sendKeybindingsToShell": true
}
```

2. Add keybindings that explicitly send Ghostxt's private CSI commands into the active terminal:

```json
[
  {
    "key": "cmd+s",
    "command": "workbench.action.terminal.sendSequence",
    "args": { "text": "\u001b[9500u" },
    "when": "terminalFocus"
  },
  {
    "key": "cmd+w",
    "command": "workbench.action.terminal.sendSequence",
    "args": { "text": "\u001b[9501u" },
    "when": "terminalFocus"
  },
  {
    "key": "cmd+up",
    "command": "workbench.action.terminal.sendSequence",
    "args": { "text": "\u001b[9502u" },
    "when": "terminalFocus"
  },
  {
    "key": "cmd+down",
    "command": "workbench.action.terminal.sendSequence",
    "args": { "text": "\u001b[9503u" },
    "when": "terminalFocus"
  },
  {
    "key": "cmd+backspace",
    "command": "workbench.action.terminal.sendSequence",
    "args": { "text": "\u001b[9504u" },
    "when": "terminalFocus"
  },
  {
    "key": "cmd+left",
    "command": "workbench.action.terminal.sendSequence",
    "args": { "text": "\u001b[9505u" },
    "when": "terminalFocus"
  },
  {
    "key": "cmd+right",
    "command": "workbench.action.terminal.sendSequence",
    "args": { "text": "\u001b[9506u" },
    "when": "terminalFocus"
  }
]
```

Ghostxt recognizes those escape sequences internally and maps them onto the same action enum as native terminal shortcuts.
