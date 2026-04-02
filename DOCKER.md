# Docker Deployment Guide

Deploy Yellow Rock Memory as a Docker container with persistent storage and bearer token authentication.

## Quick Start

```bash
# Clone
git clone https://github.com/johngalt2035-dev/yellow-rock-memory.git
cd yellow-rock-memory

# Configure
cp .env.example .env
# Edit .env — set GRM_API_KEY and GRM_PRINCIPAL_ID

# Launch
docker compose up -d
```

The memory daemon is now running at `http://127.0.0.1:9077`.

## Environment Variables

| Variable | Required | Description |
|---|---|---|
| `GRM_API_KEY` | **Yes** | Bearer token for draft endpoint auth. Generate: `openssl rand -hex 32` |
| `GRM_PRINCIPAL_ID` | **Yes** | Your identifier. Only this identity can approve/reject drafts via MCP. |
| `RUST_LOG` | No | Log level: `trace`, `debug`, `info` (default), `warn`, `error` |

## Build Only

```bash
docker build -t yellow-rock-memory .
```

## Run Standalone (without Compose)

```bash
docker run -d \
  --name yellow-rock-memory \
  -p 127.0.0.1:9077:9077 \
  -e GRM_API_KEY=$(openssl rand -hex 32) \
  -e GRM_PRINCIPAL_ID=principal \
  -v yrm-data:/data \
  yellow-rock-memory
```

## MCP Mode (stdio)

For Claude Code / MCP integration, override the entrypoint:

```bash
docker run -i --rm \
  -e GRM_PRINCIPAL_ID=principal \
  -v yrm-data:/data \
  yellow-rock-memory mcp
```

## Verify

```bash
# Health check
curl http://127.0.0.1:9077/api/v1/health

# Stats
curl http://127.0.0.1:9077/api/v1/stats

# Draft endpoints (require auth)
curl -H "Authorization: Bearer YOUR_GRM_API_KEY" \
  http://127.0.0.1:9077/api/v1/drafts
```

## Data Persistence

The database is stored in a Docker volume (`yrm-data`) mounted at `/data`. Data persists across container restarts and upgrades.

```bash
# Backup
docker run --rm -v yrm-data:/data -v $(pwd):/backup \
  busybox cp /data/yellow-rock-memory.db /backup/yrm-backup.db

# Restore
docker run --rm -v yrm-data:/data -v $(pwd):/backup \
  busybox cp /backup/yrm-backup.db /data/yellow-rock-memory.db
```

## Upgrade

```bash
docker compose down
git pull
docker compose build --no-cache
docker compose up -d
```

Data is preserved in the `yrm-data` volume across upgrades.

## Security Notes

- The container runs as a non-root user (`yrm`)
- HTTP server binds to `0.0.0.0` inside the container but Compose maps it to `127.0.0.1` on the host — not network-accessible by default
- All draft endpoints require `GRM_API_KEY` bearer token — without it, draft endpoints return 403
- To expose to the network, change the port mapping in `docker-compose.yml` (e.g., `"0.0.0.0:9077:9077"`) and ensure TLS termination via a reverse proxy

## Reverse Proxy (Production)

For production deployments, put a reverse proxy (nginx, Caddy, Traefik) in front:

```yaml
# Add to docker-compose.yml
services:
  caddy:
    image: caddy:2
    ports:
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile
    depends_on:
      - memory
```

```
# Caddyfile
memory.yourdomain.com {
    reverse_proxy memory:9077
}
```

## Logs

```bash
docker compose logs -f memory
```

## Stop

```bash
docker compose down        # Stop (data preserved)
docker compose down -v     # Stop and DELETE all data
```
