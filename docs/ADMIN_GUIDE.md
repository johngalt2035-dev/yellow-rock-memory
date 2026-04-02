# Admin Guide

## Deployment Options

### MCP Server (Recommended)

The simplest deployment is as an MCP tool server. No daemon process to manage -- Claude Code spawns the process on demand.

Configure in `~/.claude/.mcp.json` (global -- applies to all projects) or `.mcp.json` in the project root (project-level):

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

> MCP server configuration does **not** go in `settings.json` or `settings.local.json` -- those files do not support `mcpServers`.

The MCP server:
- Starts when Claude Code opens a session
- Communicates over stdio (JSON-RPC)
- Stops when the session ends
- Uses the same SQLite database as the CLI and HTTP daemon

### Standalone (Development)

Run the HTTP daemon directly in the foreground:

```bash
grey-rock-memory --db /path/to/grey-rock-memory.db serve
```

The daemon listens on `127.0.0.1:9077` by default.

### Systemd (Production HTTP Daemon)

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
Environment=RUST_LOG=grey_rock_memory=info,tower_http=info

# Graceful shutdown: checkpoints WAL before exit
KillSignal=SIGINT
TimeoutStopSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo mkdir -p /var/lib/grey-rock-memory
sudo systemctl daemon-reload
sudo systemctl enable --now grey-rock-memory
```

Check status:

```bash
sudo systemctl status grey-rock-memory
sudo journalctl -u grey-rock-memory -f
```

### Docker

Example Dockerfile:

```dockerfile
FROM rust:1.75-slim AS builder
WORKDIR /src
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /src/target/release/grey-rock-memory /usr/local/bin/
VOLUME /data
EXPOSE 9077
CMD ["grey-rock-memory", "--db", "/data/grey-rock-memory.db", "serve"]
```

Build and run:

```bash
docker build -t grey-rock-memory .
docker run -d -p 127.0.0.1:9077:9077 -v grey-rock-memory-data:/data grey-rock-memory
```

## Configuration

### CLI Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--db <path>` | `grey-rock-memory.db` | Path to SQLite database |
| `--host <addr>` | `127.0.0.1` | Bind address (serve only) |
| `--port <port>` | `9077` | Bind port (serve only) |
| `--json` | `false` | JSON output for CLI commands |

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `GREY_ROCK_MEMORY_DB` | `grey-rock-memory.db` | Database path (overridden by `--db`) |
| `RUST_LOG` | (none) | Logging filter (e.g., `grey_rock_memory=info,tower_http=debug`) |

### Compile-Time Constants

These are set in the source code and require recompilation to change:

| Constant | Value | Location |
|----------|-------|----------|
| `DEFAULT_PORT` | 9077 | `main.rs` |
| `GC_INTERVAL_SECS` | 1800 (30 min) | `main.rs` |
| `MAX_CONTENT_SIZE` | 65536 (64 KB) | `models.rs` |
| `PROMOTION_THRESHOLD` | 5 accesses | `models.rs` |
| `SHORT_TTL_EXTEND_SECS` | 3600 (1 hour) | `models.rs` |
| `MID_TTL_EXTEND_SECS` | 86400 (1 day) | `models.rs` |

## Graceful Shutdown

The HTTP daemon handles SIGINT (Ctrl+C) gracefully:

1. Stops accepting new connections
2. Waits for in-flight requests to complete
3. Checkpoints the WAL (`PRAGMA wal_checkpoint(TRUNCATE)`)
4. Exits cleanly

For systemd, use `KillSignal=SIGINT` and `TimeoutStopSec=10` to ensure the checkpoint completes.

The MCP server exits cleanly when stdin closes (Claude Code session ends).

## Database Management

### SQLite Settings

The database uses these pragmas (set automatically on open):

- **WAL mode** -- write-ahead logging for concurrent reads
- **busy_timeout = 5000** -- 5 second wait on lock contention
- **synchronous = NORMAL** -- balanced durability/performance
- **foreign_keys = ON** -- enforced referential integrity (links cascade on delete)

