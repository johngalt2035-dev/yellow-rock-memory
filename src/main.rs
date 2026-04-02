// Grey Rock Memory — Forensic Communication Archive
// Copyright (c) 2026 johngalt2035-dev. All rights reserved.
// Created by johngalt2035-dev + Anthropic Claude AI Code
//
// Licensed under the MIT License. See LICENSE file in the project root.
//
// DISCLAIMER: This software is provided "AS IS", without warranty of any kind.
// Not legal advice. See LEGAL_DISCLAIMER.md for complete terms.

mod color;
mod db;
mod errors;
mod handlers;
mod mcp;
mod models;
mod validate;

use anyhow::{Context, Result};
use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Router,
};
use chrono::{Duration, Utc};
use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::models::Tier;

const DEFAULT_DB: &str = "grey-rock-memory.db";
const DEFAULT_PORT: u16 = 9077;
const GC_INTERVAL_SECS: u64 = 1800;

/// Bearer token auth middleware for PRINCIPAL-ONLY draft endpoints.
/// Rejects all requests unless a valid GRM_API_KEY is configured and
/// the request carries a matching Authorization: Bearer header.
async fn draft_auth_middleware(
    api_key: String,
    req: Request,
    next: Next,
) -> Response {
    // If no API key is configured, ALL draft endpoints are locked out.
    // This is a fail-closed design: missing config = no access.
    if api_key.is_empty() {
        return (
            StatusCode::FORBIDDEN,
            "Draft endpoints require GRM_API_KEY. Set --api-key or GRM_API_KEY env var.",
        )
            .into_response();
    }

    // Extract and validate Bearer token
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(val) if val.starts_with("Bearer ") => {
            let token = &val[7..];
            if token == api_key {
                next.run(req).await
            } else {
                (StatusCode::UNAUTHORIZED, "Invalid API key").into_response()
            }
        }
        _ => (
            StatusCode::UNAUTHORIZED,
            "Authorization: Bearer <GRM_API_KEY> required",
        )
            .into_response(),
    }
}

#[derive(Parser)]
#[command(
    name = "grey-rock-memory",
    about = "Grey Rock memory system — structured message archival, escalation tracking, and logistics extraction"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[arg(long, env = "GREY_ROCK_MEMORY_DB", default_value = DEFAULT_DB, global = true)]
    db: PathBuf,
    /// Output as JSON (machine-parseable)
    #[arg(long, global = true, default_value_t = false)]
    json: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Start the HTTP memory daemon
    Serve(ServeArgs),
    /// Run as an MCP (Model Context Protocol) tool server over stdio
    Mcp,
    /// Store a new memory
    Store(StoreArgs),
    /// Update an existing memory by ID
    Update(UpdateArgs),
    /// Recall memories relevant to a context
    Recall(RecallArgs),
    /// Search memories by text
    Search(SearchArgs),
    /// Retrieve a memory by ID
    Get(GetArgs),
    /// List memories
    List(ListArgs),
    /// Delete a memory by ID
    Delete(DeleteArgs),
    /// Promote a memory to long-term
    Promote(PromoteArgs),
    /// Delete memories matching a pattern
    Forget(ForgetArgs),
    /// Link two memories
    Link(LinkArgs),
    /// Consolidate multiple memories into one
    Consolidate(ConsolidateArgs),
    /// Run garbage collection
    Gc,
    /// Show statistics
    Stats,
    /// List all namespaces
    Namespaces,
    /// Export all memories as JSON
    Export,
    /// Import memories from JSON (stdin)
    Import,
    /// Resolve a contradiction — mark one memory as superseding another
    Resolve(ResolveArgs),
    /// Interactive memory shell (REPL)
    Shell,
    /// Sync memories between two database files
    Sync(SyncArgs),
    /// Auto-consolidate short-term memories by namespace
    AutoConsolidate(AutoConsolidateArgs),
    /// Generate shell completions
    Completions(CompletionsArgs),
    /// Generate man page
    Man,
    /// Train: bulk-import background knowledge from JSON or Markdown files into long-term memory
    Train(TrainArgs),
    /// Generate a logistics digest for a sender (Grey Rock)
    Digest(DigestArgs),
    /// Create a forensic archive of messages with SHA-256 hash chain
    ArchiveMessages(ArchiveMessagesArgs),
    /// Verify a forensic archive's hash integrity
    VerifyArchive(VerifyArchiveArgs),
    /// Import a verified forensic archive back into the database
    ImportArchive(ImportArchiveArgs),
    /// Purge messages older than the retention period (default: 1 year)
    PurgeMessages(PurgeMessagesArgs),
    /// Verify forensic integrity of all messages in the database
    VerifyDb,
}

#[derive(Args)]
struct TrainArgs {
    /// Path to JSON or Markdown file(s) to import. Use - for stdin.
    #[arg(required = true)]
    files: Vec<String>,
    /// Namespace for imported memories
    #[arg(long, short, default_value = "grey-rock")]
    namespace: String,
    /// Source tag for imported memories
    #[arg(long, short = 'S', default_value = "import")]
    source: String,
    /// Base priority for imported memories (1-10)
    #[arg(long, short, default_value_t = 7)]
    priority: i32,
    /// Dry run — show what would be imported without writing
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[derive(Args)]
struct DigestArgs {
    /// Sender to generate digest for
    #[arg(long)]
    sender: String,
    /// Time window (e.g., "24h", "7d", "30d")
    #[arg(long, default_value = "24h")]
    since: String,
}

#[derive(Args)]
struct ArchiveMessagesArgs {
    /// Output file path for the archive JSON
    #[arg(long, short)]
    output: String,
    /// Only archive messages from this sender
    #[arg(long)]
    sender: Option<String>,
    /// Archive messages before this date (default: 1 year ago)
    #[arg(long)]
    before: Option<String>,
    /// Purge archived messages from DB after export
    #[arg(long, default_value_t = false)]
    purge: bool,
}

#[derive(Args)]
struct VerifyArchiveArgs {
    /// Path to the archive JSON file to verify
    file: String,
}

#[derive(Args)]
struct ImportArchiveArgs {
    /// Path to the archive JSON file to import
    file: String,
}

#[derive(Args)]
struct PurgeMessagesArgs {
    /// Purge messages before this date (default: 1 year ago)
    #[arg(long)]
    before: Option<String>,
    /// Only purge messages from this sender
    #[arg(long)]
    sender: Option<String>,
    /// Skip confirmation prompt
    #[arg(long, default_value_t = false)]
    yes: bool,
}

#[derive(Args)]
struct ServeArgs {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value_t = DEFAULT_PORT)]
    port: u16,
    /// API key required for all draft endpoints (PRINCIPAL-ONLY operations).
    /// If not set, draft endpoints return 403 Forbidden.
    #[arg(long, env = "GRM_API_KEY")]
    api_key: Option<String>,
}

#[derive(Args)]
struct StoreArgs {
    #[arg(long, short, default_value = "mid")]
    tier: String,
    #[arg(long, short)]
    namespace: Option<String>,
    #[arg(long, short = 'T')]
    title: String,
    /// Content (use - to read from stdin)
    #[arg(long, short)]
    content: String,
    #[arg(long, default_value = "")]
    tags: String,
    #[arg(long, short, default_value_t = 5)]
    priority: i32,
    /// Confidence 0.0-1.0
    #[arg(long, default_value_t = 1.0)]
    confidence: f64,
    /// Source: user, claude, hook, api
    #[arg(long, short = 'S', default_value = "cli")]
    source: String,
}

