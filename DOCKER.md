# Docker Deployment Guide

Deploy Yellow Rock Memory as a Docker container with persistent storage and bearer token authentication.

## Prerequisites

- **Docker Engine** + **Docker Compose v2**
- A container runtime: [OrbStack](https://orbstack.dev/) (recommended for macOS), Docker Desktop, or [Colima](https://github.com/abiosoft/colima)

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

## Full Stack: Memory + OpenClaw + Protocol

For a complete Yellow Rock deployment with OpenClaw orchestration and Protocol templates, create a `docker-compose.full.yml`:

```yaml
# Yellow Rock Full Stack — Memory + OpenClaw Gateway + Protocol Templates
#
# Usage:
#   cp .env.example .env          # Configure your keys
#   docker compose -f docker-compose.full.yml up -d

services:
  memory:
    build: .
    container_name: yellow-rock-memory
    restart: unless-stopped
    ports:
      - "127.0.0.1:9077:9077"
    volumes:
      - yrm-data:/data
    environment:
      - GRM_API_KEY=${GRM_API_KEY:?Set GRM_API_KEY in .env}
      - GRM_PRINCIPAL_ID=${GRM_PRINCIPAL_ID:-principal}
      - RUST_LOG=${RUST_LOG:-info}
    healthcheck:
      test: ["CMD", "yellow-rock-memory", "--db", "/data/yellow-rock-memory.db", "stats"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s

  openclaw-gateway:
    image: ${OPENCLAW_IMAGE:-ghcr.io/openclaw/openclaw:latest}
    container_name: openclaw-gateway
    restart: unless-stopped
    ports:
      - "${OPENCLAW_GATEWAY_PORT:-18789}:18789"
      - "${OPENCLAW_BRIDGE_PORT:-18790}:18790"
    volumes:
      - ${OPENCLAW_CONFIG_DIR:-./openclaw-config}:/home/node/.openclaw
      - ${OPENCLAW_WORKSPACE_DIR:-./openclaw-workspace}:/home/node/.openclaw/workspace
      - ../yellow-rock-protocol/templates:/home/node/.openclaw/workspace/yellow-rock-protocol:ro
    environment:
      - HOME=/home/node
      - TERM=xterm-256color
      - OPENCLAW_GATEWAY_TOKEN=${OPENCLAW_GATEWAY_TOKEN:?Set OPENCLAW_GATEWAY_TOKEN in .env}
      - XAI_API_KEY=${XAI_API_KEY:-}
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY:-}
      - OPENAI_API_KEY=${OPENAI_API_KEY:-}
      - TZ=${OPENCLAW_TZ:-UTC}
    command: ["node", "dist/index.js", "gateway", "--bind", "${OPENCLAW_GATEWAY_BIND:-lan}", "--port", "18789"]
    depends_on:
      memory:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "node", "-e", "fetch('http://127.0.0.1:18789/healthz').then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"]
      interval: 30s
      timeout: 5s
      retries: 5
      start_period: 20s

  openclaw-cli:
    image: ${OPENCLAW_IMAGE:-ghcr.io/openclaw/openclaw:latest}
    container_name: openclaw-cli
    network_mode: "service:openclaw-gateway"
    cap_drop:
      - NET_RAW
      - NET_ADMIN
    security_opt:
      - no-new-privileges:true
    environment:
      - HOME=/home/node
      - TERM=xterm-256color
      - OPENCLAW_GATEWAY_TOKEN=${OPENCLAW_GATEWAY_TOKEN:-}
      - BROWSER=echo
      - TZ=${OPENCLAW_TZ:-UTC}
    volumes:
      - ${OPENCLAW_CONFIG_DIR:-./openclaw-config}:/home/node/.openclaw
      - ${OPENCLAW_WORKSPACE_DIR:-./openclaw-workspace}:/home/node/.openclaw/workspace
    entrypoint: ["node", "dist/index.js"]
    profiles:
      - cli
    depends_on:
      - openclaw-gateway

volumes:
  yrm-data:
    driver: local
```

### Full Stack Setup

```bash
# 1. Clone both repos side by side
git clone https://github.com/johngalt2035-dev/yellow-rock-memory.git
git clone https://github.com/johngalt2035-dev/yellow-rock-protocol.git

# 2. Configure
cd yellow-rock-memory
cp .env.example .env
```

Add these variables to `.env`:

```bash
# Yellow Rock Memory (required)
GRM_API_KEY=<run: openssl rand -hex 32>
GRM_PRINCIPAL_ID=principal

# OpenClaw Gateway (required)
OPENCLAW_GATEWAY_TOKEN=<run: openssl rand -hex 32>

# AI Provider — at least one required
XAI_API_KEY=<your xAI/Grok key>
# ANTHROPIC_API_KEY=<your Anthropic key>
# OPENAI_API_KEY=<your OpenAI key>

# Optional
# OPENCLAW_TZ=America/New_York
# OPENCLAW_GATEWAY_PORT=18789
# OPENCLAW_GATEWAY_BIND=lan
# OPENCLAW_CONFIG_DIR=./openclaw-config
# OPENCLAW_WORKSPACE_DIR=./openclaw-workspace
# RUST_LOG=info
```

```bash
# 3. Launch
docker compose -f docker-compose.full.yml up -d
```

### OpenClaw Onboarding

On first launch, OpenClaw needs initial configuration. Run the onboarding wizard:

```bash
# Interactive onboarding (run before first 'up -d', or after stopping the gateway)
docker compose -f docker-compose.full.yml run --rm --no-deps --entrypoint node \
  openclaw-gateway dist/index.js onboard --mode local --no-install-daemon

# Set required gateway config
docker compose -f docker-compose.full.yml run --rm --no-deps --entrypoint node \
  openclaw-gateway dist/index.js config set gateway.mode local
docker compose -f docker-compose.full.yml run --rm --no-deps --entrypoint node \
  openclaw-gateway dist/index.js config set gateway.bind lan

# Now start the stack
docker compose -f docker-compose.full.yml up -d
```

### Configuring OpenClaw to use Yellow Rock Memory

Inside the Docker network, the memory daemon is reachable at `http://memory:9077` (Docker internal DNS). Add this to `openclaw.json` (in your `OPENCLAW_CONFIG_DIR`):

```json5
{
  "agents": {
    "defaults": {
      "memorySearch": {
        "remote": {
          "baseUrl": "http://memory:9077/v1"
        }
      }
    }
  }
}
```

Or set it via the CLI container:

```bash
docker compose -f docker-compose.full.yml run --rm --profile cli openclaw-cli \
  config set agents.defaults.memorySearch.remote.baseUrl "http://memory:9077/v1"
```

### Adding Channels

After the gateway is running, add messaging channels via the CLI container:

```bash
# Telegram
docker compose -f docker-compose.full.yml run --rm --profile cli openclaw-cli \
  channels add --channel telegram --token "YOUR_BOT_TOKEN"

# Signal
docker compose -f docker-compose.full.yml run --rm --profile cli openclaw-cli \
  channels add --channel signal

# WhatsApp (QR code)
docker compose -f docker-compose.full.yml run --rm --profile cli openclaw-cli \
  channels login

# Discord
docker compose -f docker-compose.full.yml run --rm --profile cli openclaw-cli \
  channels add --channel discord --token "YOUR_BOT_TOKEN"
```

Or configure channels directly in `openclaw.json`:

```json5
{
  "channels": {
    "signal": {
      "enabled": true,
      "phoneNumber": "+15551234567",
      "dmPolicy": "allowlist",
      "allowFrom": ["+15559876543"]
    },
    "telegram": {
      "enabled": true,
      "botToken": "YOUR_BOT_TOKEN",
      "dmPolicy": "pairing"
    }
  }
}
```

The gateway hot-reloads channel config changes — no restart needed.

### Agent Workspace and SOUL.md

The Yellow Rock Protocol templates are mounted read-only at `/home/node/.openclaw/workspace/yellow-rock-protocol/`. Agent identity files live in the workspace:

| File | Purpose |
|---|---|
| `SOUL.md` | Persona, tone, boundaries (loaded every session) |
| `AGENTS.md` | Operating instructions and memory guidelines |
| `USER.md` | User profile and preferred address |
| `IDENTITY.md` | Agent name, vibe, emoji |
| `memory/YYYY-MM-DD.md` | Daily memory logs |
| `MEMORY.md` | Curated long-term memory |

Copy the Yellow Rock Protocol SOUL.md into the workspace:

```bash
# From the host (files are bind-mounted)
cp ../yellow-rock-protocol/templates/openclaw/SOUL.md ./openclaw-workspace/SOUL.md

# Edit SOUL.md — replace {{USER_NAME}}, {{CONTACT_NAME}}, {{CHANNEL}}
```

### What Persists

| Data | Location | Mechanism |
|---|---|---|
| Memory database | `yrm-data` Docker volume | Named volume at `/data` |
| OpenClaw config | `OPENCLAW_CONFIG_DIR` | Host bind mount |
| Agent workspaces | `OPENCLAW_WORKSPACE_DIR` | Host bind mount |
| Protocol templates | `../yellow-rock-protocol/` | Host bind mount (read-only) |
| Session transcripts | `OPENCLAW_CONFIG_DIR/agents/*/sessions/` | Host bind mount |

### OpenClaw Control UI

Open `http://127.0.0.1:18789/` in your browser and paste the `OPENCLAW_GATEWAY_TOKEN` into Settings.

```bash
# Get the dashboard URL
docker compose -f docker-compose.full.yml run --rm --profile cli openclaw-cli \
  dashboard --no-open
```

### Full Stack Health Check

```bash
# Memory
curl -fsS http://127.0.0.1:9077/api/v1/health

# OpenClaw liveness
curl -fsS http://127.0.0.1:18789/healthz

# OpenClaw readiness
curl -fsS http://127.0.0.1:18789/readyz

# Deep health (authenticated)
docker compose -f docker-compose.full.yml exec openclaw-gateway \
  node dist/index.js health --token "$OPENCLAW_GATEWAY_TOKEN"
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

## Container Runtime Setup (macOS)

Docker requires a Linux VM on macOS. Recommended options:

| Runtime | Install | Notes |
|---|---|---|
| **OrbStack** (recommended) | `brew install orbstack` | Lightweight, fast startup, no VM image download |
| Docker Desktop | [docker.com/products/docker-desktop](https://www.docker.com/products/docker-desktop/) | Official, requires license for commercial use |
| Colima | `brew install colima docker` then `colima start` | Free, open-source, downloads ~600MB VM image on first start |

## Logs

```bash
docker compose logs -f memory
```

## Stop

```bash
docker compose down        # Stop (data preserved)
docker compose down -v     # Stop and DELETE all data
```
