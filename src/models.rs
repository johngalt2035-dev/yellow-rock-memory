// Yellow Rock Memory — Forensic Communication Archive
// Copyright (c) 2026 johngalt2035-dev. All rights reserved.
// Created by johngalt2035-dev + Anthropic Claude AI Code
//
// Licensed under the MIT License. See LICENSE file in the project root.
//
// DISCLAIMER: This software is provided "AS IS", without warranty of any kind.
// Not legal advice. See LEGAL_DISCLAIMER.md for complete terms.

use serde::{Deserialize, Serialize};

/// Memory tier — mirrors human memory systems.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Short,
    Mid,
    Long,
}

impl Tier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Short => "short",
            Self::Mid => "mid",
            Self::Long => "long",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "short" => Some(Self::Short),
            "mid" => Some(Self::Mid),
            "long" => Some(Self::Long),
            _ => None,
        }
    }

    pub fn default_ttl_secs(&self) -> Option<i64> {
        match self {
            Self::Short => Some(6 * 3600),
            Self::Mid => Some(7 * 24 * 3600),
            Self::Long => None,
        }
    }
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub tier: Tier,
    pub namespace: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub priority: i32,
    /// 0.0-1.0 — how certain is this memory
    pub confidence: f64,
    /// Who/what created this: "user", "claude", "hook", "api", "import"
    pub source: String,
    pub access_count: i64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_accessed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLink {
    pub source_id: String,
    pub target_id: String,
    pub relation: String, // "related_to", "supersedes", "contradicts", "derived_from"
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateMemory {
    #[serde(default = "default_tier")]
    pub tier: Tier,
    #[serde(default = "default_namespace")]
    pub namespace: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub ttl_secs: Option<i64>,
}

fn default_tier() -> Tier {
    Tier::Mid
}
fn default_namespace() -> String {
    "yellow-rock".to_string()
}
fn default_priority() -> i32 {
    5
}
fn default_confidence() -> f64 {
    1.0
}
fn default_source() -> String {
    "api".to_string()
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemory {
    pub title: Option<String>,
    pub content: Option<String>,
    pub tier: Option<Tier>,
    pub namespace: Option<String>,
    pub tags: Option<Vec<String>>,
    pub priority: Option<i32>,
    pub confidence: Option<f64>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub tier: Option<Tier>,
    #[serde(default = "default_limit")]
    pub limit: Option<usize>,
    #[serde(default)]
    pub min_priority: Option<i32>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub until: Option<String>,
    #[serde(default)]
    pub tags: Option<String>, // comma-separated
}

fn default_limit() -> Option<usize> {
    Some(20)
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub tier: Option<Tier>,
    #[serde(default = "default_limit")]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub min_priority: Option<i32>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub until: Option<String>,
    #[serde(default)]
    pub tags: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RecallQuery {
    pub context: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default = "default_recall_limit")]
    pub limit: Option<usize>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
}

fn default_recall_limit() -> Option<usize> {
    Some(10)
}

#[derive(Debug, Deserialize)]
pub struct RecallBody {
    pub context: String,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default = "default_recall_limit")]
    pub limit: Option<usize>,
    #[serde(default)]
    pub tags: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LinkBody {
    pub source_id: String,
    pub target_id: String,
    #[serde(default = "default_relation")]
    pub relation: String,
}

fn default_relation() -> String {
    "related_to".to_string()
}

#[derive(Debug, Deserialize)]
pub struct ForgetQuery {
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>, // FTS pattern
    #[serde(default)]
    pub tier: Option<Tier>,
}