#[derive(Args)]
struct UpdateArgs {
    id: String,
    #[arg(long, short = 'T')]
    title: Option<String>,
    #[arg(long, short)]
    content: Option<String>,
    #[arg(long, short)]
    tier: Option<String>,
    #[arg(long, short)]
    namespace: Option<String>,
    #[arg(long)]
    tags: Option<String>,
    #[arg(long, short)]
    priority: Option<i32>,
    #[arg(long)]
    confidence: Option<f64>,
}

#[derive(Args)]
struct RecallArgs {
    context: String,
    #[arg(long, short)]
    namespace: Option<String>,
    #[arg(long, default_value_t = 10)]
    limit: usize,
    #[arg(long)]
    tags: Option<String>,
    #[arg(long)]
    since: Option<String>,
}

#[derive(Args)]
struct SearchArgs {
    query: String,
    #[arg(long, short)]
    namespace: Option<String>,
    #[arg(long, short)]
    tier: Option<String>,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long)]
    since: Option<String>,
    #[arg(long)]
    until: Option<String>,
    #[arg(long)]
    tags: Option<String>,
}

#[derive(Args)]
struct GetArgs {
    id: String,
}

#[derive(Args)]
struct ListArgs {
    #[arg(long, short)]
    namespace: Option<String>,
    #[arg(long, short)]
    tier: Option<String>,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long)]
    since: Option<String>,
    #[arg(long)]
    until: Option<String>,
    #[arg(long)]
    tags: Option<String>,
}

#[derive(Args)]
struct DeleteArgs {
    id: String,
}

#[derive(Args)]
struct PromoteArgs {
    id: String,
}

#[derive(Args)]
struct ForgetArgs {
    #[arg(long, short)]
    namespace: Option<String>,
    #[arg(long, short)]
    pattern: Option<String>,
    #[arg(long, short)]
    tier: Option<String>,
}

#[derive(Args)]
struct LinkArgs {
    source_id: String,
    target_id: String,
    #[arg(long, short, default_value = "related_to")]
    relation: String,
}

#[derive(Args)]
struct ConsolidateArgs {
    /// Comma-separated memory IDs
    ids: String,
    #[arg(long, short = 'T')]
    title: String,
    #[arg(long, short = 's')]
    summary: String,
    #[arg(long, short)]
    namespace: Option<String>,
}

#[derive(Args)]
struct ResolveArgs {
    /// ID of the memory that wins (supersedes)
    winner_id: String,
    /// ID of the memory that loses (superseded)
    loser_id: String,
}

#[derive(Args)]
struct SyncArgs {
    /// Path to the remote database to sync with
    remote_db: PathBuf,
    /// Direction: pull, push, or merge
    #[arg(long, short, default_value = "merge")]
    direction: String,
}

