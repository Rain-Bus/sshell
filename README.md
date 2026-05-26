# sshell

A personal SSH and shell connection manager with a terminal user interface (TUI), built in Rust.

## Features

- **Connection management** — SSH and local shell profiles with tagging, usage tracking, and smart sorting
- **Credential storage** — Passwords and SSH private keys stored securely in config
- **Import from `~/.ssh/config`** — One command to import all existing hosts
- **Local shell scan** — Auto-detects available shells from `/etc/shells`
- **Quick select** — Press `Ctrl+Q` then a number key (1–9) to connect instantly
- **Encrypted cloud sync** — Push/pull config via GitHub Gist, WebDAV, or S3 with AES-256-GCM encryption
- **CLI subcommands** — Connect, import, sync, and diagnostics without launching the TUI

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/sshell`.

### Runtime requirements

- `ssh` — for SSH connections
- `sshpass` — for password-based SSH authentication

## Usage

```bash
# Launch the TUI
sshell

# Connect directly by name
sshell connect myserver

# Import hosts from ~/.ssh/config
sshell import

# Sync
sshell sync push
sshell sync pull --strategy merge

# Diagnostics
sshell doctor
sshell doctor myserver

# Print config file path
sshell config-path
```

## Configuration

Config is stored at `~/.config/sshell/config.toml` (auto-created on first run).

```toml
version = 2

[settings]
backend = "gist"
gist_id = "..."
sync_password = "..."

[connections.myserver]
type = "ssh"
host = "example.com"
port = 22
user = "admin"
auth_ref = "myserver-password"
tags = ["production"]

[credentials.myserver-password]
type = "password"
value = "secret123"
```

## TUI Keybindings

| Key | Action |
|---|---|
| `j` / `k` / Arrow keys | Navigate connections |
| `Enter` | Connect |
| `e` | Edit connection |
| `n` | New connection |
| `d` | Delete connection |
| `/` | Search |
| `Ctrl+Q` | Quick select mode |
| `2` | Credential list |
| `s` | Settings |
| `i` | Import from SSH config |
| `q` / `Esc` | Back / Quit |

## Development

```bash
# Run tests
cargo test

# Run with debug output
cargo run
```

## License

MIT