#[derive(Debug, Serialize)]
pub struct Stats {
    pub total: usize,
    pub by_tier: Vec<TierCount>,
    pub by_namespace: Vec<NamespaceCount>,
    pub expiring_soon: usize,
    pub links_count: usize,
    pub db_size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct TierCount {
    pub tier: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct NamespaceCount {
    pub namespace: String,
    pub count: usize,
}

pub const MAX_CONTENT_SIZE: usize = 65_536;
pub const PROMOTION_THRESHOLD: i64 = 5;
/// How much to extend TTL on access (1 hour for short, 1 day for mid)
pub const SHORT_TTL_EXTEND_SECS: i64 = 3600;
pub const MID_TTL_EXTEND_SECS: i64 = 86400;

// ============================================================
// Yellow Rock: Message Archive & Escalation Models
// ============================================================

/// Message categories for shadow logging
pub const CATEGORY_LOGISTICS: &str = "LOGISTICS";
pub const CATEGORY_NOISE: &str = "NOISE";
pub const CATEGORY_ESCALATION: &str = "ESCALATION_ALERT";
pub const CATEGORY_ACTION: &str = "ACTION_REQUIRED";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMessage {
    pub sender: String,
    /// Contact identifier for multi-contact routing (e.g., "opposing-counsel", "vendor-a")
    #[serde(default)]
    pub contact_id: Option<String>,
    #[serde(default = "default_channel")]
    pub channel: String,
    pub raw_content: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_logistics: Option<String>,
    #[serde(default)]
    pub escalation_score: i32,
}

fn default_channel() -> String {
    "signal".to_string()
}
fn default_category() -> String {
    CATEGORY_NOISE.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationReport {
    pub score: i32,
    pub level: String,
    pub count_1h: usize,
    pub count_6h: usize,
    pub count_24h: usize,
    pub count_7d: usize,
    pub noise_24h: usize,
    pub escalation_alerts_24h: usize,
    pub avg_message_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestItem {
    pub id: String,
    pub timestamp: String,
    pub raw_content: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_logistics: Option<String>,
    pub escalation_score: i32,
}

#[derive(Debug, Deserialize)]
pub struct DigestQuery {
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub contact_id: Option<String>,
    #[serde(default = "default_since_24h")]
    pub since: String,
}

fn default_since_24h() -> String {
    (chrono::Utc::now() - chrono::Duration::hours(24)).to_rfc3339()
}

#[derive(Debug, Deserialize)]
pub struct EscalationQuery {
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub contact_id: Option<String>,
}

// ============================================================
// Yellow Rock: Forensic Archive Models
// ============================================================

/// A single archived message with its forensic hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForensicMessage {
    pub id: String,
    pub sender: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<String>,
    pub timestamp: String,
    pub channel: String,
    pub raw_content: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_logistics: Option<String>,
    pub escalation_score: i32,
    pub created_at: String,
    /// SHA-256 hash of (id + sender + timestamp + raw_content + category)
    pub hash: String,
}

/// Complete forensic archive with chain-of-provenance metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForensicArchive {
    pub schema_version: String,
    pub archive_type: String,
    pub created_at: String,
    pub archive_period: ArchivePeriod,
    pub message_count: usize,
    pub messages: Vec<ForensicMessage>,
    /// SHA-256 hash of all individual message hashes concatenated in order
    pub chain_hash: String,
    /// SHA-256 hash of (chain_hash + created_at + message_count)
    pub archive_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivePeriod {
    pub from: String,
    pub to: String,
}

/// Retention policy: messages older than this many days are eligible for archival.
pub const MESSAGE_RETENTION_DAYS: i64 = 365;

#[derive(Debug, Deserialize)]
pub struct ArchiveQuery {
    #[serde(default)]
    pub sender: Option<String>,
    /// Archive messages older than this date (default: 1 year ago)
    #[serde(default)]
    pub before: Option<String>,
    /// If true, purge archived messages from the database after export
    #[serde(default)]
    pub purge: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveVerification {
    pub valid: bool,
    pub message_count: usize,
    pub messages_verified: usize,
    pub messages_failed: usize,
    pub chain_hash_valid: bool,
    pub archive_hash_valid: bool,
    pub failed_ids: Vec<String>,
}

// ============================================================
// Yellow Rock: Approved Draft Models
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovedDraft {
    pub id: String,
    pub draft_hash: String,
    pub contact_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incoming_message_id: Option<String>,
    pub draft_content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_hash: Option<String>,
    pub reviewer_chain: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_channel: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateDraftRequest {
    pub contact_id: String,
    #[serde(default)]
    pub incoming_message_id: Option<String>,
    pub draft_content: String,
}

#[derive(Debug, Deserialize)]
pub struct ReviewDraftRequest {
    pub reviewer: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendDraftRequest {
    pub sent_content: String,
    #[serde(default = "default_channel")]
    pub channel: String,
}

#[derive(Debug, Deserialize)]
pub struct DraftListQuery {
    #[serde(default)]
    pub contact_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default = "default_draft_limit")]
    pub limit: Option<usize>,
}

fn default_draft_limit() -> Option<usize> {
    Some(50)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftVerification {
    pub valid: bool,
    pub draft_hash_valid: bool,
    pub approval_hash_valid: bool,
    pub sent_hash_valid: bool,
    pub status: String,
}