#[derive(Args)]
struct AutoConsolidateArgs {
    /// Namespace to consolidate
    #[arg(long, short)]
    namespace: Option<String>,
    /// Only consolidate short-term memories
    #[arg(long, default_value_t = false)]
    short_only: bool,
    /// Minimum number of memories to trigger consolidation
    #[arg(long, default_value_t = 3)]
    min_count: usize,
    /// Dry run — show what would be consolidated without doing it
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[derive(Args)]
struct CompletionsArgs {
    shell: Shell,
}

fn auto_namespace() -> String {
    // Try git remote name
    if let Ok(out) = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .stderr(std::process::Stdio::null())
        .output()
    {
        let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !url.is_empty() {
            // Extract repo name from URL
            if let Some(name) = url.rsplit('/').next() {
                let name = name.trim_end_matches(".git");
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
    }
    // Fallback to current directory name
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "global".to_string())
}

fn human_age(iso: &str) -> String {
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(iso) else {
        return iso.to_string();
    };
    let dur = Utc::now().signed_duration_since(dt);
    if dur.num_seconds() < 60 {
        return "just now".to_string();
    }
    if dur.num_minutes() < 60 {
        return format!("{}m ago", dur.num_minutes());
    }
    if dur.num_hours() < 24 {
        return format!("{}h ago", dur.num_hours());
    }
    if dur.num_days() < 30 {
        return format!("{}d ago", dur.num_days());
    }
    format!("{}mo ago", dur.num_days() / 30)
}

#[tokio::main]
async fn main() -> Result<()> {
    color::init();
    let cli = Cli::parse();
    let j = cli.json;
    match cli.command {
        Command::Serve(a) => serve(cli.db, a).await,
        Command::Mcp => {
            mcp::run_mcp_server(&cli.db)?;
            Ok(())
        }
        Command::Store(a) => cmd_store(cli.db, a, j),
        Command::Update(a) => cmd_update(cli.db, a, j),
        Command::Recall(a) => cmd_recall(cli.db, a, j),
        Command::Search(a) => cmd_search(cli.db, a, j),
        Command::Get(a) => cmd_get(cli.db, a, j),
        Command::List(a) => cmd_list(cli.db, a, j),
        Command::Delete(a) => cmd_delete(cli.db, a, j),
        Command::Promote(a) => cmd_promote(cli.db, a, j),
        Command::Forget(a) => cmd_forget(cli.db, a, j),
        Command::Link(a) => cmd_link(cli.db, a, j),
        Command::Consolidate(a) => cmd_consolidate(cli.db, a, j),
        Command::Resolve(a) => cmd_resolve(cli.db, a, j),
        Command::Shell => cmd_shell(cli.db),
        Command::Sync(a) => cmd_sync(cli.db, a, j),
        Command::AutoConsolidate(a) => cmd_auto_consolidate(cli.db, a, j),
        Command::Gc => cmd_gc(cli.db, j),
        Command::Stats => cmd_stats(cli.db, j),
        Command::Namespaces => cmd_namespaces(cli.db, j),
        Command::Export => cmd_export(cli.db),
        Command::Import => cmd_import(cli.db, j),
        Command::Completions(a) => {
            generate(
                a.shell,
                &mut Cli::command(),
                "grey-rock-memory",
                &mut std::io::stdout(),
            );
            Ok(())
        }
        Command::Man => {
            let cmd = Cli::command();
            let man = clap_mangen::Man::new(cmd);
            man.render(&mut std::io::stdout())?;
            Ok(())
        }
        Command::Train(args) => train(cli.db, args),
        Command::Digest(args) => digest_cmd(cli.db, args),
        Command::ArchiveMessages(args) => archive_messages_cmd(cli.db, args),
        Command::VerifyArchive(args) => verify_archive_cmd(args),
        Command::ImportArchive(args) => import_archive_cmd(cli.db, args),
        Command::PurgeMessages(args) => purge_messages_cmd(cli.db, args),
        Command::VerifyDb => verify_db_cmd(cli.db),
    }
}

/// Train: bulk-import background knowledge from JSON or Markdown files.
///
/// JSON format: array of objects with {title, content, tags?, priority?, confidence?}
/// Markdown format: H1/H2 headings become titles, content under each heading becomes the memory content.
fn train(db_path: PathBuf, args: TrainArgs) -> Result<()> {
    let conn = db::open(&db_path)?;
    let now = Utc::now();
    let mut imported = 0usize;
    let mut skipped = 0usize;

    for file_path in &args.files {
        let content = if file_path == "-" {
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
            buf
        } else {
            std::fs::read_to_string(file_path)
                .with_context(|| format!("failed to read {}", file_path))?
        };

        let is_json = file_path.ends_with(".json")
            || file_path == "-" && content.trim_start().starts_with('[');
        let is_yaml = file_path.ends_with(".yaml") || file_path.ends_with(".yml");
        let is_markdown = file_path.ends_with(".md") || file_path.ends_with(".markdown");

        if is_json {
            // JSON: array of {title, content, tags?, priority?, confidence?}
            let entries: Vec<serde_json::Value> = serde_json::from_str(&content)
                .with_context(|| format!("failed to parse JSON from {}", file_path))?;

            for entry in &entries {
                let title = entry["title"].as_str().unwrap_or("").trim();
                let body = entry["content"].as_str().unwrap_or("").trim();
                if title.is_empty() || body.is_empty() {
                    skipped += 1;
                    continue;
                }
                let tags: Vec<String> = entry["tags"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let priority = entry["priority"].as_i64().unwrap_or(args.priority as i64) as i32;
                let confidence = entry["confidence"].as_f64().unwrap_or(1.0);

                if args.dry_run {
                    println!(
                        "[DRY RUN] title={:?} tags={:?} priority={} confidence={:.1}",
                        title, tags, priority, confidence
                    );
                    imported += 1;
                    continue;
                }

                let mem = models::Memory {
                    id: uuid::Uuid::new_v4().to_string(),
                    tier: models::Tier::Long,
                    namespace: args.namespace.clone(),
                    title: title.to_string(),
                    content: body.to_string(),
                    tags,
                    priority: priority.clamp(1, 10),
                    confidence: confidence.clamp(0.0, 1.0),
                    source: args.source.clone(),
                    access_count: 0,
                    created_at: now.to_rfc3339(),
                    updated_at: now.to_rfc3339(),
                    last_accessed_at: None,
                    expires_at: None,
                };
                match db::insert(&conn, &mem) {
                    Ok(_) => imported += 1,
                    Err(e) => {
                        eprintln!("  skip {:?}: {}", title, e);
                        skipped += 1;
                    }
                }
            }
        } else if is_yaml {
            // YAML: array of {title, content, tags?, priority?, confidence?}
            let entries: Vec<serde_json::Value> = serde_yaml::from_str(&content)
                .with_context(|| format!("failed to parse YAML from {}", file_path))?;

            for entry in &entries {
                let title = entry["title"].as_str().unwrap_or("").trim();
                let body = entry["content"].as_str().unwrap_or("").trim();
                if title.is_empty() || body.is_empty() {
                    skipped += 1;
                    continue;
                }
                let tags: Vec<String> = entry["tags"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let priority = entry["priority"].as_i64().unwrap_or(args.priority as i64) as i32;
                let confidence = entry["confidence"].as_f64().unwrap_or(1.0);

                if args.dry_run {
                    println!(
                        "[DRY RUN] title={:?} tags={:?} priority={} confidence={:.1}",
                        title, tags, priority, confidence
                    );
                    imported += 1;
                    continue;
                }

                let mem = models::Memory {
                    id: uuid::Uuid::new_v4().to_string(),
                    tier: models::Tier::Long,
                    namespace: args.namespace.clone(),
                    title: title.to_string(),
                    content: body.to_string(),
                    tags,
                    priority: priority.clamp(1, 10),
                    confidence: confidence.clamp(0.0, 1.0),
                    source: args.source.clone(),
                    access_count: 0,
                    created_at: now.to_rfc3339(),
                    updated_at: now.to_rfc3339(),
                    last_accessed_at: None,
                    expires_at: None,
                };
                match db::insert(&conn, &mem) {
                    Ok(_) => imported += 1,
                    Err(e) => {
                        eprintln!("  skip {:?}: {}", title, e);
                        skipped += 1;
                    }
                }
            }
        } else if is_markdown {
            // Markdown: split by H1/H2 headings
            let mut current_title = String::new();
            let mut current_body = String::new();

            for line in content.lines() {
                if line.starts_with("# ") || line.starts_with("## ") {
                    // Flush previous section
                    if !current_title.is_empty() && !current_body.trim().is_empty() {
                        if args.dry_run {
                            println!(
                                "[DRY RUN] title={:?} ({} chars)",
                                current_title,
                                current_body.trim().len()
                            );
                            imported += 1;
                        } else {
                            let mem = models::Memory {
                                id: uuid::Uuid::new_v4().to_string(),
                                tier: models::Tier::Long,
                                namespace: args.namespace.clone(),
                                title: current_title.clone(),
                                content: current_body.trim().to_string(),
                                tags: vec![],
                                priority: args.priority.clamp(1, 10),
                                confidence: 1.0,
                                source: args.source.clone(),
                                access_count: 0,
                                created_at: now.to_rfc3339(),
                                updated_at: now.to_rfc3339(),
                                last_accessed_at: None,
                                expires_at: None,
                            };
                            match db::insert(&conn, &mem) {
                                Ok(_) => imported += 1,
                                Err(e) => {
                                    eprintln!("  skip {:?}: {}", current_title, e);
                                    skipped += 1;
                                }
                            }
                        }
                    }
                    current_title = line.trim_start_matches('#').trim().to_string();
                    current_body.clear();
                } else {
                    current_body.push_str(line);
                    current_body.push('\n');
                }
            }
            // Flush last section
            if !current_title.is_empty() && !current_body.trim().is_empty() {
                if args.dry_run {
                    println!(
                        "[DRY RUN] title={:?} ({} chars)",
                        current_title,
                        current_body.trim().len()
                    );
                    imported += 1;
                } else {
                    let mem = models::Memory {
                        id: uuid::Uuid::new_v4().to_string(),
                        tier: models::Tier::Long,
                        namespace: args.namespace.clone(),
                        title: current_title.clone(),
                        content: current_body.trim().to_string(),
                        tags: vec![],
                        priority: args.priority.clamp(1, 10),
                        confidence: 1.0,
                        source: args.source.clone(),
                        access_count: 0,
                        created_at: now.to_rfc3339(),
                        updated_at: now.to_rfc3339(),
                        last_accessed_at: None,
                        expires_at: None,
                    };
                    match db::insert(&conn, &mem) {
                        Ok(_) => imported += 1,
                        Err(e) => {
                            eprintln!("  skip {:?}: {}", current_title, e);
                            skipped += 1;
                        }
                    }
                }
            }
        } else {
            eprintln!(
                "unknown file type: {} (expected .json, .yaml, .yml, or .md)",
                file_path
            );
            skipped += 1;
        }
    }

    if args.dry_run {
        println!(
            "\n[DRY RUN] Would import {} memories, skipped {}",
            imported, skipped
        );
    } else {
        println!(
            "Imported {} memories into namespace {:?} (skipped {})",
            imported, args.namespace, skipped
        );
    }
    Ok(())
}

/// Generate a logistics digest from the CLI.
fn digest_cmd(db_path: PathBuf, args: DigestArgs) -> Result<()> {
    let conn = db::open(&db_path)?;
    let since = parse_duration_to_rfc3339(&args.since)?;
    let items = db::digest(&conn, Some(&args.sender), None, &since)?;
    let (total, noise) = db::message_total_counts(&conn, Some(&args.sender), None, &since)?;

    if items.is_empty() {
        if total == 0 {
            println!("No interactions with {} since {}", args.sender, args.since);
        } else {
            println!(
                "No logistical items. {} messages received, all noise.",
                total
            );
        }
        return Ok(());
    }

    println!("Digest for {} (last {}):", args.sender, args.since);
    println!(
        "  Total messages: {} ({} noise, {} actionable)",
        total,
        noise,
        items.len()
    );
    println!();
    for item in &items {
        let logistics = item
            .extracted_logistics
            .as_deref()
            .unwrap_or(&item.raw_content);
        println!(
            "  [{}] {} — {}",
            item.category,
            &item.timestamp[..16],
            logistics
        );
    }
    Ok(())
}

fn parse_duration_to_rfc3339(s: &str) -> Result<String> {
    let now = Utc::now();
    let dt = if s.ends_with('h') {
        let hours: i64 = s.trim_end_matches('h').parse()?;
        now - Duration::hours(hours)
    } else if s.ends_with('d') {
        let days: i64 = s.trim_end_matches('d').parse()?;
        now - Duration::days(days)
    } else if s.ends_with('m') && !s.contains('T') {
        let mins: i64 = s.trim_end_matches('m').parse()?;
        now - Duration::minutes(mins)
    } else {
        // Assume RFC3339
        chrono::DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&chrono::Utc))?
    };
    Ok(dt.to_rfc3339())
}

/// Create a forensic archive with SHA-256 hash chain.
fn archive_messages_cmd(db_path: PathBuf, args: ArchiveMessagesArgs) -> Result<()> {
    let conn = db::open(&db_path)?;
    let archive =
        db::create_forensic_archive(&conn, args.sender.as_deref(), args.before.as_deref())?;

    if archive.messages.is_empty() {
        println!("No messages eligible for archival.");
        return Ok(());
    }

    let json = serde_json::to_string_pretty(&archive)?;
    std::fs::write(&args.output, &json)
        .with_context(|| format!("failed to write {}", args.output))?;

    println!("Forensic archive created: {}", args.output);
    println!("  Messages: {}", archive.message_count);
    println!(
        "  Period: {} to {}",
        archive.archive_period.from, archive.archive_period.to
    );
    println!("  Chain hash: {}", &archive.chain_hash[..16]);
    println!("  Archive hash: {}", &archive.archive_hash[..16]);

    if args.purge {
        let deleted = db::purge_messages(
            &conn,
            &archive.archive_period.to,
            args.sender.as_deref(),
            None,
        )?;
        println!("  Purged {} messages from database", deleted);
    }

    Ok(())
}

/// Verify a forensic archive's hash integrity.
fn verify_archive_cmd(args: VerifyArchiveArgs) -> Result<()> {
    let content = std::fs::read_to_string(&args.file)
        .with_context(|| format!("failed to read {}", args.file))?;
    let archive: models::ForensicArchive = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse archive JSON from {}", args.file))?;

    let verification = db::verify_forensic_archive(&archive);

    println!("Forensic Archive Verification: {}", args.file);
    println!("  Messages: {} total", verification.message_count);
    println!("  Verified: {}", verification.messages_verified);
    println!("  Failed: {}", verification.messages_failed);
    println!("  Chain hash valid: {}", verification.chain_hash_valid);
    println!("  Archive hash valid: {}", verification.archive_hash_valid);
    println!(
        "  Overall: {}",
        if verification.valid {
            "VALID"
        } else {
            "INVALID"
        }
    );

    if !verification.failed_ids.is_empty() {
        println!("  Failed message IDs:");
        for id in &verification.failed_ids {
            println!("    - {}", id);
        }
    }

    if verification.valid {
        Ok(())
    } else {
        anyhow::bail!("archive verification FAILED")
    }
}

/// Import a verified forensic archive back into the database.
fn import_archive_cmd(db_path: PathBuf, args: ImportArchiveArgs) -> Result<()> {
    let content = std::fs::read_to_string(&args.file)
        .with_context(|| format!("failed to read {}", args.file))?;
    let archive: models::ForensicArchive = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse archive JSON from {}", args.file))?;

    // Verify first
    let verification = db::verify_forensic_archive(&archive);
    if !verification.valid {
        println!("Archive verification FAILED:");
        println!("  Chain hash valid: {}", verification.chain_hash_valid);
        println!("  Archive hash valid: {}", verification.archive_hash_valid);
        println!("  Failed messages: {}", verification.messages_failed);
        anyhow::bail!("refusing to import — archive failed integrity verification");
    }

    println!(
        "Archive verified: {} messages, all hashes valid.",
        archive.message_count
    );

    let conn = db::open(&db_path)?;
    let (imported, skipped) = db::import_forensic_archive(&conn, &archive)?;
    println!(
        "Imported {} messages, skipped {} (already exist).",
        imported, skipped
    );
    Ok(())
}

/// Purge messages older than the retention period.
fn purge_messages_cmd(db_path: PathBuf, args: PurgeMessagesArgs) -> Result<()> {
    let conn = db::open(&db_path)?;
    let default_before = (Utc::now() - Duration::days(models::MESSAGE_RETENTION_DAYS)).to_rfc3339();
    let before = args.before.unwrap_or(default_before);

    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM messages WHERE timestamp < ?1 AND (?2 IS NULL OR sender = ?2)",
        rusqlite::params![before, args.sender],
        |r| r.get(0),
    )?;

    if count == 0 {
        println!("No messages eligible for purge (before {}).", &before[..10]);
        return Ok(());
    }

    if !args.yes {
        println!(
            "WARNING: This will permanently delete {} messages older than {}.",
            count,
            &before[..10]
        );
        println!(
            "Run with --yes to confirm, or use archive-messages first to create a forensic backup."
        );
        return Ok(());
    }

    let deleted = db::purge_messages(&conn, &before, args.sender.as_deref(), None)?;
    println!("Purged {} messages.", deleted);
    Ok(())
}

/// Verify forensic integrity of all messages in the database.
fn verify_db_cmd(db_path: PathBuf) -> Result<()> {
    let conn = db::open(&db_path)?;
    let (total, verified, failed, failed_ids) = db::verify_db_integrity(&conn)?;

    println!("Forensic Database Integrity Verification");
    println!("  Total messages: {}", total);
    println!("  Verified: {}", verified);
    println!("  Failed: {}", failed);
    println!(
        "  Result: {}",
        if failed == 0 {
            "ALL HASHES VALID"
        } else {
            "INTEGRITY VIOLATION DETECTED"
        }
    );

    if !failed_ids.is_empty() {
        println!("\n  TAMPERED MESSAGE IDs:");
        for id in &failed_ids {
            println!("    - {}", id);
        }
        anyhow::bail!(
            "database integrity verification FAILED — {} message(s) tampered",
            failed
        );
    }

    Ok(())
}

async fn serve(db_path: PathBuf, args: ServeArgs) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("grey_rock_memory=info".parse()?)
                .add_directive("tower_http=info".parse()?),
        )
        .init();

    let conn = db::open(&db_path)?;
    let state: handlers::Db = Arc::new(Mutex::new((conn, db_path.clone())));

    // Automatic GC
    let gc_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(GC_INTERVAL_SECS)).await;
            let lock = gc_state.lock().await;
            match db::gc(&lock.0) {
                Ok(n) if n > 0 => tracing::info!("gc: expired {n} memories"),
                _ => {}
            }
        }
    });

    // Graceful shutdown with WAL checkpoint
    let shutdown_state = state.clone();
    let shutdown = async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutting down — checkpointing WAL");
        let lock = shutdown_state.lock().await;
        let _ = db::checkpoint(&lock.0);
    };

    let app = Router::new()
        .route("/api/v1/health", get(handlers::health))
        .route("/api/v1/memories", get(handlers::list_memories))
        .route("/api/v1/memories", post(handlers::create_memory))
        .route("/api/v1/memories/bulk", post(handlers::bulk_create))
        .route("/api/v1/memories/{id}", get(handlers::get_memory))
        .route("/api/v1/memories/{id}", put(handlers::update_memory))
        .route("/api/v1/memories/{id}", delete(handlers::delete_memory))
        .route("/api/v1/search", get(handlers::search_memories))
        .route("/api/v1/recall", get(handlers::recall_memories_get))
        .route("/api/v1/recall", post(handlers::recall_memories_post))
        .route("/api/v1/forget", post(handlers::forget_memories))
        .route("/api/v1/consolidate", post(handlers::consolidate_memories))
        .route("/api/v1/links", post(handlers::create_link))
        .route(
            "/api/v1/links/{source_id}/{target_id}",
            delete(handlers::delete_link_handler),
        )
        .route("/api/v1/links/{id}", get(handlers::get_links))
        .route("/api/v1/namespaces", get(handlers::list_namespaces))
        .route("/api/v1/stats", get(handlers::get_stats))
        .route("/api/v1/gc", post(handlers::run_gc))
        .route("/api/v1/export", get(handlers::export_memories))
        .route("/api/v1/import", post(handlers::import_memories))
        // Grey Rock: Message Archive, Escalation & Training
        .route("/api/v1/train", post(handlers::train_handler))
        .route("/api/v1/messages", post(handlers::archive_message_handler))
        .route(
            "/api/v1/messages/export",
            get(handlers::export_messages_handler),
        )
        .route(
            "/api/v1/escalation",
            get(handlers::escalation_score_handler),
        )
        .route("/api/v1/digest", get(handlers::digest_handler))
        // Grey Rock: Forensic Archive
        .route("/api/v1/archive", get(handlers::create_archive_handler))
        .route(
            "/api/v1/archive/verify",
            post(handlers::verify_archive_handler),
        )
        .route(
            "/api/v1/archive/import",
            post(handlers::import_archive_handler),
        )
        .route(
            "/api/v1/messages/purge",
            post(handlers::purge_messages_handler),
        )
        .route("/api/v1/messages/verify", get(handlers::verify_db_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    // Grey Rock: Draft Management — PRINCIPAL-ONLY, gated by API key auth.
    // All draft endpoints require Authorization: Bearer <GRM_API_KEY>.
    // Without --api-key / GRM_API_KEY, draft endpoints return 403 Forbidden.
    let api_key_for_middleware = args.api_key.clone().unwrap_or_default();
    let draft_routes = Router::new()
        .route("/api/v1/drafts", post(handlers::create_draft_handler))
        .route("/api/v1/drafts", get(handlers::list_drafts_handler))
        .route("/api/v1/drafts/{id}", get(handlers::get_draft_handler))
        .route(
            "/api/v1/drafts/{id}/approve",
            put(handlers::approve_draft_handler),
        )
        .route(
            "/api/v1/drafts/{id}/reject",
            put(handlers::reject_draft_handler),
        )
        .route(
            "/api/v1/drafts/{id}/send",
            put(handlers::send_draft_handler),
        )
        .route(
            "/api/v1/drafts/{id}/verify",
            get(handlers::verify_draft_handler),
        )
        .layer(middleware::from_fn(move |req, next| {
            let key = api_key_for_middleware.clone();
            draft_auth_middleware(key, req, next)
        }))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let app = app.merge(draft_routes);

    let addr = format!("{}:{}", args.host, args.port);
    tracing::info!("grey-rock-memory listening on {addr}");
    tracing::info!("database: {}", db_path.display());

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

// --- CLI ---

fn cmd_store(db_path: PathBuf, args: StoreArgs, json_out: bool) -> Result<()> {
    let conn = db::open(&db_path)?;
    let tier = Tier::from_str(&args.tier)
        .ok_or_else(|| anyhow::anyhow!("invalid tier: {} (use short, mid, long)", args.tier))?;
    let namespace = args.namespace.unwrap_or_else(auto_namespace);
    let content = if args.content == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        buf
    } else {
        args.content
    };
    let tags: Vec<String> = args
        .tags
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Validate all fields before touching the DB
    validate::validate_title(&args.title)?;
    validate::validate_content(&content)?;
    validate::validate_namespace(&namespace)?;
    validate::validate_source(&args.source)?;
    validate::validate_tags(&tags)?;
    validate::validate_priority(args.priority)?;
    validate::validate_confidence(args.confidence)?;

    let now = Utc::now();
    let expires_at = tier
        .default_ttl_secs()
        .map(|s| (now + Duration::seconds(s)).to_rfc3339());
    let mem = models::Memory {
        id: uuid::Uuid::new_v4().to_string(),
        tier,
        namespace,
        title: args.title,
        content,
        tags,
        priority: args.priority.clamp(1, 10),
        confidence: args.confidence.clamp(0.0, 1.0),
        source: args.source,
        access_count: 0,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        last_accessed_at: None,
        expires_at,
    };
    let contradictions =
        db::find_contradictions(&conn, &mem.title, &mem.namespace).unwrap_or_default();
    let actual_id = db::insert(&conn, &mem)?;
    if json_out {
        let mut j = serde_json::to_value(&mem)?;
        j["id"] = serde_json::json!(actual_id);
        if !contradictions.is_empty() {
            j["potential_contradictions"] =
                serde_json::json!(contradictions.iter().map(|c| &c.id).collect::<Vec<_>>());
        }
        println!("{}", serde_json::to_string(&j)?);
    } else {
        println!(
            "stored: {} [{}] (ns={})",
            actual_id, mem.tier, mem.namespace
        );
        if !contradictions.is_empty() {
            eprintln!(
                "warning: {} similar memories found in same namespace (potential contradictions)",
                contradictions.len()
            );
        }
    }
    Ok(())
}

fn cmd_update(db_path: PathBuf, args: UpdateArgs, json_out: bool) -> Result<()> {
    let conn = db::open(&db_path)?;
    let tier = args.tier.as_deref().and_then(Tier::from_str);
    let tags: Option<Vec<String>> = args.tags.as_ref().map(|t| {
        t.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    });
    // Validate present fields
    if let Some(ref t) = args.title {
        validate::validate_title(t)?;
    }
    if let Some(ref c) = args.content {
        validate::validate_content(c)?;
    }
    if let Some(ref ns) = args.namespace {
        validate::validate_namespace(ns)?;
    }
    if let Some(ref tags) = tags {
        validate::validate_tags(tags)?;
    }
    if let Some(p) = args.priority {
        validate::validate_priority(p)?;
    }
    if let Some(c) = args.confidence {
        validate::validate_confidence(c)?;
    }
    let updated = db::update(
        &conn,
        &args.id,
        args.title.as_deref(),
        args.content.as_deref(),
        tier.as_ref(),
        args.namespace.as_deref(),
        tags.as_ref(),
        args.priority,
        args.confidence,
        None,
    )?;
    if !updated {
        eprintln!("not found: {}", args.id);
        std::process::exit(1);
    }
    if let Some(mem) = db::get(&conn, &args.id)? {
        if json_out {
            println!("{}", serde_json::to_string(&mem)?);
        } else {
            println!("updated: {} [{}]", mem.id, mem.title);
        }
    }
    Ok(())
}

fn cmd_recall(db_path: PathBuf, args: RecallArgs, json_out: bool) -> Result<()> {
    let conn = db::open(&db_path)?;
    let results = db::recall(
        &conn,
        &args.context,
        args.namespace.as_deref(),
        args.limit,
        args.tags.as_deref(),
        args.since.as_deref(),
    )?;
    if json_out {
        println!(
            "{}",
            serde_json::to_string(
                &serde_json::json!({"memories": results, "count": results.len()})
            )?
        );
        return Ok(());
    }
    if results.is_empty() {
        eprintln!("no memories found for: {}", args.context);
        return Ok(());
    }
    for mem in &results {
        let age = human_age(&mem.updated_at);
        let conf = if mem.confidence < 1.0 {
            format!(" conf={:.0}%", mem.confidence * 100.0)
        } else {
            String::new()
        };
        println!(
            "[{}] {} {} (ns={}, {}x, {}{})",
            color::tier_color(mem.tier.as_str(), &format!("{}/{}", mem.tier, &mem.id[..8])),
            color::bold(&mem.title),
            color::priority_bar(mem.priority),
            color::cyan(&mem.namespace),
            mem.access_count,
            color::dim(&age),
            conf
        );
        let preview: String = mem.content.chars().take(200).collect();
        println!("  {}\n", color::dim(&preview));
    }
    println!("{} memory(ies) recalled", results.len());
    Ok(())
}

fn cmd_search(db_path: PathBuf, args: SearchArgs, json_out: bool) -> Result<()> {
    let conn = db::open(&db_path)?;
    let tier = args.tier.as_deref().and_then(Tier::from_str);
    let results = db::search(
        &conn,
        &args.query,
        args.namespace.as_deref(),
        tier.as_ref(),
        args.limit,
        None,
        args.since.as_deref(),
        args.until.as_deref(),
        args.tags.as_deref(),
    )?;
    if json_out {
        println!(
            "{}",
            serde_json::to_string(
                &serde_json::json!({"results": results, "count": results.len()})
            )?
        );
        return Ok(());
    }
    if results.is_empty() {
        eprintln!("no results for: {}", args.query);
        return Ok(());
    }
    for mem in &results {
        let age = human_age(&mem.updated_at);
        println!(
            "[{}/{}] {} (p={}, ns={}, {})",
            mem.tier,
            &mem.id[..8],
            mem.title,
            mem.priority,
            mem.namespace,
            age
        );
    }
    println!("\n{} result(s)", results.len());
    Ok(())
}

fn cmd_get(db_path: PathBuf, args: GetArgs, json_out: bool) -> Result<()> {
    let conn = db::open(&db_path)?;
    match db::get(&conn, &args.id)? {
        Some(mem) => {
            let links = db::get_links(&conn, &args.id).unwrap_or_default();
            if json_out {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({"memory": mem, "links": links}))?
                );
            } else {
                println!("{}", serde_json::to_string_pretty(&mem)?);
                if !links.is_empty() {
                    println!("\nlinks:");
                    for l in &links {
                        println!("  {} --[{}]--> {}", l.source_id, l.relation, l.target_id);
                    }
                }
            }
        }
        None => {
            eprintln!("not found: {}", args.id);
            std::process::exit(1);
        }
    }
    Ok(())
}

