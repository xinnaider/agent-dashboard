# agent-dashboard

Windows-native TUI dashboard for monitoring Claude Code CLI sessions. Equivalent to [gavraz/recon](https://github.com/gavraz/recon) but built for Windows — without tmux.

## Requirements

- Windows 10/11
- [Rust](https://rustup.rs) installed
- Claude Code CLI in use (sessions at `~/.claude/`)

## Installation

```bash
git clone <repo-url>
cd agent-dashboard
cargo build --release
```

The binary will be at `target/release/agent-dashboard.exe`.

Optionally, add it to your PATH:

```bash
cp target/release/agent-dashboard.exe "$HOME/.local/bin/"
```

## Usage

### Table view (default)

```bash
agent-dashboard
```

Opens a live table showing all active Claude Code sessions:

```
 #  PID    Project          Directory        Status    Model         Context    Last Activity
 1  12345  my-project       src/             Working   sonnet-4-5    [###   ]   5s ago
 2  67890  other-repo       /                Idle      opus-4-5      [#     ]   2m ago
```

| Column | Description |
|--------|-------------|
| # | Row index |
| PID | Process ID of the Claude CLI |
| Project | Git repository name |
| Directory | Working directory relative to repo root |
| Status | Working / Idle / Waiting Input / New |
| Model | Claude model in use |
| Context | Token usage bar (input/context window) |
| Last Activity | Time since last JSONL entry |

**Status colors:**

- `Working` — green, active request in progress
- `Idle` — dim, no recent activity
- `Waiting Input` — yellow, awaiting user input (row highlighted)
- `New` — blue, session just started

### Tamagotchi view

```bash
agent-dashboard view
```

Or press `v` inside the table view.

Shows pixel-art sprites for each session, grouped by project (room). Each sprite animates based on session status.

### JSON output

```bash
agent-dashboard json
```

Prints all session state as JSON. Useful for scripting or piping to other tools.

```json
[
  {
    "pid": 12345,
    "session_id": "abc123",
    "project": "my-project",
    "directory": "src/",
    "status": "Working",
    "model": "claude-sonnet-4-5",
    "total_input_tokens": 4200,
    "total_output_tokens": 312,
    "last_activity": "2026-03-24T10:00:00Z"
  }
]
```

## Keybindings

### Table view

| Key | Action |
|-----|--------|
| `j` / `k` | Move selection down / up |
| `v` | Switch to tamagotchi view |
| `r` | Force refresh |
| `q` | Quit |

### Tamagotchi view

| Key | Action |
|-----|--------|
| `h` / `l` | Select previous / next agent |
| `1` – `9` | Zoom into room N |
| `Esc` | Zoom out / back to table |
| `q` | Quit |

## How it works

1. Detects running Claude Code CLI processes via a single PowerShell `Get-Process` call (filters by `.local\` path to exclude the desktop app)
2. Maps each PID to a session ID via `~/.claude/sessions/{PID}.json`
3. Finds the corresponding JSONL file in `~/.claude/projects/*/` and parses it for token usage, model, and activity
4. Status is inferred from the JSONL tail: last entry type + timestamp recency (30s threshold for "Working")
5. Refreshes every 2 seconds

## Troubleshooting

**No sessions appear**

- Make sure Claude Code CLI is running (not just the desktop app)
- Verify session files exist: `ls ~/.claude/sessions/`
- Check that `claude.exe` is in `.local\bin\`, not `WindowsApps\`

**`cargo: command not found`**

Add Rust to your PATH:

```bash
export PATH="$PATH:/c/Users/$USERNAME/.cargo/bin"
```

Or add it permanently to your shell profile (`~/.bashrc`, `~/.bash_profile`).