### Backup

**Live backup (while daemon is running):**

```bash
sqlite3 /path/to/grey-rock-memory.db ".backup /path/to/backup.db"
```

**JSON export (includes links):**

```bash
grey-rock-memory --db /path/to/grey-rock-memory.db export > backup.json
```

**File copy (daemon must be stopped or use WAL checkpoint first):**

```bash
systemctl stop grey-rock-memory
cp /path/to/grey-rock-memory.db /path/to/backup.db
cp /path/to/grey-rock-memory.db-wal /path/to/backup.db-wal 2>/dev/null
systemctl start grey-rock-memory
```

### Restore

**From JSON (preserves links):**

```bash
grey-rock-memory --db /path/to/new.db import < backup.json
```

**From SQLite backup:**

```bash
systemctl stop grey-rock-memory
cp /path/to/backup.db /var/lib/grey-rock-memory/grey-rock-memory.db
systemctl start grey-rock-memory
```

### Migration

The schema is auto-migrated on startup. The `schema_version` table tracks the current version (currently 2). Migrations are forward-only and non-destructive.

- v1 -> v2: Added `confidence` (REAL) and `source` (TEXT) columns

### Database Maintenance

Manually trigger garbage collection:

```bash
# Via CLI
grey-rock-memory gc

# Via API
curl -X POST http://127.0.0.1:9077/api/v1/gc
```

Compact the database (reduces file size after many deletions):

```bash
sqlite3 /path/to/grey-rock-memory.db "VACUUM"
```

Rebuild the FTS index (if it becomes corrupt):

```bash
sqlite3 /path/to/grey-rock-memory.db "INSERT INTO memories_fts(memories_fts) VALUES('rebuild')"
```

## Monitoring

### Health Endpoint (Deep Check)

```bash
curl http://127.0.0.1:9077/api/v1/health
```

The health check performs a **deep verification**:
1. Database is readable (runs `SELECT COUNT(*) FROM memories`)
2. FTS5 index integrity check (`INSERT INTO memories_fts(memories_fts) VALUES('integrity-check')`)

Returns `200 OK` with `{"status": "ok", "service": "grey-rock-memory"}` if healthy.
Returns `503 Service Unavailable` with `{"status": "error", "service": "grey-rock-memory"}` if the database or FTS index is unhealthy.

### Stats Endpoint

```bash
curl http://127.0.0.1:9077/api/v1/stats
```

Returns:
- Total memory count
- Breakdown by tier
- Breakdown by namespace
- Memories expiring within 1 hour
- Total link count
- Database file size in bytes

### MCP Server Monitoring

The MCP server logs to stderr. Monitor via:

```bash
# If running via Claude Code, check Claude Code's MCP logs
# If running manually:
grey-rock-memory mcp 2>mcp-server.log
```

Key log messages:
- `grey-rock-memory MCP server started (stdio)` -- server is ready
- `grey-rock-memory MCP server stopped` -- stdin closed, server exiting

### Logs

The HTTP daemon logs via `tracing` with configurable levels:

```bash
# Info level (default recommended)
RUST_LOG=grey_rock_memory=info,tower_http=info grey-rock-memory serve

# Debug level (verbose, includes all HTTP requests)
RUST_LOG=grey_rock_memory=debug,tower_http=debug grey-rock-memory serve

# Trace level (extremely verbose)
RUST_LOG=grey_rock_memory=trace grey-rock-memory serve
```

With systemd, logs go to the journal:

```bash
sudo journalctl -u grey-rock-memory -f
sudo journalctl -u grey-rock-memory --since "1 hour ago"
```

### Monitoring Script Example

```bash
#!/bin/bash
HEALTH=$(curl -sf http://127.0.0.1:9077/api/v1/health | jq -r '.status')
if [ "$HEALTH" != "ok" ]; then
    echo "grey-rock-memory health check failed"
    systemctl restart grey-rock-memory
fi
```

## CI/CD Pipeline

The project uses GitHub Actions for continuous integration and release automation.