fn cmd_list(db_path: PathBuf, args: ListArgs, json_out: bool) -> Result<()> {
    let conn = db::open(&db_path)?;
    let tier = args.tier.as_deref().and_then(Tier::from_str);
    let results = db::list(
        &conn,
        args.namespace.as_deref(),
        tier.as_ref(),
        args.limit,
        0,
        None,
        args.since.as_deref(),
        args.until.as_deref(),
        args.tags.as_deref(),
    )?;
    if json_out {
        println!(
            "{}",
            serde_json::to_string(
                &serde_json::json!({"memories": results, "count": results.len()})
            )?
        );
        return Ok(());
    }
    if results.is_empty() {
        eprintln!("no memories stored");
        return Ok(());
    }
    for mem in &results {
        let age = human_age(&mem.updated_at);
        println!(
            "[{}/{}] {} (p={}, ns={}, {})",
            mem.tier,
            &mem.id[..8],
            mem.title,
            mem.priority,
            mem.namespace,
            age
        );
    }
    println!("\n{} memory(ies)", results.len());
    Ok(())
}

fn cmd_delete(db_path: PathBuf, args: DeleteArgs, json_out: bool) -> Result<()> {
    let conn = db::open(&db_path)?;
    if db::delete(&conn, &args.id)? {
        if json_out {
            println!("{}", serde_json::json!({"deleted": true, "id": args.id}));
        } else {
            println!("deleted: {}", args.id);
        }
    } else {
        eprintln!("not found: {}", args.id);
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_promote(db_path: PathBuf, args: PromoteArgs, json_out: bool) -> Result<()> {
    let conn = db::open(&db_path)?;
    let updated = db::update(
        &conn,
        &args.id,
        None,
        None,
        Some(&Tier::Long),
        None,
        None,
        None,
        None,
        Some(""),
    )?;
    if !updated {
        eprintln!("not found: {}", args.id);
        std::process::exit(1);
    }
    // Clear expires_at for long-term
    conn.execute(
        "UPDATE memories SET expires_at = NULL WHERE id = ?1",
        rusqlite::params![args.id],
    )?;
    if json_out {
        println!(
            "{}",
            serde_json::json!({"promoted": true, "id": args.id, "tier": "long"})
        );
    } else {
        println!("promoted to long-term: {}", args.id);
    }
    Ok(())
}

fn cmd_forget(db_path: PathBuf, args: ForgetArgs, json_out: bool) -> Result<()> {
    let tier = args.tier.as_deref().and_then(Tier::from_str);
    let conn = db::open(&db_path)?;
    match db::forget(
        &conn,
        args.namespace.as_deref(),
        args.pattern.as_deref(),
        tier.as_ref(),
    ) {
        Ok(n) => {
            if json_out {
                println!("{}", serde_json::json!({"deleted": n}));
            } else {
                println!("forgot {} memories", n);
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}

fn cmd_link(db_path: PathBuf, args: LinkArgs, json_out: bool) -> Result<()> {
    validate::validate_link(&args.source_id, &args.target_id, &args.relation)?;
    let conn = db::open(&db_path)?;
    db::create_link(&conn, &args.source_id, &args.target_id, &args.relation)?;
    if json_out {
        println!("{}", serde_json::json!({"linked": true}));
    } else {
        println!(
            "linked: {} --[{}]--> {}",
            args.source_id, args.relation, args.target_id
        );
    }
    Ok(())
}

fn cmd_consolidate(db_path: PathBuf, args: ConsolidateArgs, json_out: bool) -> Result<()> {
    let ids: Vec<String> = args
        .ids
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let namespace = args.namespace.unwrap_or_else(auto_namespace);
    validate::validate_consolidate(&ids, &args.title, &args.summary, &namespace)?;
    let conn = db::open(&db_path)?;
    let new_id = db::consolidate(
        &conn,
        &ids,
        &args.title,
        &args.summary,
        &namespace,
        &Tier::Long,
        "cli",
    )?;
    if json_out {
        println!(
            "{}",
            serde_json::json!({"id": new_id, "consolidated": ids.len()})
        );
    } else {
        println!("consolidated {} memories into: {}", ids.len(), new_id);
    }
    Ok(())
}

fn cmd_gc(db_path: PathBuf, json_out: bool) -> Result<()> {
    let conn = db::open(&db_path)?;
    let count = db::gc(&conn)?;
    if json_out {
        println!("{}", serde_json::json!({"expired_deleted": count}));
    } else {
        println!("expired memories deleted: {}", count);
    }
    Ok(())
}

fn cmd_stats(db_path: PathBuf, json_out: bool) -> Result<()> {
    let conn = db::open(&db_path)?;
    let stats = db::stats(&conn, &db_path)?;
    if json_out {
        println!("{}", serde_json::to_string(&stats)?);
        return Ok(());
    }
    println!("total memories: {}", stats.total);
    println!("expiring within 1h: {}", stats.expiring_soon);
    println!("links: {}", stats.links_count);
    println!("database size: {} bytes", stats.db_size_bytes);
    println!("\nby tier:");
    for t in &stats.by_tier {
        println!("  {}: {}", t.tier, t.count);
    }
    println!("\nby namespace:");
    for ns in &stats.by_namespace {
        println!("  {}: {}", ns.namespace, ns.count);
    }
    Ok(())
}

fn cmd_namespaces(db_path: PathBuf, json_out: bool) -> Result<()> {
    let conn = db::open(&db_path)?;
    let ns = db::list_namespaces(&conn)?;
    if json_out {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"namespaces": ns}))?
        );
        return Ok(());
    }
    if ns.is_empty() {
        eprintln!("no namespaces");
    } else {
        for n in &ns {
            println!("  {}: {} memories", n.namespace, n.count);
        }
    }
    Ok(())
}

fn cmd_export(db_path: PathBuf) -> Result<()> {
    let conn = db::open(&db_path)?;
    let memories = db::export_all(&conn)?;
    let links = db::export_links(&conn)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "memories": memories, "links": links, "count": memories.len(),
            "exported_at": Utc::now().to_rfc3339(),
        }))?
    );
    Ok(())
}

