# Installation Guide

## Prerequisites

- **Rust toolchain** (1.75+): Install via [rustup](https://rustup.rs/)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

## Install from Source (One-Liner)

```bash
cargo install --git https://github.com/johngalt2035-dev/grey-rock-memory.git
```

This builds a release binary and places it in `~/.cargo/bin/grey-rock-memory`.

Or clone and build locally:

```bash
git clone https://github.com/johngalt2035-dev/grey-rock-memory.git
cd grey-rock-memory
cargo install --path .
```

## Binary Download

Pre-built binaries are available on the [Releases](https://github.com/johngalt2035-dev/grey-rock-memory/releases) page for Linux (x86_64) and macOS (aarch64). Download the tarball for your platform:

```bash
tar xzf grey-rock-memory-x86_64-unknown-linux-gnu.tar.gz
chmod +x grey-rock-memory
sudo mv grey-rock-memory /usr/local/bin/
```

## MCP Server Setup (Recommended)

The primary integration path is the **MCP tool server**. This makes memory operations available as native tools inside Claude Code.

### Step 1: Add MCP configuration

Create or edit `~/.claude/.mcp.json` (global -- applies to all projects) or `.mcp.json` in your project root (project-level):

```json
{
  "mcpServers": {
    "memory": {
      "command": "grey-rock-memory",
      "args": ["--db", "/path/to/grey-rock-memory.db", "mcp"]
    }
  }
}
```

If `grey-rock-memory` is not in your PATH, use the full path to the binary:

```json
{
  "mcpServers": {
    "memory": {
      "command": "/usr/local/bin/grey-rock-memory",
      "args": ["--db", "/var/lib/grey-rock-memory/grey-rock-memory.db", "mcp"]
    }
  }
}
```

> **Important:** MCP server configuration does **not** go in `settings.json` or `settings.local.json` -- those files do not support `mcpServers`.

### Step 2: Verify

Restart Claude Code. You should see 8 new tools available: `memory_store`, `memory_recall`, `memory_search`, `memory_list`, `memory_delete`, `memory_promote`, `memory_forget`, `memory_stats`.

### Step 3: Test

Ask Claude to store a memory. It should use the `memory_store` tool automatically.

## Hook Installation (Optional)

The `hooks/session-start.sh` script auto-recalls relevant memories at the start of each Claude Code session.

### Install the hook

```bash
# Copy the hook
cp hooks/session-start.sh ~/.claude/hooks/

# Make it executable
chmod +x ~/.claude/hooks/session-start.sh
```

### Configure the hook in settings.json

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "command": "~/.claude/hooks/session-start.sh"
      }
    ]
  }
}
```

### Environment variables for the hook

| Variable | Default | Description |
|----------|---------|-------------|
| `GREY_ROCK_MEMORY_DB` | `grey-rock-memory.db` | Path to the database |
| `GREY_ROCK_MEMORY_BIN` | `grey-rock-memory` | Path to the binary |

## Systemd Service Setup (HTTP Daemon)

If you want to run the HTTP daemon as a background service (alternative to MCP):

```bash
sudo tee /etc/systemd/system/grey-rock-memory.service > /dev/null << 'EOF'
[Unit]
Description=Grey Rock Memory Daemon
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/grey-rock-memory --db /var/lib/grey-rock-memory/grey-rock-memory.db serve
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=grey_rock_memory=info

# Graceful shutdown checkpoints the WAL
KillSignal=SIGINT
TimeoutStopSec=10

[Install]
WantedBy=multi-user.target
EOF
```

Create the data directory and enable the service:

```bash
sudo mkdir -p /var/lib/grey-rock-memory
sudo systemctl daemon-reload
sudo systemctl enable --now grey-rock-memory
```

## Verify Installation

```bash
# Check the binary
grey-rock-memory --help

# If running as MCP server, test manually:
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | grey-rock-memory mcp
# Expected: JSON-RPC response with serverInfo

# If running as HTTP daemon, check health:
curl http://127.0.0.1:9077/api/v1/health
# Expected: {"status":"ok","service":"grey-rock-memory"}

# Store a test memory via CLI
grey-rock-memory store -T "Installation test" -c "It works." --tier short

# Recall it
grey-rock-memory recall "installation"
```

## Man Page

Generate and install the man page:

```bash
# View immediately
grey-rock-memory man | man -l -

# Install system-wide
grey-rock-memory man | sudo tee /usr/local/share/man/man1/grey-rock-memory.1 > /dev/null
sudo mandb
man grey-rock-memory
```

## Shell Completions

Generate completions for your shell:

```bash
# Bash
grey-rock-memory completions bash > ~/.local/share/bash-completion/completions/grey-rock-memory

# Zsh
grey-rock-memory completions zsh > ~/.zfunc/_grey-rock-memory

# Fish
grey-rock-memory completions fish > ~/.config/fish/completions/grey-rock-memory.fish
```

## Multi-Node Sync Setup

If you use grey-rock-memory on multiple machines (e.g., laptop and server), you can sync databases:

```bash
# Pull memories from a remote database (e.g., over NFS, sshfs, or rsync'd copy)
grey-rock-memory sync /mnt/server/grey-rock-memory.db --direction pull

# Push local memories to remote
grey-rock-memory sync /mnt/server/grey-rock-memory.db --direction push

# Bidirectional merge (both sides get all memories, dedup-safe)
grey-rock-memory sync /mnt/server/grey-rock-memory.db --direction merge
```

The sync operation uses the same dedup-safe upsert as regular stores -- title+namespace conflicts are resolved by keeping the higher priority and never downgrading tier.

## Uninstall

```bash
# Stop and remove the service (if using systemd)
sudo systemctl stop grey-rock-memory
sudo systemctl disable grey-rock-memory
sudo rm /etc/systemd/system/grey-rock-memory.service
sudo systemctl daemon-reload

# Remove MCP configuration from ~/.claude/.mcp.json or .mcp.json

# Remove the binary
cargo uninstall grey-rock-memory
# or: sudo rm /usr/local/bin/grey-rock-memory

# Remove the database (WARNING: deletes all memories)
rm -f grey-rock-memory.db grey-rock-memory.db-wal grey-rock-memory.db-shm
# or if using the systemd path:
# sudo rm -rf /var/lib/grey-rock-memory
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `GREY_ROCK_MEMORY_DB` | `grey-rock-memory.db` | Path to the SQLite database file |
| `RUST_LOG` | (none) | Log level filter (e.g., `grey_rock_memory=info,tower_http=info`) |
