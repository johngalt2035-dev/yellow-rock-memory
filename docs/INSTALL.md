# Installation Guide

## Prerequisites

- **Rust toolchain** (1.75+): Install via [rustup](https://rustup.rs/)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

## Install from Source (One-Liner)

```bash
cargo install --git https://github.com/johngalt2035-dev/yellow-rock-memory.git
```

This builds a release binary and places it in `~/.cargo/bin/yellow-rock-memory`.

Or clone and build locally:

```bash
git clone https://github.com/johngalt2035-dev/yellow-rock-memory.git
cd yellow-rock-memory
cargo install --path .
```

## Binary Download

Pre-built binaries are available on the [Releases](https://github.com/johngalt2035-dev/yellow-rock-memory/releases) page for Linux (x86_64) and macOS (aarch64). Download the tarball for your platform:

```bash
tar xzf yellow-rock-memory-x86_64-unknown-linux-gnu.tar.gz
chmod +x yellow-rock-memory
sudo mv yellow-rock-memory /usr/local/bin/
```

## MCP Server Setup (Recommended)

The primary integration path is the **MCP tool server**. This makes memory operations available as native tools inside Claude Code.

### Step 1: Add MCP configuration

Create or edit `~/.claude/.mcp.json` (global -- applies to all projects) or `.mcp.json` in your project root (project-level):

```json
{
  "mcpServers": {
    "memory": {
      "command": "yellow-rock-memory",
      "args": ["--db", "/path/to/yellow-rock-memory.db", "mcp"]
    }
  }
}
```

If `yellow-rock-memory` is not in your PATH, use the full path to the binary:

```json
{
  "mcpServers": {
    "memory": {
      "command": "/usr/local/bin/yellow-rock-memory",
      "args": ["--db", "/var/lib/yellow-rock-memory/yellow-rock-memory.db", "mcp"]
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
| `GREY_ROCK_MEMORY_DB` | `yellow-rock-memory.db` | Path to the database |
| `GREY_ROCK_MEMORY_BIN` | `yellow-rock-memory` | Path to the binary |

## Systemd Service Setup (HTTP Daemon)

If you want to run the HTTP daemon as a background service (alternative to MCP):

```bash
sudo tee /etc/systemd/system/yellow-rock-memory.service > /dev/null << 'EOF'
[Unit]
Description=Yellow Rock Memory Daemon
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/yellow-rock-memory --db /var/lib/yellow-rock-memory/yellow-rock-memory.db serve
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=yellow_rock_memory=info

# Graceful shutdown checkpoints the WAL
KillSignal=SIGINT
TimeoutStopSec=10

[Install]
WantedBy=multi-user.target
EOF
```

Create the data directory and enable the service:

```bash
sudo mkdir -p /var/lib/yellow-rock-memory
sudo systemctl daemon-reload
sudo systemctl enable --now yellow-rock-memory
```

## Verify Installation

```bash
# Check the binary
yellow-rock-memory --help

# If running as MCP server, test manually:
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | yellow-rock-memory mcp
# Expected: JSON-RPC response with serverInfo

# If running as HTTP daemon, check health:
curl http://127.0.0.1:9077/api/v1/health
# Expected: {"status":"ok","service":"yellow-rock-memory"}

# Store a test memory via CLI
yellow-rock-memory store -T "Installation test" -c "It works." --tier short

# Recall it
yellow-rock-memory recall "installation"
```

## Man Page

Generate and install the man page:

```bash
# View immediately
yellow-rock-memory man | man -l -

# Install system-wide
yellow-rock-memory man | sudo tee /usr/local/share/man/man1/yellow-rock-memory.1 > /dev/null
sudo mandb
man yellow-rock-memory
```

## Shell Completions

Generate completions for your shell:

```bash
# Bash
yellow-rock-memory completions bash > ~/.local/share/bash-completion/completions/yellow-rock-memory

# Zsh
yellow-rock-memory completions zsh > ~/.zfunc/_yellow-rock-memory

# Fish
yellow-rock-memory completions fish > ~/.config/fish/completions/yellow-rock-memory.fish
```

## Multi-Node Sync Setup

If you use yellow-rock-memory on multiple machines (e.g., laptop and server), you can sync databases:

```bash
# Pull memories from a remote database (e.g., over NFS, sshfs, or rsync'd copy)
yellow-rock-memory sync /mnt/server/yellow-rock-memory.db --direction pull

# Push local memories to remote
yellow-rock-memory sync /mnt/server/yellow-rock-memory.db --direction push

# Bidirectional merge (both sides get all memories, dedup-safe)
yellow-rock-memory sync /mnt/server/yellow-rock-memory.db --direction merge
```

The sync operation uses the same dedup-safe upsert as regular stores -- title+namespace conflicts are resolved by keeping the higher priority and never downgrading tier.

## Uninstall

```bash
# Stop and remove the service (if using systemd)
sudo systemctl stop yellow-rock-memory
sudo systemctl disable yellow-rock-memory
sudo rm /etc/systemd/system/yellow-rock-memory.service
sudo systemctl daemon-reload

# Remove MCP configuration from ~/.claude/.mcp.json or .mcp.json

# Remove the binary
cargo uninstall yellow-rock-memory
# or: sudo rm /usr/local/bin/yellow-rock-memory

# Remove the database (WARNING: deletes all memories)
rm -f yellow-rock-memory.db yellow-rock-memory.db-wal yellow-rock-memory.db-shm
# or if using the systemd path:
# sudo rm -rf /var/lib/yellow-rock-memory
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `GREY_ROCK_MEMORY_DB` | `yellow-rock-memory.db` | Path to the SQLite database file |
| `RUST_LOG` | (none) | Log level filter (e.g., `yellow_rock_memory=info,tower_http=info`) |