fn cmd_import(db_path: PathBuf, json_out: bool) -> Result<()> {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let data: serde_json::Value = serde_json::from_str(&buf)?;
    let memories: Vec<models::Memory> =
        serde_json::from_value(data.get("memories").cloned().unwrap_or_default())?;
    let links: Vec<models::MemoryLink> =
        serde_json::from_value(data.get("links").cloned().unwrap_or_default()).unwrap_or_default();
    let conn = db::open(&db_path)?;
    let mut imported = 0usize;
    let mut errors = Vec::new();
    for mem in memories {
        if let Err(e) = validate::validate_memory(&mem) {
            errors.push(format!("{}: {}", mem.id, e));
            continue;
        }
        match db::insert(&conn, &mem) {
            Ok(_) => imported += 1,
            Err(e) => errors.push(format!("{}: {}", mem.id, e)),
        }
    }
    for link in links {
        if validate::validate_link(&link.source_id, &link.target_id, &link.relation).is_err() {
            continue;
        }
        let _ = db::create_link(&conn, &link.source_id, &link.target_id, &link.relation);
    }
    if json_out {
        println!(
            "{}",
            serde_json::json!({"imported": imported, "errors": errors})
        );
    } else {
        println!("imported: {}", imported);
        if !errors.is_empty() {
            for e in &errors {
                eprintln!("  {}", e);
            }
        }
    }
    Ok(())
}

