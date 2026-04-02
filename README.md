```
                                               _
  __ _ _ __ ___ _   _       _ __ ___   ___ _ __ ___   ___  _ __ _   _
 / _` | '__/ _ \ | | |___  | '_ ` _ \ / _ \ '_ ` _ \ / _ \| '__| | | |
| (_| | | |  __/ |_| |___| | | | | | |  __/ | | | | | (_) | |  | |_| |
 \__, |_|  \___|\__, |     |_| |_| |_|\___|_| |_| |_|\___/|_|   \__, |
 |___/          |___/                                             |___/
```

[![CI](https://github.com/johngalt2035-dev/grey-rock-memory/actions/workflows/ci.yml/badge.svg)](https://github.com/johngalt2035-dev/grey-rock-memory/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![SQLite](https://img.shields.io/badge/sqlite-FTS5-003B57?logo=sqlite)](https://www.sqlite.org/)
[![Tests](https://img.shields.io/badge/tests-33-brightgreen)]()
[![MCP](https://img.shields.io/badge/MCP-11_tools-blueviolet)]()
[![Version](https://img.shields.io/badge/version-3.0.0-orange)]()

**Grey Rock memory system** -- structured message archival, escalation tracking, forensic documentation, and per-contact communication management for Executive Assistants utilizing the Grey Rock communications protocol. Works with any AI provider and any messaging platform (Signal, Telegram, WhatsApp, SMS). Built on SQLite + FTS5.

---

## What Is This?

`grey-rock-memory` is a Rust daemon that provides the memory backbone for AI agents running the [Grey Rock Protocol](https://github.com/johngalt2035-dev/grey-rock-protocol). It handles:

- **Shadow logging** -- archive all incoming messages with category tags (`LOGISTICS`, `NOISE`, `ESCALATION_ALERT`, `ACTION_REQUIRED`)
- **Escalation tracking** -- quantified 1-10 scoring from message patterns (volume, frequency, tone over time windows)
- **Logistics extraction** -- structured digest generation that strips emotional noise
- **Commitment verification** -- FTS5 search to verify "you said X" claims against actual records
- **Legal documentation** -- timestamped exports with chain of provenance for legal admissibility
- **Training facility** -- bulk import background knowledge from JSON or Markdown files
- **Communication style training** -- per-contact personal or executive tone with trainable style data

Integrates with any LLM agent platform via HTTP API, or with Claude Code via MCP tool server.

## Architecture

```
Contact ──Channel──▶ Agent Gateway ──▶ AI Agent (any LLM)
          (Signal/Telegram/WhatsApp/SMS)
                                            │
                                            ▼
                                  grey-rock-memory (port 9077)
                                       │
                                  ┌────┴────┐
                                  │ SQLite  │
                                  │ + FTS5  │
                                  └────┬────┘
                                       │
                         ┌─────────────┼─────────────┐
                         │             │             │
                    memories      messages     messages_fts
                  (knowledge)   (shadow log)    (search)
```

## Quick Start

### Install

```bash
cargo install --path .
# or
cargo build --release && cp target/release/grey-rock-memory /usr/local/bin/
```

### Start the daemon

```bash
grey-rock-memory --db ~/grey-rock-memory.db serve --port 9077
```

### Train with background knowledge

```bash
# From JSON (array of {title, content, tags?, priority?})
grey-rock-memory --db ~/grey-rock-memory.db train background.json

# From Markdown (H1/H2 headings = titles, content underneath = memory)
grey-rock-memory --db ~/grey-rock-memory.db train knowledge.md

# Dry run first
grey-rock-memory --db ~/grey-rock-memory.db train data.json --dry-run

# Multiple files at once
grey-rock-memory --db ~/grey-rock-memory.db train people.json schedule.md triggers.json
```

### Archive a message

```bash
curl -X POST http://localhost:9077/api/v1/messages \
  -H 'Content-Type: application/json' \
  -d '{"sender":"contact-a","raw_content":"Pick up at 5pm","category":"LOGISTICS"}'
```

### Check escalation level

```bash
curl "http://localhost:9077/api/v1/escalation?sender=contact-a"
# Returns: {"score":4,"level":"ROUTINE","count_1h":2,...}
```

### Generate logistics digest

```bash
# Via CLI
grey-rock-memory --db ~/grey-rock-memory.db digest --sender "contact-a" --since 24h

# Via HTTP
curl "http://localhost:9077/api/v1/digest?sender=contact-a&since=2026-03-30T00:00:00Z"
```

### Verify a claim ("you said X")

```bash
grey-rock-memory --db ~/grey-rock-memory.db recall "tuition payment"
```

## MCP Integration

Configure as an MCP tool server for Claude Code:

```json
{
  "mcpServers": {
    "grey-rock": {
      "command": "grey-rock-memory",
      "args": ["--db", "/path/to/grey-rock-memory.db", "mcp"]
    }
  }
}
```

Exposes **11 tools**:

| Tool | Description |
|------|-------------|
| `grm_store` | Store a memory (deduplicates by title+namespace) |
| `grm_recall` | Fuzzy recall relevant to context (ranked by 6-factor scoring) |
| `grm_search` | Exact keyword search (AND semantics) |
| `grm_list` | List memories with filters |
| `grm_delete` | Delete a memory by ID |
| `grm_promote` | Promote to long-term (permanent) |
| `grm_forget` | Bulk delete by pattern/namespace/tier |
| `grm_stats` | Memory store + message statistics |
| `grm_archive_message` | Shadow log a message with category + escalation score |
| `grm_escalation_score` | Compute escalation level from message patterns |
| `grm_digest` | Generate logistics-only digest for time window |

## HTTP API (24 endpoints)

### Core Memory
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/memories` | Create memory |
| GET | `/api/v1/memories` | List memories |
| GET | `/api/v1/memories/{id}` | Get by ID |
| PUT | `/api/v1/memories/{id}` | Update |
| DELETE | `/api/v1/memories/{id}` | Delete |
| POST | `/api/v1/memories/bulk` | Bulk create |
| GET | `/api/v1/search` | FTS5 search |
| GET/POST | `/api/v1/recall` | Fuzzy recall |
| POST | `/api/v1/forget` | Bulk delete |
| POST | `/api/v1/consolidate` | Merge memories |
| POST | `/api/v1/links` | Create link |
| GET | `/api/v1/links/{id}` | Get links |
| DELETE | `/api/v1/links/{s}/{t}` | Delete link |

### Grey Rock
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/train` | Bulk training import |
| POST | `/api/v1/messages` | Archive message |
| GET | `/api/v1/messages/export` | Legal export |
| GET | `/api/v1/escalation` | Escalation score |
| GET | `/api/v1/digest` | Logistics digest |

### System
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/health` | Health check |
| GET | `/api/v1/stats` | Statistics |
| GET | `/api/v1/namespaces` | List namespaces |
| POST | `/api/v1/gc` | Garbage collection |
| GET | `/api/v1/export` | Export all |
| POST | `/api/v1/import` | Import |

## CLI Commands (25)

`mcp` `serve` `store` `update` `recall` `search` `get` `list` `delete` `promote` `forget` `link` `consolidate` `resolve` `shell` `sync` `auto-consolidate` `gc` `stats` `namespaces` `export` `import` `completions` `man` `train` `digest`

## Training Data Format

### JSON

```json
[
  {
    "title": "Contact schedule",
    "content": "Contact works Mon-Fri 9am-5pm. Commute is 25 minutes.",
    "tags": ["schedule", "work"],
    "priority": 8
  },
  {
    "title": "Known escalation triggers",
    "content": "Discussions about finances, schedule changes, holidays.",
    "tags": ["triggers"],
    "priority": 8
  }
]
```

### Markdown

```markdown
# Contact Schedule
Contact works Mon-Fri 9am-5pm. Commute is 25 minutes.

# Known Escalation Triggers
Discussions about finances, schedule changes, holidays.
```

Each H1/H2 heading becomes a memory title. Content under each heading becomes the memory content. All imports go to long-term tier (permanent).

## Deployment (macOS launchd)

```xml
<!-- ~/Library/LaunchAgents/com.example.grey-rock-memory.plist -->
<plist version="1.0">
<dict>
    <key>Label</key><string>com.example.grey-rock-memory</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/grey-rock-memory</string>
        <string>--db</string><string>/path/to/grey-rock-memory.db</string>
        <string>serve</string><string>--port</string><string>9077</string>
    </array>
    <key>RunAtLoad</key><true/>
    <key>KeepAlive</key><true/>
</dict>
</plist>
```

## Memory System

### Three-Tier Architecture
- **Short** (6h TTL) -- ephemeral context, today's messages
- **Mid** (7d TTL) -- working knowledge, this week's patterns
- **Long** (permanent) -- core facts, training data, legal records

### Recall Scoring (6 factors)
```
score = FTS_relevance + (priority * 0.5) + (access_count * 0.1)
      + (confidence * 2.0) + tier_boost + recency_decay
```

### Automatic Behaviors
- TTL extension on recall: short +1h, mid +1d
- Auto-promotion: mid to long at 5 accesses
- Priority reinforcement: +1 every 10 accesses (max 10)
- Contradiction detection on store
- Deduplication: upsert on title+namespace

## Forensic Chain of Provenance

Every message in the database has a `forensic_hash` column (SHA-256) computed at INSERT time. Three-layer verification plus approved draft signing:

### Approved Draft Signing (v6.0 Roadmap)

```
approved_drafts table:
  id                  TEXT PK
  draft_hash          TEXT (SHA-256 of draft content at creation)
  contact_id          TEXT
  incoming_message_id TEXT
  draft_content       TEXT
  approval_hash       TEXT (SHA-256 of draft_hash + reviewer + action + timestamp)
  reviewer_chain      TEXT (JSON array of sequential reviewer actions)
  sent_hash           TEXT (SHA-256 of sent content + send timestamp + channel)
  sent_at             TEXT
  status              TEXT (pending/approved/edited/rejected/sent)
```

### Current: Message Archive Verification

1. **Layer 1 (In-DB)**: `H(id|sender|timestamp|raw_content|category)` per row -- `verify-db` checks all rows
2. **Layer 2 (Archive)**: Per-message hash + chain hash over all messages + archive-level hash
3. **Layer 3 (Cycle)**: archive → verify → purge → reimport → verify -- complete chain of provenance

```bash
grey-rock-memory verify-db                              # Check all DB rows
grey-rock-memory archive-messages -o archive.json       # Export with hashes
grey-rock-memory verify-archive archive.json            # Verify archive integrity
grey-rock-memory import-archive archive.json            # Reimport (rejects tampered)
grey-rock-memory purge-messages --yes                   # Purge after archival
```

Retention: 365 days in SQLite, then archive + purge.
Configurable: set `retention_days` in config for SEC (3-6yr), SOX (7yr), litigation hold, or custom. See [docs/RETENTION.md](docs/RETENTION.md).

## Symbiotic System

Grey Rock Memory is designed to work with [Grey Rock Protocol](https://github.com/johngalt2035-dev/grey-rock-protocol) as a symbiotic system. Both are built upon [OpenClaw](https://openclaw.ai).

| Component | Role |
|---|---|
| **Grey Rock Memory** | Data backbone. Forensic archive. Escalation scoring. Training facility. SHA-256 chain. |
| **[Grey Rock Protocol](https://github.com/johngalt2035-dev/grey-rock-protocol)** | Communication rules. BIFF templates. Anti-JADE/DARVO. Legal framework. |
| **[OpenClaw](https://openclaw.ai)** | Agent orchestration. Channel routing. Cron scheduling. LLM management. |

## Legal

### License

This software is licensed under the [MIT License](LICENSE).

### Disclaimer

**THIS SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND.** The authors accept **zero liability** for any use, misuse, legal outcome, or consequence arising from the use of this software.

**This is not legal advice.** Consult a licensed attorney before using this software in any legal context. The forensic verification features are technical tools only and do not guarantee court admissibility.

**This is not legal advice.** Consult a licensed attorney before using this software in any legal context.

See [LEGAL_DISCLAIMER.md](LEGAL_DISCLAIMER.md) for complete terms including assumption of risk, indemnification, AI-generated communication disclaimers, forensic evidence disclaimers, and jurisdictional notices.

---

*Copyright &copy; 2026 [johngalt2035-dev](https://github.com/johngalt2035-dev). MIT License. Built upon [OpenClaw](https://openclaw.ai). See [LICENSE](LICENSE) and [LEGAL_DISCLAIMER.md](LEGAL_DISCLAIMER.md).*
