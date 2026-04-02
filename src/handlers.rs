// Yellow Rock Memory — Forensic Communication Archive
// Copyright (c) 2026 johngalt2035-dev. All rights reserved.
// Created by johngalt2035-dev + Anthropic Claude AI Code
//
// Licensed under the MIT License. See LICENSE file in the project root.
//
// DISCLAIMER: This software is provided "AS IS", without warranty of any kind.
// Not legal advice. See LEGAL_DISCLAIMER.md for complete terms.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{Duration, Utc};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::db;
use crate::errors::MemoryError;
use crate::models::*;
use crate::validate;

pub type Db = Arc<Mutex<(rusqlite::Connection, std::path::PathBuf)>>;

pub async fn health(State(state): State<Db>) -> impl IntoResponse {
    let lock = state.lock().await;
    let ok = db::health_check(&lock.0).unwrap_or(false);
    let code = if ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        code,
        Json(json!({"status": if ok { "ok" } else { "error" }, "service": "yellow-rock-memory"})),
    )
        .into_response()
}

pub async fn create_memory(
    State(state): State<Db>,
    Json(body): Json<CreateMemory>,
) -> impl IntoResponse {
    if let Err(e) = validate::validate_create(&body) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response();
    }
    let now = Utc::now();
    let expires_at = body.expires_at.or_else(|| {
        body.ttl_secs
            .or(body.tier.default_ttl_secs())
            .map(|s| (now + Duration::seconds(s)).to_rfc3339())
    });
    let mem = Memory {
        id: Uuid::new_v4().to_string(),
        tier: body.tier,
        namespace: body.namespace,
        title: body.title,
        content: body.content,
        tags: body.tags,
        priority: body.priority.clamp(1, 10),
        confidence: body.confidence.clamp(0.0, 1.0),
        source: body.source,
        access_count: 0,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        last_accessed_at: None,
        expires_at,
    };
    let lock = state.lock().await;

    // Check for contradictions
    let contradictions =
        db::find_contradictions(&lock.0, &mem.title, &mem.namespace).unwrap_or_default();
    let contradiction_ids: Vec<String> = contradictions
        .iter()
        .filter(|c| c.id != mem.id)
        .map(|c| c.id.clone())
        .collect();

    match db::insert(&lock.0, &mem) {
        Ok(actual_id) => {
            let mut response = json!({"id": actual_id, "tier": mem.tier, "namespace": mem.namespace, "title": mem.title});
            if !contradiction_ids.is_empty() {
                response["potential_contradictions"] = json!(contradiction_ids);
            }
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn get_memory(State(state): State<Db>, Path(id): Path<String>) -> impl IntoResponse {
    let lock = state.lock().await;
    match db::get(&lock.0, &id) {
        Ok(Some(mem)) => {
            let links = db::get_links(&lock.0, &id).unwrap_or_default();
            Json(json!({"memory": mem, "links": links})).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "not found"}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn update_memory(
    State(state): State<Db>,
    Path(id): Path<String>,
    Json(body): Json<UpdateMemory>,
) -> impl IntoResponse {
    if let Err(e) = validate::validate_update(&body) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response();
    }
    let lock = state.lock().await;
    match db::update(
        &lock.0,
        &id,
        body.title.as_deref(),
        body.content.as_deref(),
        body.tier.as_ref(),
        body.namespace.as_deref(),
        body.tags.as_ref(),
        body.priority,
        body.confidence,
        body.expires_at.as_deref(),
    ) {
        Ok(true) => {
            let mem = db::get(&lock.0, &id).ok().flatten();
            Json(json!(mem)).into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({"error": "not found"}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn delete_memory(State(state): State<Db>, Path(id): Path<String>) -> impl IntoResponse {
    let lock = state.lock().await;
    match db::delete(&lock.0, &id) {
        Ok(true) => Json(json!({"deleted": true})).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, Json(json!({"error": "not found"}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn list_memories(
    State(state): State<Db>,
    Query(p): Query<ListQuery>,
) -> impl IntoResponse {
    let lock = state.lock().await;
    let limit = p.limit.unwrap_or(20).min(200);
    match db::list(
        &lock.0,
        p.namespace.as_deref(),
        p.tier.as_ref(),
        limit,
        p.offset.unwrap_or(0),
        p.min_priority,
        p.since.as_deref(),
        p.until.as_deref(),
        p.tags.as_deref(),
    ) {
        Ok(mems) => Json(json!({"memories": mems, "count": mems.len()})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn search_memories(
    State(state): State<Db>,
    Query(p): Query<SearchQuery>,
) -> impl IntoResponse {
    if p.q.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "query is required"})),
        )
            .into_response();
    }
    let lock = state.lock().await;
    let limit = p.limit.unwrap_or(20).min(200);
    match db::search(
        &lock.0,
        &p.q,
        p.namespace.as_deref(),
        p.tier.as_ref(),
        limit,
        p.min_priority,
        p.since.as_deref(),
        p.until.as_deref(),
        p.tags.as_deref(),
    ) {
        Ok(r) => Json(json!({"results": r, "count": r.len(), "query": p.q})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn recall_memories_get(
    State(state): State<Db>,
    Query(p): Query<RecallQuery>,
) -> impl IntoResponse {
    let ctx = p.context.unwrap_or_default();
    if ctx.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "context is required"})),
        )
            .into_response();
    }
    let lock = state.lock().await;
    let limit = p.limit.unwrap_or(10).min(50);
    match db::recall(
        &lock.0,
        &ctx,
        p.namespace.as_deref(),
        limit,
        p.tags.as_deref(),
        p.since.as_deref(),
    ) {
        Ok(r) => Json(json!({"memories": r, "count": r.len()})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn recall_memories_post(
    State(state): State<Db>,
    Json(body): Json<RecallBody>,
) -> impl IntoResponse {
    if body.context.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "context is required"})),
        )
            .into_response();
    }
    let lock = state.lock().await;
    let limit = body.limit.unwrap_or(10).min(50);
    match db::recall(
        &lock.0,
        &body.context,
        body.namespace.as_deref(),
        limit,
        body.tags.as_deref(),
        body.since.as_deref(),
    ) {
        Ok(r) => Json(json!({"memories": r, "count": r.len()})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn forget_memories(
    State(state): State<Db>,
    Json(body): Json<ForgetQuery>,
) -> impl IntoResponse {
    let lock = state.lock().await;
    match db::forget(
        &lock.0,
        body.namespace.as_deref(),
        body.pattern.as_deref(),
        body.tier.as_ref(),
    ) {
        Ok(n) => Json(json!({"deleted": n})).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn list_namespaces(State(state): State<Db>) -> impl IntoResponse {
    let lock = state.lock().await;
    match db::list_namespaces(&lock.0) {
        Ok(ns) => Json(json!({"namespaces": ns})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn create_link(State(state): State<Db>, Json(body): Json<LinkBody>) -> impl IntoResponse {
    if let Err(e) = validate::validate_link(&body.source_id, &body.target_id, &body.relation) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response();
    }
    let lock = state.lock().await;
    match db::create_link(&lock.0, &body.source_id, &body.target_id, &body.relation) {
        Ok(()) => (StatusCode::CREATED, Json(json!({"linked": true}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn get_links(State(state): State<Db>, Path(id): Path<String>) -> impl IntoResponse {
    let lock = state.lock().await;
    match db::get_links(&lock.0, &id) {
        Ok(links) => Json(json!({"links": links})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn get_stats(State(state): State<Db>) -> impl IntoResponse {
    let lock = state.lock().await;
    match db::stats(&lock.0, &lock.1) {
        Ok(s) => {
            let since_24h = (chrono::Utc::now() - Duration::hours(24)).to_rfc3339();
            let msg_categories =
                db::message_category_counts(&lock.0, None, None, &since_24h).unwrap_or_default();
            let archivable = db::count_archivable_messages(&lock.0).unwrap_or(0);
            Json(json!({
                "memories": s,
                "messages_24h": msg_categories.into_iter().collect::<std::collections::HashMap<String, usize>>(),
                "messages_archivable": archivable,
            })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn run_gc(State(state): State<Db>) -> impl IntoResponse {
    let lock = state.lock().await;
    match db::gc(&lock.0) {
        Ok(n) => Json(json!({"expired_deleted": n})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn export_memories(State(state): State<Db>) -> impl IntoResponse {
    let lock = state.lock().await;
    let memories = db::export_all(&lock.0).unwrap_or_default();
    let links = db::export_links(&lock.0).unwrap_or_default();
    Json(json!({"memories": memories, "links": links, "count": memories.len(), "exported_at": Utc::now().to_rfc3339()})).into_response()
}

pub async fn import_memories(
    State(state): State<Db>,
    Json(body): Json<ImportBody>,
) -> impl IntoResponse {
    let lock = state.lock().await;
    let mut imported = 0usize;
    let mut errors = Vec::new();
    for mem in body.memories {
        if let Err(e) = validate::validate_memory(&mem) {
            errors.push(format!("{}: {}", mem.id, e));
            continue;
        }
        match db::insert(&lock.0, &mem) {
            Ok(_) => imported += 1,
            Err(e) => errors.push(format!("{}: {}", mem.id, e)),
        }
    }
    for link in body.links.unwrap_or_default() {
        if validate::validate_link(&link.source_id, &link.target_id, &link.relation).is_err() {
            continue;
        }
        let _ = db::create_link(&lock.0, &link.source_id, &link.target_id, &link.relation);
    }
    Json(json!({"imported": imported, "errors": errors})).into_response()
}

#[derive(serde::Deserialize)]
pub struct ImportBody {
    pub memories: Vec<Memory>,
    #[serde(default)]
    pub links: Option<Vec<MemoryLink>>,
}

#[derive(serde::Deserialize)]
pub struct ConsolidateBody {
    pub ids: Vec<String>,
    pub title: String,
    pub summary: String,
    #[serde(default = "default_ns")]
    pub namespace: String,
    #[serde(default)]
    pub tier: Option<Tier>,
}
fn default_ns() -> String {
    "global".to_string()
}

pub async fn consolidate_memories(
    State(state): State<Db>,
    Json(body): Json<ConsolidateBody>,
) -> impl IntoResponse {
    if let Err(e) =
        validate::validate_consolidate(&body.ids, &body.title, &body.summary, &body.namespace)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response();
    }
    let lock = state.lock().await;
    let tier = body.tier.unwrap_or(Tier::Long);
    match db::consolidate(
        &lock.0,
        &body.ids,
        &body.title,
        &body.summary,
        &body.namespace,
        &tier,
        "consolidation",
    ) {
        Ok(new_id) => (
            StatusCode::CREATED,
            Json(json!({"id": new_id, "consolidated": body.ids.len()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn bulk_create(
    State(state): State<Db>,
    Json(bodies): Json<Vec<CreateMemory>>,
) -> impl IntoResponse {
    let now = Utc::now();
    let lock = state.lock().await;
    let mut created = 0usize;
    let mut errors = Vec::new();
    for body in bodies {
        if let Err(e) = validate::validate_create(&body) {
            errors.push(format!("{}: {}", body.title, e));
            continue;
        }
        let expires_at = body.expires_at.or_else(|| {
            body.ttl_secs
                .or(body.tier.default_ttl_secs())
                .map(|s| (now + Duration::seconds(s)).to_rfc3339())
        });
        let mem = Memory {
            id: Uuid::new_v4().to_string(),
            tier: body.tier,
            namespace: body.namespace,
            title: body.title,
            content: body.content,
            tags: body.tags,
            priority: body.priority.clamp(1, 10),
            confidence: body.confidence.clamp(0.0, 1.0),
            source: body.source,
            access_count: 0,
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
            last_accessed_at: None,
            expires_at,
        };
        match db::insert(&lock.0, &mem) {
            Ok(_) => created += 1,
            Err(e) => errors.push(e.to_string()),
        }
    }
    Json(json!({"created": created, "errors": errors})).into_response()
}

// ============================================================
// Yellow Rock: Message Archive & Escalation Handlers
// ============================================================

pub async fn archive_message_handler(
    State(state): State<Db>,
    Json(body): Json<ArchiveMessage>,
) -> impl IntoResponse {
    if let Err(e) = validate::validate_archive_message(&body) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response();
    }
    let lock = state.lock().await;
    match db::archive_message(
        &lock.0,
        &body.sender,
        body.contact_id.as_deref(),
        &body.channel,
        &body.raw_content,
        &body.category,
        body.extracted_logistics.as_deref(),
        body.escalation_score,
    ) {
        Ok(id) => (
            StatusCode::CREATED,
            Json(json!({"id": id, "category": body.category, "escalation_score": body.escalation_score})),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ).into_response(),
    }
}

pub async fn escalation_score_handler(
    State(state): State<Db>,
    Query(query): Query<EscalationQuery>,
) -> impl IntoResponse {
    let lock = state.lock().await;
    match db::compute_escalation_score(
        &lock.0,
        query.sender.as_deref(),
        query.contact_id.as_deref(),
    ) {
        Ok(report) => Json(json!(report)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn digest_handler(
    State(state): State<Db>,
    Query(query): Query<DigestQuery>,
) -> impl IntoResponse {
    let lock = state.lock().await;
    let items = db::digest(
        &lock.0,
        query.sender.as_deref(),
        query.contact_id.as_deref(),
        &query.since,
    )
    .unwrap_or_default();
    let (total, noise) = db::message_total_counts(
        &lock.0,
        query.sender.as_deref(),
        query.contact_id.as_deref(),
        &query.since,
    )
    .unwrap_or((0, 0));
    Json(json!({
        "items": items,
        "total_messages": total,
        "noise_messages": noise,
        "logistics_items": items.len(),
        "since": query.since,
    }))
    .into_response()
}

pub async fn export_messages_handler(
    State(state): State<Db>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let lock = state.lock().await;
    let sender = params.get("sender").map(|s| s.as_str());
    let contact_id = params.get("contact_id").map(|s| s.as_str());
    let since = params.get("since").map(|s| s.as_str());
    match db::export_messages(&lock.0, sender, contact_id, since) {
        Ok(messages) => {
            Json(json!({"messages": messages, "count": messages.len()})).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Bulk training endpoint — import JSON array of memories as long-term.
pub async fn train_handler(
    State(state): State<Db>,
    Json(body): Json<Vec<serde_json::Value>>,
) -> impl IntoResponse {
    let lock = state.lock().await;
    let now = chrono::Utc::now();
    let mut imported = 0usize;
    let mut errors: Vec<String> = Vec::new();

    for entry in &body {
        let title = match entry["title"].as_str() {
            Some(t) if !t.trim().is_empty() => t.trim(),
            _ => {
                errors.push("missing or empty title".into());
                continue;
            }
        };
        let content = match entry["content"].as_str() {
            Some(c) if !c.trim().is_empty() => c.trim(),
            _ => {
                errors.push(format!("missing content for {:?}", title));
                continue;
            }
        };
        let tags: Vec<String> = entry["tags"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let namespace = entry["namespace"].as_str().unwrap_or("yellow-rock");
        let priority = entry["priority"].as_i64().unwrap_or(7) as i32;
        let confidence = entry["confidence"].as_f64().unwrap_or(1.0);
        let source = entry["source"].as_str().unwrap_or("import");

        let mem = Memory {
            id: uuid::Uuid::new_v4().to_string(),
            tier: Tier::Long,
            namespace: namespace.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            tags,
            priority: priority.clamp(1, 10),
            confidence: confidence.clamp(0.0, 1.0),
            source: source.to_string(),
            access_count: 0,
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
            last_accessed_at: None,
            expires_at: None,
        };
        match db::insert(&lock.0, &mem) {
            Ok(_) => imported += 1,
            Err(e) => errors.push(format!("{}: {}", title, e)),
        }
    }

    Json(json!({
        "imported": imported,
        "errors": errors,
        "total_submitted": body.len(),
    }))
    .into_response()
}

pub async fn delete_link_handler(
    State(state): State<Db>,
    Path((source_id, target_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, MemoryError> {
    if source_id.trim().is_empty() || target_id.trim().is_empty() {
        return Err(MemoryError::ValidationFailed(
            "source_id and target_id must be non-empty".to_string(),
        ));
    }
    let lock = state.lock().await;
    // Check if both memories exist before attempting delete
    let source_exists = db::get(&lock.0, &source_id).ok().flatten().is_some();
    let target_exists = db::get(&lock.0, &target_id).ok().flatten().is_some();
    if source_exists && target_exists {
        // Both exist but maybe no link between them — that's a not-found on the link
    } else if !source_exists && !target_exists {
        return Err(MemoryError::Conflict(
            "neither source nor target memory exists".to_string(),
        ));
    }
    match db::delete_link(&lock.0, &source_id, &target_id) {
        Ok(true) => Ok(Json(json!({"deleted": true}))),
        Ok(false) => Err(MemoryError::NotFound("link not found".to_string())),
        Err(e) => Err(MemoryError::DatabaseError(e.to_string())),
    }
}

// ============================================================
// Yellow Rock: Forensic Archive Handlers
// ============================================================

/// Create a forensic archive with SHA-256 hash chain.
pub async fn create_archive_handler(
    State(state): State<Db>,
    Query(query): Query<ArchiveQuery>,
) -> impl IntoResponse {
    let lock = state.lock().await;
    match db::create_forensic_archive(&lock.0, query.sender.as_deref(), query.before.as_deref()) {
        Ok(archive) => {
            let purge = query.purge.unwrap_or(false);
            if purge && !archive.messages.is_empty() {
                let before = &archive.archive_period.to;
                let _ = db::purge_messages(&lock.0, before, query.sender.as_deref(), None);
            }
            Json(json!(archive)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Verify a forensic archive's hash integrity.
pub async fn verify_archive_handler(Json(archive): Json<ForensicArchive>) -> impl IntoResponse {
    let verification = db::verify_forensic_archive(&archive);
    let status = if verification.valid {
        StatusCode::OK
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    };
    (status, Json(json!(verification))).into_response()
}

/// Import a verified forensic archive back into the database.
pub async fn import_archive_handler(
    State(state): State<Db>,
    Json(archive): Json<ForensicArchive>,
) -> impl IntoResponse {
    // Verify integrity first
    let verification = db::verify_forensic_archive(&archive);
    if !verification.valid {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": "archive failed integrity verification", "verification": verification})),
        ).into_response();
    }

    let lock = state.lock().await;
    match db::import_forensic_archive(&lock.0, &archive) {
        Ok((imported, skipped)) => Json(json!({
            "imported": imported,
            "skipped": skipped,
            "verification": verification,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Purge messages older than the retention period.
pub async fn purge_messages_handler(
    State(state): State<Db>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let lock = state.lock().await;
    let default_before =
        (chrono::Utc::now() - chrono::Duration::days(MESSAGE_RETENTION_DAYS)).to_rfc3339();
    let before = params.get("before").unwrap_or(&default_before);
    let sender = params.get("sender").map(|s| s.as_str());
    let contact_id = params.get("contact_id").map(|s| s.as_str());
    match db::purge_messages(&lock.0, before, sender, contact_id) {
        Ok(deleted) => Json(json!({"purged": deleted, "before": before})).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Verify forensic integrity of all messages in the database.
pub async fn verify_db_handler(State(state): State<Db>) -> impl IntoResponse {
    let lock = state.lock().await;
    match db::verify_db_integrity(&lock.0) {
        Ok((total, verified, failed, failed_ids)) => {
            let status = if failed == 0 {
                StatusCode::OK
            } else {
                StatusCode::UNPROCESSABLE_ENTITY
            };
            (
                status,
                Json(json!({
                    "total": total,
                    "verified": verified,
                    "failed": failed,
                    "valid": failed == 0,
                    "failed_ids": failed_ids,
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

// ============================================================
// Yellow Rock: Draft Management Handlers
// ============================================================

pub async fn create_draft_handler(
    State(state): State<Db>,
    Json(body): Json<CreateDraftRequest>,
) -> impl IntoResponse {
    if body.contact_id.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "contact_id cannot be empty"})),
        )
            .into_response();
    }
    if body.draft_content.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "draft_content cannot be empty"})),
        )
            .into_response();
    }
    let lock = state.lock().await;
    match db::create_draft(
        &lock.0,
        &body.contact_id,
        body.incoming_message_id.as_deref(),
        &body.draft_content,
    ) {
        Ok((id, hash)) => (
            StatusCode::CREATED,
            Json(json!({"id": id, "draft_hash": hash, "status": "pending"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn list_drafts_handler(
    State(state): State<Db>,
    Query(q): Query<DraftListQuery>,
) -> impl IntoResponse {
    let lock = state.lock().await;
    let drafts = db::list_drafts(
        &lock.0,
        q.contact_id.as_deref(),
        q.status.as_deref(),
        q.since.as_deref(),
        q.limit.unwrap_or(50),
    )
    .unwrap_or_default();
    let count = drafts.len();
    Json(json!({"drafts": drafts, "count": count})).into_response()
}

pub async fn get_draft_handler(
    State(state): State<Db>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let lock = state.lock().await;
    match db::get_draft(&lock.0, &id) {
        Ok(Some(d)) => Json(json!(d)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "draft not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn approve_draft_handler(
    State(state): State<Db>,
    Path(id): Path<String>,
    Json(body): Json<ReviewDraftRequest>,
) -> impl IntoResponse {
    if body.reviewer.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "reviewer cannot be empty"})),
        )
            .into_response();
    }
    let lock = state.lock().await;
    match db::approve_draft(&lock.0, &id, &body.reviewer, body.reason.as_deref()) {
        Ok(true) => Json(json!({"approved": true, "id": id})).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "draft not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn reject_draft_handler(
    State(state): State<Db>,
    Path(id): Path<String>,
    Json(body): Json<ReviewDraftRequest>,
) -> impl IntoResponse {
    if body.reviewer.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "reviewer cannot be empty"})),
        )
            .into_response();
    }
    let lock = state.lock().await;
    match db::reject_draft(&lock.0, &id, &body.reviewer, body.reason.as_deref()) {
        Ok(true) => Json(json!({"rejected": true, "id": id})).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "draft not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn send_draft_handler(
    State(state): State<Db>,
    Path(id): Path<String>,
    Json(body): Json<SendDraftRequest>,
) -> impl IntoResponse {
    let lock = state.lock().await;
    match db::mark_draft_sent(&lock.0, &id, &body.sent_content, &body.channel) {
        Ok(true) => Json(json!({"sent": true, "id": id})).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "draft not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn verify_draft_handler(
    State(state): State<Db>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let lock = state.lock().await;
    match db::verify_draft_chain(&lock.0, &id) {
        Ok(v) => {
            let status = if v.valid {
                StatusCode::OK
            } else {
                StatusCode::UNPROCESSABLE_ENTITY
            };
            (status, Json(json!(v))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}