fn cmd_resolve(db_path: PathBuf, args: ResolveArgs, json_out: bool) -> Result<()> {
    let conn = db::open(&db_path)?;
    db::create_link(&conn, &args.winner_id, &args.loser_id, "supersedes")?;
    db::update(
        &conn,
        &args.loser_id,
        None,
        None,
        None,
        None,
        None,
        Some(1),
        Some(0.1),
        None,
    )?;
    db::touch(&conn, &args.winner_id)?;
    if json_out {
        println!(
            "{}",
            serde_json::json!({"resolved": true, "winner": args.winner_id, "loser": args.loser_id})
        );
    } else {
        println!(
            "resolved: {} supersedes {}",
            color::long(&args.winner_id),
            color::dim(&args.loser_id)
        );
    }
    Ok(())
}

fn cmd_shell(db_path: PathBuf) -> Result<()> {
    let conn = db::open(&db_path)?;
    println!(
        "{}",
        color::bold("grey-rock-memory shell — type 'help' for commands, 'quit' to exit")
    );
    let stdin = std::io::stdin();
    loop {
        eprint!("{} ", color::cyan("memory>"));
        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        match parts[0] {
            "quit" | "exit" | "q" => break,
            "help" | "h" => {
                println!("  recall <context>    — fuzzy recall");
                println!("  search <query>      — keyword search");
                println!("  list [namespace]    — list memories");
                println!("  get <id>            — show memory details");
                println!("  stats               — show statistics");
                println!("  namespaces          — list namespaces");
                println!("  delete <id>         — delete a memory");
                println!("  quit                — exit shell");
            }
            "recall" | "r" => {
                let ctx = parts[1..].join(" ");
                if ctx.is_empty() {
                    eprintln!("usage: recall <context>");
                    continue;
                }
                match db::recall(&conn, &ctx, None, 10, None, None) {
                    Ok(results) => {
                        for mem in &results {
                            println!(
                                "  [{}] {} {}",
                                color::tier_color(mem.tier.as_str(), mem.tier.as_str()),
                                color::bold(&mem.title),
                                color::priority_bar(mem.priority)
                            );
                            let preview: String = mem.content.chars().take(100).collect();
                            println!("    {}", color::dim(&preview));
                        }
                        println!("  {} result(s)", results.len());
                    }
                    Err(e) => eprintln!("error: {e}"),
                }
            }
            "search" | "s" => {
                let q = parts[1..].join(" ");
                if q.is_empty() {
                    eprintln!("usage: search <query>");
                    continue;
                }
                match db::search(&conn, &q, None, None, 20, None, None, None, None) {
                    Ok(results) => {
                        for mem in &results {
                            println!(
                                "  [{}] {} (p={})",
                                color::tier_color(mem.tier.as_str(), mem.tier.as_str()),
                                mem.title,
                                mem.priority
                            );
                        }
                        println!("  {} result(s)", results.len());
                    }
                    Err(e) => eprintln!("error: {e}"),
                }
            }
            "list" | "ls" => {
                let ns = parts.get(1).copied();
                match db::list(&conn, ns, None, 20, 0, None, None, None, None) {
                    Ok(results) => {
                        for mem in &results {
                            let age = human_age(&mem.updated_at);
                            println!(
                                "  [{}] {} (ns={}, {})",
                                color::tier_color(mem.tier.as_str(), mem.tier.as_str()),
                                mem.title,
                                mem.namespace,
                                color::dim(&age)
                            );
                        }
                        println!("  {} memory(ies)", results.len());
                    }
                    Err(e) => eprintln!("error: {e}"),
                }
            }
            "get" => {
                let id = parts.get(1).unwrap_or(&"");
                if id.is_empty() {
                    eprintln!("usage: get <id>");
                    continue;
                }
                match db::get(&conn, id) {
                    Ok(Some(mem)) => {
                        println!("{}", serde_json::to_string_pretty(&mem).unwrap_or_default())
                    }
                    Ok(None) => eprintln!("not found"),
                    Err(e) => eprintln!("error: {e}"),
                }
            }
            "stats" => match db::stats(&conn, &db_path) {
                Ok(s) => {
                    println!("  total: {}, links: {}", s.total, s.links_count);
                    for t in &s.by_tier {
                        println!("    {}: {}", color::tier_color(&t.tier, &t.tier), t.count);
                    }
                }
                Err(e) => eprintln!("error: {e}"),
            },
            "namespaces" | "ns" => match db::list_namespaces(&conn) {
                Ok(ns) => {
                    for n in &ns {
                        println!("  {}: {}", color::cyan(&n.namespace), n.count);
                    }
                }
                Err(e) => eprintln!("error: {e}"),
            },
            "delete" | "del" | "rm" => {
                let id = parts.get(1).unwrap_or(&"");
                if id.is_empty() {
                    eprintln!("usage: delete <id>");
                    continue;
                }
                match db::delete(&conn, id) {
                    Ok(true) => println!("  deleted"),
                    Ok(false) => eprintln!("  not found"),
                    Err(e) => eprintln!("error: {e}"),
                }
            }
            _ => eprintln!("unknown command: {}. Type 'help' for commands.", parts[0]),
        }
    }
    println!("goodbye");
    Ok(())
}