### CI (Every Push and PR)

Runs on `ubuntu-latest` and `macos-latest`:

1. **Formatting** -- `cargo fmt --check`
2. **Linting** -- `cargo clippy -- -D warnings`
3. **Tests** -- `cargo test` (41 tests: 8 unit + 33 integration)
4. **Build** -- `cargo build --release`

Uses `Swatinem/rust-cache@v2` for build caching.

### Release (On Tag Push)

Triggered by tags matching `v*` (e.g., `v0.1.0`):

1. Builds release binaries for:
   - `x86_64-unknown-linux-gnu` (Ubuntu)
   - `aarch64-apple-darwin` (macOS ARM)
2. Packages each as `grey-rock-memory-<target>.tar.gz`
3. Creates a GitHub Release with the artifacts

### Running CI Locally

```bash
# Replicate the CI checks
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

## Multi-Node Sync

For multi-machine deployments (e.g., laptop + server, or multiple workstations), use the `sync` command to keep databases in sync.

### Manual Sync

```bash
# Pull remote changes to local
grey-rock-memory sync /mnt/shared/grey-rock-memory.db --direction pull

# Push local changes to remote
grey-rock-memory sync /mnt/shared/grey-rock-memory.db --direction push

# Bidirectional merge (recommended)
grey-rock-memory sync /mnt/shared/grey-rock-memory.db --direction merge
```

### Automated Sync via Cron

```bash
# Sync every 15 minutes (bidirectional merge)
*/15 * * * * /usr/local/bin/grey-rock-memory --db /var/lib/grey-rock-memory/grey-rock-memory.db sync /mnt/shared/remote-memory.db --direction merge --json >> /var/log/grey-rock-memory-sync.log 2>&1
```

Sync uses the same dedup-safe upsert as regular stores:
- Title+namespace conflicts are resolved by keeping the higher priority
- Tier never downgrades
- Links are synced alongside memories
- Safe to run concurrently from multiple machines (SQLite WAL mode handles locking)

### Sync via sshfs or rsync

If the remote database is on another machine, mount it or copy it first:

```bash
# Option 1: sshfs mount
mkdir -p /mnt/remote-memory
sshfs user@server:/var/lib/grey-rock-memory /mnt/remote-memory
grey-rock-memory sync /mnt/remote-memory/grey-rock-memory.db --direction merge

# Option 2: rsync + sync + rsync
rsync -a server:/var/lib/grey-rock-memory/grey-rock-memory.db /tmp/remote.db
grey-rock-memory sync /tmp/remote.db --direction merge
rsync -a /tmp/remote.db server:/var/lib/grey-rock-memory/grey-rock-memory.db
```

## Auto-Consolidation (Maintenance)

Auto-consolidation groups memories by namespace and primary tag, then merges groups with enough members into a single long-term summary. This reduces memory count and improves recall relevance.

### Manual Run

```bash
# Preview what would be consolidated
grey-rock-memory auto-consolidate --dry-run

# Consolidate all namespaces (groups of 3+)
grey-rock-memory auto-consolidate

