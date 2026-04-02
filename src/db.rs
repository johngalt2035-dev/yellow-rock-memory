// Yellow Rock Memory — Forensic Communication Archive
// Copyright (c) 2026 johngalt2035-dev. All rights reserved.
// Created by johngalt2035-dev + Anthropic Claude AI Code
//
// Licensed under the MIT License. See LICENSE file in the project root.
//
// DISCLAIMER: This software is provided "AS IS", without warranty of any kind.
// Not legal advice. See LEGAL_DISCLAIMER.md for complete terms.

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::path::Path;

use tracing;

use crate::models::*;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS memories (
    id               TEXT PRIMARY KEY,
    tier             TEXT NOT NULL,
    namespace        TEXT NOT NULL DEFAULT 'global',
    title            TEXT NOT NULL,
    content          TEXT NOT NULL,
    tags             TEXT NOT NULL DEFAULT '[]',
    priority         INTEGER NOT NULL DEFAULT 5,
    confidence       REAL NOT NULL DEFAULT 1.0,
    source           TEXT NOT NULL DEFAULT 'api',
    access_count     INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,
    last_accessed_at TEXT,
    expires_at       TEXT
);

CREATE INDEX IF NOT EXISTS idx_memories_tier ON memories(tier);
CREATE INDEX IF NOT EXISTS idx_memories_namespace ON memories(namespace);
CREATE INDEX IF NOT EXISTS idx_memories_priority ON memories(priority DESC);
CREATE INDEX IF NOT EXISTS idx_memories_expires ON memories(expires_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_memories_title_ns ON memories(title, namespace);

CREATE TABLE IF NOT EXISTS memory_links (
    source_id   TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    target_id   TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    relation    TEXT NOT NULL DEFAULT 'related_to',
    created_at  TEXT NOT NULL,
    PRIMARY KEY (source_id, target_id, relation)
);

CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    title,
    content,
    tags,
    content=memories,
    content_rowid=rowid
);

CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, title, content, tags)
    VALUES (new.rowid, new.title, new.content, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, title, content, tags)
    VALUES ('delete', old.rowid, old.title, old.content, old.tags);
END;

CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, title, content, tags)
    VALUES ('delete', old.rowid, old.title, old.content, old.tags);
    INSERT INTO memories_fts(rowid, title, content, tags)
    VALUES (new.rowid, new.title, new.content, new.tags);
END;

CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);

-- Yellow Rock: Message archive for shadow logging with forensic integrity
CREATE TABLE IF NOT EXISTS messages (
    id               TEXT PRIMARY KEY,
    sender           TEXT NOT NULL,
    contact_id       TEXT,
    timestamp        TEXT NOT NULL,
    channel          TEXT NOT NULL DEFAULT 'signal',
    raw_content      TEXT NOT NULL,
    category         TEXT NOT NULL DEFAULT 'NOISE',
    extracted_logistics TEXT,
    escalation_score INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT NOT NULL,
    forensic_hash    TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender);
CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
CREATE INDEX IF NOT EXISTS idx_messages_category ON messages(category);
CREATE INDEX IF NOT EXISTS idx_messages_sender_ts ON messages(sender, timestamp);
CREATE INDEX IF NOT EXISTS idx_messages_contact_id ON messages(contact_id);

CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    raw_content,
    extracted_logistics,
    content=messages,
    content_rowid=rowid
);

CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, raw_content, extracted_logistics)
    VALUES (new.rowid, new.raw_content, COALESCE(new.extracted_logistics, ''));
END;

CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, raw_content, extracted_logistics)
    VALUES ('delete', old.rowid, old.raw_content, COALESCE(old.extracted_logistics, ''));
END;

-- Yellow Rock: Approved draft tracking with forensic signing chain
CREATE TABLE IF NOT EXISTS approved_drafts (
    id                  TEXT PRIMARY KEY,
    draft_hash          TEXT NOT NULL,
    contact_id          TEXT NOT NULL,
    incoming_message_id TEXT,
    draft_content       TEXT NOT NULL,
    approval_hash       TEXT,
    reviewer_chain      TEXT NOT NULL DEFAULT '[]',
    sent_hash           TEXT,
    sent_at             TEXT,
    sent_channel        TEXT,
    status              TEXT NOT NULL DEFAULT 'pending',
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_drafts_contact ON approved_drafts(contact_id);
CREATE INDEX IF NOT EXISTS idx_drafts_status ON approved_drafts(status);
CREATE INDEX IF NOT EXISTS idx_drafts_created ON approved_drafts(created_at);
"#;

const CURRENT_SCHEMA_VERSION: i64 = 6;

pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path).context("failed to open database")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.execute_batch(SCHEMA)
        .context("failed to initialize schema")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if version < 2 {
        // Add confidence and source columns if missing (v1 -> v2)
        let _ = conn.execute(
            "ALTER TABLE memories ADD COLUMN confidence REAL NOT NULL DEFAULT 1.0",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE memories ADD COLUMN source TEXT NOT NULL DEFAULT 'api'",
            [],
        );
    }
    if version < 3 {
        // v2 -> v3: messages table created via schema
    }
    if version < 4 {
        // v3 -> v4: add forensic_hash column to messages table
        let _ = conn.execute(
            "ALTER TABLE messages ADD COLUMN forensic_hash TEXT NOT NULL DEFAULT ''",
            [],
        );
        // Backfill forensic hashes for existing messages
        let mut stmt = conn.prepare(
            "SELECT id, sender, timestamp, raw_content, category FROM messages WHERE forensic_hash = ''"
        )?;
        let rows: Vec<(String, String, String, String, String)> = stmt
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?
            .filter_map(|r| r.ok())
            .collect();
        for (id, sender, ts, content, cat) in &rows {
            let hash = compute_message_hash(id, sender, ts, content, cat);
            let _ = conn.execute(
                "UPDATE messages SET forensic_hash = ?1 WHERE id = ?2",
                params![hash, id],
            );
        }
    }
    if version < 5 {
        // v4 -> v5: add contact_id column for multi-contact routing
        let _ = conn.execute("ALTER TABLE messages ADD COLUMN contact_id TEXT", []);
    }
    if version < 6 {
        // v5 -> v6: approved_drafts table created via schema
        let _ = conn.execute(
            "ALTER TABLE approved_drafts ADD COLUMN sent_channel TEXT",
            [],
        );
    }
    if version < CURRENT_SCHEMA_VERSION {
        conn.execute("DELETE FROM schema_version", [])?;
        conn.execute(
            "INSERT INTO schema_version (version) VALUES (?1)",
            params![CURRENT_SCHEMA_VERSION],
        )?;
    }
    Ok(())
}