fn cmd_sync(db_path: PathBuf, args: SyncArgs, json_out: bool) -> Result<()> {
    let local_conn = db::open(&db_path)?;
    let remote_conn = db::open(&args.remote_db)?;
    match args.direction.as_str() {
        "pull" => {
            let mems = db::export_all(&remote_conn)?;
            let links = db::export_links(&remote_conn)?;
            let mut n = 0;
            for mem in &mems {
                if db::insert(&local_conn, mem).is_ok() {
                    n += 1;
                }
            }
            for link in &links {
                let _ = db::create_link(
                    &local_conn,
                    &link.source_id,
                    &link.target_id,
                    &link.relation,
                );
            }
            if json_out {
                println!(
                    "{}",
                    serde_json::json!({"direction": "pull", "imported": n})
                );
            } else {
                println!("pulled {} memories from remote", n);
            }
        }
        "push" => {
            let mems = db::export_all(&local_conn)?;
            let links = db::export_links(&local_conn)?;
            let mut n = 0;
            for mem in &mems {
                if db::insert(&remote_conn, mem).is_ok() {
                    n += 1;
                }
            }
            for link in &links {
                let _ = db::create_link(
                    &remote_conn,
                    &link.source_id,
                    &link.target_id,
                    &link.relation,
                );
            }
            if json_out {
                println!(
                    "{}",
                    serde_json::json!({"direction": "push", "exported": n})
                );
            } else {
                println!("pushed {} memories to remote", n);
            }
        }
        "merge" => {
            let r_mems = db::export_all(&remote_conn)?;
            let r_links = db::export_links(&remote_conn)?;
            let l_mems = db::export_all(&local_conn)?;
            let l_links = db::export_links(&local_conn)?;
            let (mut pulled, mut pushed) = (0, 0);
            for mem in &r_mems {
                if db::insert(&local_conn, mem).is_ok() {
                    pulled += 1;
                }
            }
            for link in &r_links {
                let _ = db::create_link(
                    &local_conn,
                    &link.source_id,
                    &link.target_id,
                    &link.relation,
                );
            }
            for mem in &l_mems {
                if db::insert(&remote_conn, mem).is_ok() {
                    pushed += 1;
                }
            }
            for link in &l_links {
                let _ = db::create_link(
                    &remote_conn,
                    &link.source_id,
                    &link.target_id,
                    &link.relation,
                );
            }
            if json_out {
                println!(
                    "{}",
                    serde_json::json!({"direction": "merge", "pulled": pulled, "pushed": pushed})
                );
            } else {
                println!("merged: pulled {}, pushed {}", pulled, pushed);
            }
        }
        _ => anyhow::bail!(
            "invalid direction: {} (use pull, push, merge)",
            args.direction
        ),
    }
    Ok(())
}