# Only short-term memories, minimum 5 per group
grey-rock-memory auto-consolidate --short-only --min-count 5
```

### Cron Schedule

```bash
# Run auto-consolidation daily at 3am, short-term memories only
0 3 * * * /usr/local/bin/grey-rock-memory --db /var/lib/grey-rock-memory/grey-rock-memory.db auto-consolidate --short-only --json >> /var/log/grey-rock-memory-consolidate.log 2>&1
```

## Man Page

Install the man page for system-wide documentation:

```bash
grey-rock-memory man | sudo tee /usr/local/share/man/man1/grey-rock-memory.1 > /dev/null
sudo mandb
man grey-rock-memory
```

## Scaling Considerations

`grey-rock-memory` is designed for single-machine use. It is not a distributed system.

- **Concurrency**: The daemon uses `Arc<Mutex<Connection>>` -- one write at a time, but this is fine for a single-user tool. SQLite WAL mode allows concurrent reads.
- **MCP concurrency**: The MCP server is single-threaded (synchronous stdio loop), one request at a time. This is by design -- Claude Code sends one request at a time.
- **Database size**: SQLite handles databases up to 281 TB. Practically, performance stays excellent up to millions of rows.
- **Memory usage**: Minimal. The daemon holds only the connection and a path in memory. All data is on disk.
- **Multiple instances**: You can run multiple daemons on different ports with different databases. Do not point two daemons at the same database file. The MCP server and CLI can share a database (both use WAL mode).

## Troubleshooting

### Daemon won't start

**Port already in use:**
```bash
ss -tlnp | grep 9077
# Kill the existing process or use a different port
grey-rock-memory serve --port 9078
```

**Database locked:**
```bash
# Remove stale WAL files (only if daemon is not running)
rm -f grey-rock-memory.db-wal grey-rock-memory.db-shm
```

**Permission denied:**
```bash
# Check file permissions
ls -la /path/to/grey-rock-memory.db
# Ensure the user running the daemon has read/write access
```

### MCP server not connecting

**Binary not found:**
Check that the path in `.mcp.json` is correct and the binary is executable.

**Database path issues:**
The MCP server opens the database at the path specified by `--db`. Ensure the directory exists and is writable.

**Protocol errors:**
Check stderr output. The MCP server logs parse errors and protocol issues to stderr.

### Slow queries

If recall or search is slow:

```bash
# Rebuild the FTS index
sqlite3 /path/to/grey-rock-memory.db "INSERT INTO memories_fts(memories_fts) VALUES('rebuild')"

# Compact the database
sqlite3 /path/to/grey-rock-memory.db "VACUUM"
```

### FTS index corruption

Symptoms: search returns no results or errors.

```bash
# Check integrity
sqlite3 /path/to/grey-rock-memory.db "INSERT INTO memories_fts(memories_fts) VALUES('integrity-check')"

# Rebuild if corrupt
sqlite3 /path/to/grey-rock-memory.db "INSERT INTO memories_fts(memories_fts) VALUES('rebuild')"
```

### Database is growing too large

```bash
# Check what's taking space
grey-rock-memory stats

# Delete expired memories
grey-rock-memory gc

# Delete all short-term memories in a namespace
grey-rock-memory forget --tier short --namespace my-app

# Compact after deletion
sqlite3 /path/to/grey-rock-memory.db "VACUUM"
```

## Security

### Localhost Binding

By default, the HTTP daemon binds to `127.0.0.1` only. It is **not accessible from the network**. This is intentional -- `grey-rock-memory` is a local-machine tool.

The MCP server communicates over stdio only -- no network exposure.

### No Authentication

There is no authentication mechanism. This is by design -- the daemon is intended for localhost access only. If you expose it to a network, you are responsible for adding a reverse proxy with authentication.

### Data at Rest

The SQLite database is stored as a regular file. It is not encrypted. If you need encryption at rest, use filesystem-level encryption (LUKS, FileVault, BitLocker).

### Input Validation

All write paths go through the validation layer (`validate.rs`):
- Title: max 512 bytes, no null bytes
- Content: max 64KB, no null bytes
- Namespace: max 128 bytes, no slashes/spaces/nulls
- Source: whitelist (user, claude, hook, api, cli, import, consolidation, system)
- Tags: max 50 tags, each max 128 bytes
- Priority: 1-10
- Confidence: 0.0-1.0, finite
- Relations: whitelist (related_to, supersedes, contradicts, derived_from)
- IDs: max 128 bytes, no null bytes
- Timestamps: valid RFC3339
- TTL: positive, max 1 year

### WAL Files

SQLite WAL mode creates two additional files alongside the database:
- `grey-rock-memory.db-wal` -- write-ahead log
- `grey-rock-memory.db-shm` -- shared memory file

Both are cleaned up on graceful shutdown (the daemon runs `PRAGMA wal_checkpoint(TRUNCATE)` on SIGINT). If the daemon crashes, these files persist but are automatically recovered on next open.