fn row_to_memory(row: &rusqlite::Row) -> rusqlite::Result<Memory> {
    let tags_json: String = row.get("tags")?;
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    let tier_str: String = row.get("tier")?;
    let tier = Tier::from_str(&tier_str).unwrap_or(Tier::Mid);
    Ok(Memory {
        id: row.get("id")?,
        tier,
        namespace: row.get("namespace")?,
        title: row.get("title")?,
        content: row.get("content")?,
        tags,
        priority: row.get("priority")?,
        confidence: row.get("confidence").unwrap_or(1.0),
        source: row.get("source").unwrap_or_else(|_| "api".to_string()),
        access_count: row.get("access_count")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        last_accessed_at: row.get("last_accessed_at")?,
        expires_at: row.get("expires_at")?,
    })
}

/// Insert with upsert on title+namespace. Returns the ID (existing or new).
pub fn insert(conn: &Connection, mem: &Memory) -> Result<String> {
    let tags_json = serde_json::to_string(&mem.tags)?;
    conn.execute(
        "INSERT INTO memories (id, tier, namespace, title, content, tags, priority, confidence, source, access_count, created_at, updated_at, last_accessed_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
         ON CONFLICT(title, namespace) DO UPDATE SET
            content = excluded.content,
            tags = excluded.tags,
            priority = MAX(memories.priority, excluded.priority),
            confidence = excluded.confidence,
            source = excluded.source,
            tier = CASE WHEN excluded.tier = 'long' THEN 'long'
                        WHEN memories.tier = 'long' THEN 'long'
                        WHEN excluded.tier = 'mid' THEN 'mid'
                        ELSE memories.tier END,
            updated_at = excluded.updated_at,
            expires_at = CASE WHEN excluded.tier = 'long' OR memories.tier = 'long' THEN NULL
                              ELSE COALESCE(excluded.expires_at, memories.expires_at) END",
        params![
            mem.id, mem.tier.as_str(), mem.namespace, mem.title, mem.content,
            tags_json, mem.priority, mem.confidence, mem.source, mem.access_count,
            mem.created_at, mem.updated_at, mem.last_accessed_at, mem.expires_at,
        ],
    )?;
    // Return the actual ID (could be the existing one on conflict)
    let actual_id: String = conn
        .query_row(
            "SELECT id FROM memories WHERE title = ?1 AND namespace = ?2",
            params![mem.title, mem.namespace],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| mem.id.clone());
    Ok(actual_id)
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<Memory>> {
    let mut stmt = conn.prepare("SELECT * FROM memories WHERE id = ?1")?;
    let mut rows = stmt.query_map(params![id], row_to_memory)?;
    match rows.next() {
        Some(Ok(m)) => Ok(Some(m)),
        Some(Err(e)) => Err(e.into()),
        None => Ok(None),
    }
}

/// Bump access count, extend TTL, auto-promote.
pub fn touch(conn: &Connection, id: &str) -> Result<()> {
    let mem = get(conn, id)?;
    let Some(mem) = mem else { return Ok(()) };
    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let new_count = mem.access_count + 1;

    // Extend TTL on access
    let new_expires = match mem.tier {
        Tier::Short => mem
            .expires_at
            .map(|_| (now + chrono::Duration::seconds(SHORT_TTL_EXTEND_SECS)).to_rfc3339()),
        Tier::Mid => mem
            .expires_at
            .map(|_| (now + chrono::Duration::seconds(MID_TTL_EXTEND_SECS)).to_rfc3339()),
        Tier::Long => None,
    };

    conn.execute(
        "UPDATE memories SET access_count = ?1, last_accessed_at = ?2, expires_at = COALESCE(?3, expires_at) WHERE id = ?4",
        params![new_count, now_str, new_expires, id],
    )?;

    // Auto-promote mid → long
    if mem.tier == Tier::Mid && new_count >= PROMOTION_THRESHOLD {
        conn.execute(
            "UPDATE memories SET tier = 'long', expires_at = NULL, updated_at = ?1 WHERE id = ?2 AND tier = 'mid'",
            params![now_str, id],
        )?;
    }

    // Reinforce priority every 10 accesses
    if new_count > 0 && new_count % 10 == 0 && mem.priority < 10 {
        conn.execute(
            "UPDATE memories SET priority = MIN(priority + 1, 10) WHERE id = ?1",
            params![id],
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn update(
    conn: &Connection,
    id: &str,
    title: Option<&str>,
    content: Option<&str>,
    tier: Option<&Tier>,
    namespace: Option<&str>,
    tags: Option<&Vec<String>>,
    priority: Option<i32>,
    confidence: Option<f64>,
    expires_at: Option<&str>,
) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT * FROM memories WHERE id = ?1")?;
    let mut rows = stmt.query_map(params![id], row_to_memory)?;
    let existing = match rows.next() {
        Some(Ok(m)) => m,
        _ => return Ok(false),
    };
    drop(rows);
    drop(stmt);

    let title = title.unwrap_or(&existing.title);
    let content = content.unwrap_or(&existing.content);
    let tier = tier.unwrap_or(&existing.tier);
    let namespace = namespace.unwrap_or(&existing.namespace);
    let tags = tags.unwrap_or(&existing.tags);
    let priority = priority.unwrap_or(existing.priority);
    let confidence = confidence.unwrap_or(existing.confidence);
    let expires_at = expires_at.or(existing.expires_at.as_deref());
    let tags_json = serde_json::to_string(tags)?;
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "UPDATE memories SET tier=?1, namespace=?2, title=?3, content=?4, tags=?5, priority=?6, confidence=?7, updated_at=?8, expires_at=?9
         WHERE id=?10",
        params![tier.as_str(), namespace, title, content, tags_json, priority, confidence, now, expires_at, id],
    )?;
    Ok(true)
}

pub fn delete(conn: &Connection, id: &str) -> Result<bool> {
    let changed = conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
    Ok(changed > 0)
}

/// Forget by pattern — delete memories matching namespace + FTS pattern + tier.
pub fn forget(
    conn: &Connection,
    namespace: Option<&str>,
    pattern: Option<&str>,
    tier: Option<&Tier>,
) -> Result<usize> {
    if pattern.is_none() && namespace.is_none() && tier.is_none() {
        anyhow::bail!("at least one of namespace, pattern, or tier is required");
    }

    // If pattern provided, use FTS to find matching IDs
    if let Some(pat) = pattern {
        let fts_query = sanitize_fts_query(pat, true);
        let tier_str = tier.map(|t| t.as_str().to_string());
        let deleted = conn.execute(
            "DELETE FROM memories WHERE rowid IN (
                SELECT m.rowid FROM memories_fts fts
                JOIN memories m ON m.rowid = fts.rowid
                WHERE memories_fts MATCH ?1
                  AND (?2 IS NULL OR m.namespace = ?2)
                  AND (?3 IS NULL OR m.tier = ?3)
            )",
            params![fts_query, namespace, tier_str],
        )?;
        return Ok(deleted);
    }

    let tier_str = tier.map(|t| t.as_str().to_string());
    let deleted = conn.execute(
        "DELETE FROM memories WHERE (?1 IS NULL OR namespace = ?1) AND (?2 IS NULL OR tier = ?2)",
        params![namespace, tier_str],
    )?;
    Ok(deleted)
}

#[allow(clippy::too_many_arguments)]
pub fn list(
    conn: &Connection,
    namespace: Option<&str>,
    tier: Option<&Tier>,
    limit: usize,
    offset: usize,
    min_priority: Option<i32>,
    since: Option<&str>,
    until: Option<&str>,
    tags_filter: Option<&str>,
) -> Result<Vec<Memory>> {
    let now = Utc::now().to_rfc3339();
    let tier_str = tier.map(|t| t.as_str().to_string());
    let mut stmt = conn.prepare(
        "SELECT * FROM memories
         WHERE (?1 IS NULL OR namespace = ?1)
           AND (?2 IS NULL OR tier = ?2)
           AND (?3 IS NULL OR priority >= ?3)
           AND (expires_at IS NULL OR expires_at > ?4)
           AND (?5 IS NULL OR created_at >= ?5)
           AND (?6 IS NULL OR created_at <= ?6)
           AND (?7 IS NULL OR tags LIKE '%' || ?7 || '%')
         ORDER BY priority DESC, updated_at DESC
         LIMIT ?8 OFFSET ?9",
    )?;
    let rows = stmt.query_map(
        params![
            namespace,
            tier_str,
            min_priority,
            now,
            since,
            until,
            tags_filter,
            limit as i64,
            offset as i64
        ],
        row_to_memory,
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

#[allow(clippy::too_many_arguments)]
pub fn search(
    conn: &Connection,
    query: &str,
    namespace: Option<&str>,
    tier: Option<&Tier>,
    limit: usize,
    min_priority: Option<i32>,
    since: Option<&str>,
    until: Option<&str>,
    tags_filter: Option<&str>,
) -> Result<Vec<Memory>> {
    let now = Utc::now().to_rfc3339();
    let tier_str = tier.map(|t| t.as_str().to_string());
    let fts_query = sanitize_fts_query(query, false);

    let mut stmt = conn.prepare(
        "SELECT m.id, m.tier, m.namespace, m.title, m.content, m.tags, m.priority,
                m.confidence, m.source, m.access_count, m.created_at, m.updated_at,
                m.last_accessed_at, m.expires_at
         FROM memories_fts fts
         JOIN memories m ON m.rowid = fts.rowid
         WHERE memories_fts MATCH ?1
           AND (?2 IS NULL OR m.namespace = ?2)
           AND (?3 IS NULL OR m.tier = ?3)
           AND (?4 IS NULL OR m.priority >= ?4)
           AND (m.expires_at IS NULL OR m.expires_at > ?5)
           AND (?6 IS NULL OR m.created_at >= ?6)
           AND (?7 IS NULL OR m.created_at <= ?7)
           AND (?8 IS NULL OR m.tags LIKE '%' || ?8 || '%')
         ORDER BY (fts.rank * -1)
           + (m.priority * 0.5)
           + (m.access_count * 0.1)
           + (m.confidence * 2.0)
           + (1.0 / (1.0 + (julianday('now') - julianday(m.updated_at)) * 0.1))
           DESC
         LIMIT ?9",
    )?;
    let rows = stmt.query_map(
        params![
            fts_query,
            namespace,
            tier_str,
            min_priority,
            now,
            since,
            until,
            tags_filter,
            limit as i64
        ],
        row_to_memory,
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

/// Recall — fuzzy OR search + touch + auto-promote + TTL extension.
pub fn recall(
    conn: &Connection,
    context: &str,
    namespace: Option<&str>,
    limit: usize,
    tags_filter: Option<&str>,
    since: Option<&str>,
) -> Result<Vec<Memory>> {
    let now = Utc::now().to_rfc3339();
    let fts_query = sanitize_fts_query(context, true);

    let mut stmt = conn.prepare(
        "SELECT m.id, m.tier, m.namespace, m.title, m.content, m.tags, m.priority,
                m.confidence, m.source, m.access_count, m.created_at, m.updated_at,
                m.last_accessed_at, m.expires_at
         FROM memories_fts fts
         JOIN memories m ON m.rowid = fts.rowid
         WHERE memories_fts MATCH ?1
           AND (?2 IS NULL OR m.namespace = ?2)
           AND (m.expires_at IS NULL OR m.expires_at > ?3)
           AND (?4 IS NULL OR m.tags LIKE '%' || ?4 || '%')
           AND (?5 IS NULL OR m.created_at >= ?5)
         ORDER BY
           (fts.rank * -1)
           + (m.priority * 0.5)
           + (m.access_count * 0.1)
           + (m.confidence * 2.0)
           + (CASE m.tier WHEN 'long' THEN 3.0 WHEN 'mid' THEN 1.0 ELSE 0.0 END)
           + (1.0 / (1.0 + (julianday('now') - julianday(m.updated_at)) * 0.1))
           DESC
         LIMIT ?6",
    )?;
    let rows = stmt.query_map(
        params![fts_query, namespace, now, tags_filter, since, limit as i64],
        row_to_memory,
    )?;
    let results: Vec<Memory> = rows.collect::<rusqlite::Result<Vec<_>>>()?;

    // Touch all recalled memories (bumps access, extends TTL, auto-promotes)
    for mem in &results {
        let _ = touch(conn, &mem.id);
    }
    Ok(results)
}

/// Detect potential contradictions: memories in same namespace with similar titles.
pub fn find_contradictions(conn: &Connection, title: &str, namespace: &str) -> Result<Vec<Memory>> {
    let fts_query = sanitize_fts_query(title, true);
    let mut stmt = conn.prepare(
        "SELECT m.id, m.tier, m.namespace, m.title, m.content, m.tags, m.priority,
                m.confidence, m.source, m.access_count, m.created_at, m.updated_at,
                m.last_accessed_at, m.expires_at
         FROM memories_fts fts
         JOIN memories m ON m.rowid = fts.rowid
         WHERE memories_fts MATCH ?1 AND m.namespace = ?2
         ORDER BY fts.rank
         LIMIT 5",
    )?;
    let rows = stmt.query_map(params![fts_query, namespace], row_to_memory)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

// --- Links ---

pub fn create_link(
    conn: &Connection,
    source_id: &str,
    target_id: &str,
    relation: &str,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO memory_links (source_id, target_id, relation, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![source_id, target_id, relation, now],
    )?;
    Ok(())
}

pub fn get_links(conn: &Connection, id: &str) -> Result<Vec<MemoryLink>> {
    let mut stmt = conn.prepare(
        "SELECT source_id, target_id, relation, created_at FROM memory_links
         WHERE source_id = ?1 OR target_id = ?1",
    )?;
    let rows = stmt.query_map(params![id], |row| {
        Ok(MemoryLink {
            source_id: row.get(0)?,
            target_id: row.get(1)?,
            relation: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn delete_link(conn: &Connection, source_id: &str, target_id: &str) -> Result<bool> {
    let changed = conn.execute(
        "DELETE FROM memory_links WHERE source_id = ?1 AND target_id = ?2",
        params![source_id, target_id],
    )?;
    Ok(changed > 0)
}

// --- Consolidation ---

/// Consolidate multiple memories into one. Returns the new memory ID.
/// Deletes the source memories and creates links from new → old (derived_from).
pub fn consolidate(
    conn: &Connection,
    ids: &[String],
    title: &str,
    summary: &str,
    namespace: &str,
    tier: &Tier,
    source: &str,
) -> Result<String> {
    let now = Utc::now().to_rfc3339();
    let new_id = uuid::Uuid::new_v4().to_string();

    // Collect max priority and all tags from source memories
    let mut max_priority = 5i32;
    let mut all_tags: Vec<String> = Vec::new();
    let mut total_access = 0i64;
    for id in ids {
        if let Some(mem) = get(conn, id)? {
            max_priority = max_priority.max(mem.priority);
            all_tags.extend(mem.tags);
            total_access += mem.access_count;
        }
    }
    all_tags.sort();
    all_tags.dedup();
    let tags_json = serde_json::to_string(&all_tags)?;

    conn.execute(
        "INSERT INTO memories (id, tier, namespace, title, content, tags, priority, confidence, source, access_count, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1.0, ?8, ?9, ?10, ?10)",
        params![new_id, tier.as_str(), namespace, title, summary, tags_json, max_priority, source, total_access, now],
    )?;

    // Delete source memories (links cascade)
    for id in ids {
        let _ = delete(conn, id);
    }

    Ok(new_id)
}

fn sanitize_fts_query(input: &str, use_or: bool) -> String {
    let has_operators = input.contains('"')
        || input.contains(" OR ")
        || input.contains(" AND ")
        || input.contains(" NOT ")
        || input.contains('*');
    if has_operators {
        return input.to_string();
    }
    let joiner = if use_or { " OR " } else { " " };
    input
        .split_whitespace()
        .filter(|t| t.len() > 1) // skip single chars
        .map(|token| format!("\"{}\"", token.replace('"', "")))
        .collect::<Vec<_>>()
        .join(joiner)
}

pub fn list_namespaces(conn: &Connection) -> Result<Vec<NamespaceCount>> {
    let now = Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "SELECT namespace, COUNT(*) FROM memories WHERE expires_at IS NULL OR expires_at > ?1 GROUP BY namespace ORDER BY COUNT(*) DESC",
    )?;
    let rows = stmt.query_map(params![now], |row| {
        Ok(NamespaceCount {
            namespace: row.get(0)?,
            count: row.get(1)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn stats(conn: &Connection, db_path: &Path) -> Result<Stats> {
    let total: usize = conn.query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))?;

    let mut stmt =
        conn.prepare("SELECT tier, COUNT(*) FROM memories GROUP BY tier ORDER BY COUNT(*) DESC")?;
    let by_tier = stmt
        .query_map([], |row| {
            Ok(TierCount {
                tier: row.get(0)?,
                count: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut stmt = conn.prepare(
        "SELECT namespace, COUNT(*) FROM memories GROUP BY namespace ORDER BY COUNT(*) DESC",
    )?;
    let by_namespace = stmt
        .query_map([], |row| {
            Ok(NamespaceCount {
                namespace: row.get(0)?,
                count: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let now = Utc::now().to_rfc3339();
    let one_hour = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
    let expiring_soon: usize = conn.query_row(
        "SELECT COUNT(*) FROM memories WHERE expires_at IS NOT NULL AND expires_at > ?1 AND expires_at <= ?2",
        params![now, one_hour], |r| r.get(0),
    )?;

    let links_count: usize = conn
        .query_row("SELECT COUNT(*) FROM memory_links", [], |r| r.get(0))
        .unwrap_or(0);
    let db_size_bytes = std::fs::metadata(db_path).map(|m| m.len()).unwrap_or(0);

    Ok(Stats {
        total,
        by_tier,
        by_namespace,
        expiring_soon,
        links_count,
        db_size_bytes,
    })
}

pub fn gc(conn: &Connection) -> Result<usize> {
    let now = Utc::now().to_rfc3339();
    let deleted = conn.execute(
        "DELETE FROM memories WHERE expires_at IS NOT NULL AND expires_at < ?1",
        params![now],
    )?;
    Ok(deleted)
}

pub fn export_all(conn: &Connection) -> Result<Vec<Memory>> {
    let mut stmt = conn.prepare("SELECT * FROM memories ORDER BY created_at ASC")?;
    let rows = stmt.query_map([], row_to_memory)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub fn export_links(conn: &Connection) -> Result<Vec<MemoryLink>> {
    let mut stmt =
        conn.prepare("SELECT source_id, target_id, relation, created_at FROM memory_links")?;
    let rows = stmt.query_map([], |row| {
        Ok(MemoryLink {
            source_id: row.get(0)?,
            target_id: row.get(1)?,
            relation: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

/// Checkpoint WAL for clean shutdown.
pub fn checkpoint(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "wal_checkpoint", "TRUNCATE")?;
    Ok(())
}

/// Deep health check — verifies DB is accessible and FTS is functional.
pub fn health_check(conn: &Connection) -> Result<bool> {
    let _: i64 = conn.query_row("SELECT COUNT(*) FROM memories", [], |r| r.get(0))?;
    conn.execute(
        "INSERT INTO memories_fts(memories_fts) VALUES('integrity-check')",
        [],
    )?;
    Ok(true)
}

// ============================================================
// Yellow Rock: Message Archive & Escalation Analysis
// ============================================================

/// Archive an incoming message with classification.
#[allow(clippy::too_many_arguments)]
pub fn archive_message(
    conn: &Connection,
    sender: &str,
    contact_id: Option<&str>,
    channel: &str,
    raw_content: &str,
    category: &str,
    extracted_logistics: Option<&str>,
    escalation_score: i32,
) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let forensic_hash = compute_message_hash(&id, sender, &now, raw_content, category);
    conn.execute(
        "INSERT INTO messages (id, sender, contact_id, timestamp, channel, raw_content, category, extracted_logistics, escalation_score, created_at, forensic_hash)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![id, sender, contact_id, now, channel, raw_content, category, extracted_logistics, escalation_score, now, forensic_hash],
    )?;
    Ok(id)
}

/// Get message count by category within a time window. Pass None for all senders/contacts.
pub fn message_category_counts(
    conn: &Connection,
    sender: Option<&str>,
    contact_id: Option<&str>,
    since: &str,
) -> Result<Vec<(String, usize)>> {
    let (filter_col, filter_val) = resolve_contact(sender, contact_id);
    let sql = format!(
        "SELECT category, COUNT(*) FROM messages
         WHERE {filter_col} AND timestamp >= ?2
         GROUP BY category ORDER BY COUNT(*) DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![filter_val, since], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

/// Resolve sender or contact_id to a SQL WHERE clause fragment.
/// Returns (condition_sql, param_value) for use in queries.
/// The condition uses `?1` as the parameter placeholder.
fn resolve_contact(sender: Option<&str>, contact_id: Option<&str>) -> (&'static str, String) {
    if let Some(cid) = contact_id {
        ("contact_id = ?1", cid.to_string())
    } else if let Some(s) = sender {
        ("sender = ?1", s.to_string())
    } else {
        ("1=1", String::new()) // no filter
    }
}

/// Compute escalation score (1-10) from message patterns over time windows.
/// Accepts either sender or contact_id for multi-contact support.
pub fn compute_escalation_score(
    conn: &Connection,
    sender: Option<&str>,
    contact_id: Option<&str>,
) -> Result<EscalationReport> {
    // Determine which field to filter on
    let (filter_col, filter_val) = resolve_contact(sender, contact_id);
    let now = Utc::now();
    let h1 = (now - chrono::Duration::hours(1)).to_rfc3339();
    let h6 = (now - chrono::Duration::hours(6)).to_rfc3339();
    let h24 = (now - chrono::Duration::hours(24)).to_rfc3339();
    let d7 = (now - chrono::Duration::days(7)).to_rfc3339();

    // Message frequency per window
    let count_1h: usize = conn.query_row(
        &format!("SELECT COUNT(*) FROM messages WHERE {filter_col} AND timestamp >= ?2"),
        params![filter_val, h1],
        |r| r.get(0),
    )?;
    let count_6h: usize = conn.query_row(
        &format!("SELECT COUNT(*) FROM messages WHERE {filter_col} AND timestamp >= ?2"),
        params![filter_val, h6],
        |r| r.get(0),
    )?;
    let count_24h: usize = conn.query_row(
        &format!("SELECT COUNT(*) FROM messages WHERE {filter_col} AND timestamp >= ?2"),
        params![filter_val, h24],
        |r| r.get(0),
    )?;
    let count_7d: usize = conn.query_row(
        &format!("SELECT COUNT(*) FROM messages WHERE {filter_col} AND timestamp >= ?2"),
        params![filter_val, d7],
        |r| r.get(0),
    )?;

    // Category distribution (last 24h)
    let noise_24h: usize = conn.query_row(
        &format!("SELECT COUNT(*) FROM messages WHERE {filter_col} AND timestamp >= ?2 AND category = 'NOISE'"),
        params![filter_val, h24], |r| r.get(0),
    ).unwrap_or(0);
    let escalation_24h: usize = conn.query_row(
        &format!("SELECT COUNT(*) FROM messages WHERE {filter_col} AND timestamp >= ?2 AND category = 'ESCALATION_ALERT'"),
        params![filter_val, h24], |r| r.get(0),
    ).unwrap_or(0);

    // Average escalation score from individual messages (last 24h)
    let avg_score: f64 = conn.query_row(
        &format!("SELECT COALESCE(AVG(escalation_score), 0) FROM messages WHERE {filter_col} AND timestamp >= ?2"),
        params![filter_val, h24], |r| r.get(0),
    ).unwrap_or(0.0);

    // Compute composite score (1-10)
    let mut score: f64 = 1.0;

    // Volume factor: high message count in short windows = escalation
    if count_1h >= 10 {
        score += 3.0;
    } else if count_1h >= 5 {
        score += 2.0;
    } else if count_1h >= 3 {
        score += 1.0;
    }

    // Noise ratio: high NOISE% = emotional escalation
    if count_24h > 0 {
        let noise_ratio = noise_24h as f64 / count_24h as f64;
        if noise_ratio > 0.8 {
            score += 2.0;
        } else if noise_ratio > 0.5 {
            score += 1.0;
        }
    }

    // Escalation alerts present
    if escalation_24h >= 3 {
        score += 3.0;
    } else if escalation_24h >= 1 {
        score += 2.0;
    }

    // Average per-message escalation
    score += avg_score * 0.3;

    let score = score.clamp(1.0, 10.0) as i32;

    let level = match score {
        1..=4 => "ROUTINE",
        5..=6 => "ELEVATED",
        7..=8 => "HIGH",
        _ => "CRITICAL",
    };

    Ok(EscalationReport {
        score,
        level: level.to_string(),
        count_1h,
        count_6h,
        count_24h,
        count_7d,
        noise_24h,
        escalation_alerts_24h: escalation_24h,
        avg_message_score: avg_score,
    })
}

/// Generate a logistics-only digest for a time window.
pub fn digest(
    conn: &Connection,
    sender: Option<&str>,
    contact_id: Option<&str>,
    since: &str,
) -> Result<Vec<DigestItem>> {
    let (filter_col, filter_val) = resolve_contact(sender, contact_id);
    let sql = format!(
        "SELECT id, timestamp, raw_content, category, extracted_logistics, escalation_score
         FROM messages
         WHERE {filter_col} AND timestamp >= ?2
           AND (category = 'LOGISTICS' OR category = 'ESCALATION_ALERT' OR category = 'ACTION_REQUIRED')
         ORDER BY timestamp ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![filter_val, since], |row| {
        Ok(DigestItem {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            raw_content: row.get(2)?,
            category: row.get(3)?,
            extracted_logistics: row.get(4)?,
            escalation_score: row.get(5)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

/// Get total message counts for a sender/contact (for digest summary line).
pub fn message_total_counts(
    conn: &Connection,
    sender: Option<&str>,
    contact_id: Option<&str>,
    since: &str,
) -> Result<(usize, usize)> {
    let (filter_col, filter_val) = resolve_contact(sender, contact_id);
    let total: usize = conn.query_row(
        &format!("SELECT COUNT(*) FROM messages WHERE {filter_col} AND timestamp >= ?2"),
        params![filter_val, since],
        |r| r.get(0),
    )?;
    let noise: usize = conn.query_row(
        &format!("SELECT COUNT(*) FROM messages WHERE {filter_col} AND timestamp >= ?2 AND category = 'NOISE'"),
        params![filter_val, since], |r| r.get(0),
    ).unwrap_or(0);
    Ok((total, noise))
}

/// Export all messages for legal documentation (basic).
pub fn export_messages(
    conn: &Connection,
    sender: Option<&str>,
    contact_id: Option<&str>,
    since: Option<&str>,
) -> Result<Vec<DigestItem>> {
    let (filter_col, filter_val) = resolve_contact(sender, contact_id);
    let sql = format!(
        "SELECT id, timestamp, raw_content, category, extracted_logistics, escalation_score
         FROM messages
         WHERE {filter_col}
           AND (?2 IS NULL OR timestamp >= ?2)
         ORDER BY timestamp ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![filter_val, since], |row| {
        Ok(DigestItem {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            raw_content: row.get(2)?,
            category: row.get(3)?,
            extracted_logistics: row.get(4)?,
            escalation_score: row.get(5)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

// ============================================================
// Yellow Rock: Forensic Archive System
// ============================================================

/// Compute SHA-256 hash of a message's forensic fields.
fn compute_message_hash(
    id: &str,
    sender: &str,
    timestamp: &str,
    raw_content: &str,
    category: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(id.as_bytes());
    hasher.update(b"|");
    hasher.update(sender.as_bytes());
    hasher.update(b"|");
    hasher.update(timestamp.as_bytes());
    hasher.update(b"|");
    hasher.update(raw_content.as_bytes());
    hasher.update(b"|");
    hasher.update(category.as_bytes());
    hex::encode(hasher.finalize())
}

/// Compute the chain hash from all individual message hashes in order.
fn compute_chain_hash(messages: &[ForensicMessage]) -> String {
    let mut hasher = Sha256::new();
    for msg in messages {
        hasher.update(msg.hash.as_bytes());
    }
    hex::encode(hasher.finalize())
}

/// Compute the archive-level hash (chain_hash + metadata).
fn compute_archive_hash(chain_hash: &str, created_at: &str, message_count: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(chain_hash.as_bytes());
    hasher.update(b"|");
    hasher.update(created_at.as_bytes());
    hasher.update(b"|");
    hasher.update(message_count.to_string().as_bytes());
    hex::encode(hasher.finalize())
}

/// Create a forensic archive of messages within a time range.
pub fn create_forensic_archive(
    conn: &Connection,
    sender: Option<&str>,
    before: Option<&str>,
) -> Result<ForensicArchive> {
    let default_before = (Utc::now() - chrono::Duration::days(MESSAGE_RETENTION_DAYS)).to_rfc3339();
    let before = before.unwrap_or(&default_before);

    let mut stmt = conn.prepare(
        "SELECT id, sender, contact_id, timestamp, channel, raw_content, category,
                extracted_logistics, escalation_score, created_at
         FROM messages
         WHERE (?1 IS NULL OR sender = ?1)
           AND timestamp < ?2
         ORDER BY timestamp ASC",
    )?;

    let rows = stmt.query_map(params![sender, before], |row| {
        let id: String = row.get(0)?;
        let sender_val: String = row.get(1)?;
        let contact_id_val: Option<String> = row.get(2)?;
        let timestamp: String = row.get(3)?;
        let channel: String = row.get(4)?;
        let raw_content: String = row.get(5)?;
        let category: String = row.get(6)?;
        let extracted_logistics: Option<String> = row.get(7)?;
        let escalation_score: i32 = row.get(8)?;
        let created_at: String = row.get(9)?;
        let hash = compute_message_hash(&id, &sender_val, &timestamp, &raw_content, &category);
        Ok(ForensicMessage {
            id,
            sender: sender_val,
            contact_id: contact_id_val,
            timestamp,
            channel,
            raw_content,
            category,
            extracted_logistics,
            escalation_score,
            created_at,
            hash,
        })
    })?;

    let messages: Vec<ForensicMessage> = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let earliest = messages
        .first()
        .map(|m| m.timestamp.clone())
        .unwrap_or_default();
    let latest = messages
        .last()
        .map(|m| m.timestamp.clone())
        .unwrap_or_default();
    let now = Utc::now().to_rfc3339();
    let chain_hash = compute_chain_hash(&messages);
    let archive_hash = compute_archive_hash(&chain_hash, &now, messages.len());

    Ok(ForensicArchive {
        schema_version: "1.0".to_string(),
        archive_type: "yellow-rock-forensic-archive".to_string(),
        created_at: now,
        archive_period: ArchivePeriod {
            from: earliest,
            to: latest,
        },
        message_count: messages.len(),
        messages,
        chain_hash,
        archive_hash,
    })
}

/// Verify a forensic archive's integrity by recomputing all hashes.
pub fn verify_forensic_archive(archive: &ForensicArchive) -> ArchiveVerification {
    let mut messages_verified = 0usize;
    let mut messages_failed = 0usize;
    let mut failed_ids = Vec::new();

    for msg in &archive.messages {
        let expected = compute_message_hash(
            &msg.id,
            &msg.sender,
            &msg.timestamp,
            &msg.raw_content,
            &msg.category,
        );
        if expected == msg.hash {
            messages_verified += 1;
        } else {
            messages_failed += 1;
            failed_ids.push(msg.id.clone());
        }
    }

    let expected_chain = compute_chain_hash(&archive.messages);
    let chain_hash_valid = expected_chain == archive.chain_hash;
    let expected_archive = compute_archive_hash(
        &archive.chain_hash,
        &archive.created_at,
        archive.message_count,
    );
    let archive_hash_valid = expected_archive == archive.archive_hash;
    let valid = messages_failed == 0 && chain_hash_valid && archive_hash_valid;

    ArchiveVerification {
        valid,
        message_count: archive.message_count,
        messages_verified,
        messages_failed,
        chain_hash_valid,
        archive_hash_valid,
        failed_ids,
    }
}

/// Import messages from a verified forensic archive. Returns (imported, skipped).
pub fn import_forensic_archive(
    conn: &Connection,
    archive: &ForensicArchive,
) -> Result<(usize, usize)> {
    let mut imported = 0usize;
    let mut skipped = 0usize;

    for msg in &archive.messages {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM messages WHERE id = ?1",
                params![msg.id],
                |r| r.get(0),
            )
            .unwrap_or(false);

        if exists {
            skipped += 1;
            continue;
        }

        conn.execute(
            "INSERT INTO messages (id, sender, contact_id, timestamp, channel, raw_content, category, extracted_logistics, escalation_score, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![msg.id, msg.sender, msg.contact_id, msg.timestamp, msg.channel, msg.raw_content,
                    msg.category, msg.extracted_logistics, msg.escalation_score, msg.created_at],
        )?;
        imported += 1;
    }
    Ok((imported, skipped))
}

/// Purge messages older than the specified date. Returns count deleted.
pub fn purge_messages(
    conn: &Connection,
    before: &str,
    sender: Option<&str>,
    contact_id: Option<&str>,
) -> Result<usize> {
    let (filter_col, filter_val) = resolve_contact(sender, contact_id);
    let sql = format!("DELETE FROM messages WHERE timestamp < ?2 AND {filter_col}");
    // Note: ?1 = filter_val, ?2 = before (swapped order to match resolve_contact convention)
    let deleted = conn.execute(&sql, params![filter_val, before])?;
    Ok(deleted)
}

/// Count messages eligible for archival (older than retention period).
pub fn count_archivable_messages(conn: &Connection) -> Result<usize> {
    let cutoff = (Utc::now() - chrono::Duration::days(MESSAGE_RETENTION_DAYS)).to_rfc3339();
    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM messages WHERE timestamp < ?1",
        params![cutoff],
        |r| r.get(0),
    )?;
    Ok(count)
}

/// Verify forensic integrity of ALL messages in the database.
/// Recomputes each row's hash and compares against stored forensic_hash.
/// Returns (total, verified, failed, failed_ids).
pub fn verify_db_integrity(conn: &Connection) -> Result<(usize, usize, usize, Vec<String>)> {
    let mut stmt = conn.prepare(
        "SELECT id, sender, timestamp, raw_content, category, forensic_hash FROM messages ORDER BY timestamp ASC"
    )?;

    let mut total = 0usize;
    let mut verified = 0usize;
    let mut failed = 0usize;
    let mut failed_ids = Vec::new();

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
        ))
    })?;

    for row in rows {
        let (id, sender, timestamp, raw_content, category, stored_hash) = row?;
        total += 1;
        let computed = compute_message_hash(&id, &sender, &timestamp, &raw_content, &category);
        if computed == stored_hash {
            verified += 1;
        } else {
            failed += 1;
            failed_ids.push(id);
        }
    }

    Ok((total, verified, failed, failed_ids))
}

// ============================================================
// Yellow Rock: Approved Draft Management
// ============================================================

/// Compute SHA-256 hash for a draft.
fn compute_draft_hash(id: &str, contact_id: &str, draft_content: &str, timestamp: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(id.as_bytes());
    hasher.update(b"|");
    hasher.update(contact_id.as_bytes());
    hasher.update(b"|");
    hasher.update(draft_content.as_bytes());
    hasher.update(b"|");
    hasher.update(timestamp.as_bytes());
    hex::encode(hasher.finalize())
}

/// Compute approval hash (chains from draft_hash).
fn compute_approval_hash(
    draft_hash: &str,
    reviewer: &str,
    action: &str,
    timestamp: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(draft_hash.as_bytes());
    hasher.update(b"|");
    hasher.update(reviewer.as_bytes());
    hasher.update(b"|");
    hasher.update(action.as_bytes());
    hasher.update(b"|");
    hasher.update(timestamp.as_bytes());
    hex::encode(hasher.finalize())
}

/// Compute sent hash (chains from approval).
fn compute_sent_hash(sent_content: &str, send_timestamp: &str, channel: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(sent_content.as_bytes());
    hasher.update(b"|");
    hasher.update(send_timestamp.as_bytes());
    hasher.update(b"|");
    hasher.update(channel.as_bytes());
    hex::encode(hasher.finalize())
}

/// Create a new draft. Returns (id, draft_hash).
pub fn create_draft(
    conn: &Connection,
    contact_id: &str,
    incoming_message_id: Option<&str>,
    draft_content: &str,
) -> Result<(String, String)> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let draft_hash = compute_draft_hash(&id, contact_id, draft_content, &now);
    conn.execute(
        "INSERT INTO approved_drafts (id, draft_hash, contact_id, incoming_message_id, draft_content, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6, ?6)",
        params![id, draft_hash, contact_id, incoming_message_id, draft_content, now],
    )?;
    Ok((id, draft_hash))
}

/// Approve a draft. Computes approval_hash, updates reviewer_chain.
/// SAFETY: The reviewer MUST be the principal (system owner), NEVER the contact.
/// This function enforces that the reviewer cannot be the draft's contact_id.
pub fn approve_draft(
    conn: &Connection,
    id: &str,
    reviewer: &str,
    reason: Option<&str>,
) -> Result<bool> {
    let now = Utc::now().to_rfc3339();
    // Get current draft
    let (draft_hash, chain_json, contact_id): (String, String, String) = conn.query_row(
        "SELECT draft_hash, reviewer_chain, contact_id FROM approved_drafts WHERE id = ?1 AND status = 'pending'",
        params![id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).map_err(|_| anyhow::anyhow!("operation failed"))?;

    // CRITICAL SAFETY CHECK: reviewer must NOT be the contact (high-conflict recipient)
    if reviewer.eq_ignore_ascii_case(&contact_id) {
        tracing::error!(
            "SECURITY: approve_draft blocked — reviewer '{}' matches contact_id '{}'",
            reviewer, contact_id
        );
        anyhow::bail!("operation denied");
    }

    let approval_hash = compute_approval_hash(&draft_hash, reviewer, "approved", &now);

    // Append to reviewer chain
    let mut chain: Vec<serde_json::Value> = serde_json::from_str(&chain_json).unwrap_or_default();
    chain.push(serde_json::json!({
        "reviewer": reviewer,
        "action": "approved",
        "reason": reason,
        "timestamp": now,
    }));
    let chain_updated = serde_json::to_string(&chain)?;

    conn.execute(
        "UPDATE approved_drafts SET status = 'approved', approval_hash = ?1, reviewer_chain = ?2, updated_at = ?3 WHERE id = ?4",
        params![approval_hash, chain_updated, now, id],
    )?;
    Ok(true)
}

/// Reject a draft. Updates reviewer_chain.
/// SAFETY: The reviewer MUST be the principal (system owner), NEVER the contact.
pub fn reject_draft(
    conn: &Connection,
    id: &str,
    reviewer: &str,
    reason: Option<&str>,
) -> Result<bool> {
    let now = Utc::now().to_rfc3339();
    let (chain_json, contact_id): (String, String) = conn
        .query_row(
            "SELECT reviewer_chain, contact_id FROM approved_drafts WHERE id = ?1 AND status = 'pending'",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| anyhow::anyhow!("operation failed"))?;

    // CRITICAL SAFETY CHECK: reviewer must NOT be the contact (high-conflict recipient)
    if reviewer.eq_ignore_ascii_case(&contact_id) {
        tracing::error!(
            "SECURITY: reject_draft blocked — reviewer '{}' matches contact_id '{}'",
            reviewer, contact_id
        );
        anyhow::bail!("operation denied");
    }

    let mut chain: Vec<serde_json::Value> = serde_json::from_str(&chain_json).unwrap_or_default();
    chain.push(serde_json::json!({
        "reviewer": reviewer,
        "action": "rejected",
        "reason": reason,
        "timestamp": now,
    }));
    let chain_updated = serde_json::to_string(&chain)?;

    conn.execute(
        "UPDATE approved_drafts SET status = 'rejected', reviewer_chain = ?1, updated_at = ?2 WHERE id = ?3",
        params![chain_updated, now, id],
    )?;
    Ok(true)
}

/// Get a single draft by ID.
pub fn get_draft(conn: &Connection, id: &str) -> Result<Option<ApprovedDraft>> {
    let mut stmt = conn.prepare(
        "SELECT id, draft_hash, contact_id, incoming_message_id, draft_content, approval_hash, reviewer_chain, sent_hash, sent_at, sent_channel, status, created_at, updated_at FROM approved_drafts WHERE id = ?1"
    )?;
    let mut rows = stmt.query_map(params![id], row_to_draft)?;
    match rows.next() {
        Some(Ok(d)) => Ok(Some(d)),
        Some(Err(e)) => Err(e.into()),
        None => Ok(None),
    }
}

fn row_to_draft(row: &rusqlite::Row) -> rusqlite::Result<ApprovedDraft> {
    Ok(ApprovedDraft {
        id: row.get(0)?,
        draft_hash: row.get(1)?,
        contact_id: row.get(2)?,
        incoming_message_id: row.get(3)?,
        draft_content: row.get(4)?,
        approval_hash: row.get(5)?,
        reviewer_chain: row.get(6)?,
        sent_hash: row.get(7)?,
        sent_at: row.get(8)?,
        sent_channel: row.get(9)?,
        status: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

/// List drafts with optional filters.
pub fn list_drafts(
    conn: &Connection,
    contact_id: Option<&str>,
    status: Option<&str>,
    since: Option<&str>,
    limit: usize,
) -> Result<Vec<ApprovedDraft>> {
    let mut stmt = conn.prepare(
        "SELECT id, draft_hash, contact_id, incoming_message_id, draft_content, approval_hash, reviewer_chain, sent_hash, sent_at, sent_channel, status, created_at, updated_at
         FROM approved_drafts
         WHERE (?1 IS NULL OR contact_id = ?1)
           AND (?2 IS NULL OR status = ?2)
           AND (?3 IS NULL OR created_at >= ?3)
         ORDER BY created_at DESC
         LIMIT ?4"
    )?;
    let rows = stmt.query_map(
        params![contact_id, status, since, limit as i64],
        row_to_draft,
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

/// Mark a draft as sent. Verifies content match, computes sent_hash.
/// SAFETY: This is a PRINCIPAL-ONLY operation (enforced by HTTP auth middleware).
/// Error messages are sanitized to avoid leaking draft state to unauthorized callers.
pub fn mark_draft_sent(
    conn: &Connection,
    id: &str,
    sent_content: &str,
    channel: &str,
) -> Result<bool> {
    let draft = get_draft(conn, id)?;
    let Some(draft) = draft else {
        anyhow::bail!("operation failed")
    };

    if draft.status != "approved" {
        anyhow::bail!("operation failed: precondition not met");
    }

    // Verify content match — zero deviation
    if draft.draft_content != sent_content {
        anyhow::bail!("operation failed: content mismatch");
    }

    let now = Utc::now().to_rfc3339();
    let sent_hash = compute_sent_hash(sent_content, &now, channel);

    conn.execute(
        "UPDATE approved_drafts SET status = 'sent', sent_hash = ?1, sent_at = ?2, sent_channel = ?3, updated_at = ?2 WHERE id = ?4",
        params![sent_hash, now, channel, id],
    )?;
    Ok(true)
}

/// Verify the complete hash chain for a draft.
pub fn verify_draft_chain(conn: &Connection, id: &str) -> Result<DraftVerification> {
    let draft = get_draft(conn, id)?;
    let Some(draft) = draft else {
        anyhow::bail!("draft not found")
    };

    // Verify draft hash
    let expected_draft = compute_draft_hash(
        &draft.id,
        &draft.contact_id,
        &draft.draft_content,
        &draft.created_at,
    );
    let draft_hash_valid = expected_draft == draft.draft_hash;

    // Verify approval hash (if approved)
    let approval_hash_valid = if let Some(ref approval_hash) = draft.approval_hash {
        let chain: Vec<serde_json::Value> =
            serde_json::from_str(&draft.reviewer_chain).unwrap_or_default();
        if let Some(last) = chain.last() {
            let reviewer = last["reviewer"].as_str().unwrap_or("");
            let action = last["action"].as_str().unwrap_or("");
            let ts = last["timestamp"].as_str().unwrap_or("");
            let expected = compute_approval_hash(&draft.draft_hash, reviewer, action, ts);
            expected == *approval_hash
        } else {
            false
        }
    } else {
        true // No approval yet = OK
    };

    // Verify sent hash (if sent)
    let sent_hash_valid =
        if let (Some(ref sent_hash), Some(ref sent_at)) = (&draft.sent_hash, &draft.sent_at) {
            let channel = draft.sent_channel.as_deref().unwrap_or("signal");
            let expected = compute_sent_hash(&draft.draft_content, sent_at, channel);
            expected == *sent_hash
        } else {
            true // Not sent yet = OK
        };

    let valid = draft_hash_valid && approval_hash_valid && sent_hash_valid;

    Ok(DraftVerification {
        valid,
        draft_hash_valid,
        approval_hash_valid,
        sent_hash_valid,
        status: draft.status,
    })
}