fn cmd_auto_consolidate(db_path: PathBuf, args: AutoConsolidateArgs, json_out: bool) -> Result<()> {
    let conn = db::open(&db_path)?;
    let tier_filter = if args.short_only {
        Some(Tier::Short)
    } else {
        None
    };
    let namespaces = if let Some(ref ns) = args.namespace {
        vec![models::NamespaceCount {
            namespace: ns.clone(),
            count: 0,
        }]
    } else {
        db::list_namespaces(&conn)?
    };

    let mut total = 0;
    let mut groups = Vec::new();

    for ns in &namespaces {
        let memories = db::list(
            &conn,
            Some(&ns.namespace),
            tier_filter.as_ref(),
            200,
            0,
            None,
            None,
            None,
            None,
        )?;
        if memories.len() < args.min_count {
            continue;
        }

        // Group by primary tag
        let mut tag_groups: std::collections::HashMap<String, Vec<&models::Memory>> =
            std::collections::HashMap::new();
        for mem in &memories {
            let key = mem
                .tags
                .first()
                .cloned()
                .unwrap_or_else(|| "_untagged".to_string());
            tag_groups.entry(key).or_default().push(mem);
        }

        for (tag, group) in &tag_groups {
            if group.len() < args.min_count {
                continue;
            }
            let ids: Vec<String> = group.iter().map(|m| m.id.clone()).collect();
            if args.dry_run {
                let titles: Vec<&str> = group.iter().map(|m| m.title.as_str()).collect();
                groups.push(serde_json::json!({"namespace": ns.namespace, "tag": tag, "count": group.len(), "titles": titles}));
            } else {
                let title = format!(
                    "Consolidated: {} ({} memories)",
                    if tag == "_untagged" {
                        &ns.namespace
                    } else {
                        tag
                    },
                    group.len()
                );
                let content: String = group
                    .iter()
                    .map(|m| format!("- {}: {}", m.title, &m.content[..m.content.len().min(200)]))
                    .collect::<Vec<_>>()
                    .join("\n");
                db::consolidate(
                    &conn,
                    &ids,
                    &title,
                    &content,
                    &ns.namespace,
                    &Tier::Long,
                    "auto-consolidate",
                )?;
                total += group.len();
            }
        }
    }

    if json_out {
        if args.dry_run {
            println!("{}", serde_json::json!({"dry_run": true, "groups": groups}));
        } else {
            println!("{}", serde_json::json!({"consolidated": total}));
        }
    } else if args.dry_run {
        println!("dry run — would consolidate:");
        for g in &groups {
            println!(
                "  {} [{}]: {} memories",
                g["namespace"], g["tag"], g["count"]
            );
        }
    } else {
        println!("auto-consolidated {} memories", total);
    }
    Ok(())
}
