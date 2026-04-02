# Data Retention Guide

Grey Rock Memory supports configurable data retention to meet regulatory and legal requirements.

## Default Retention

Messages are retained for **365 days** (1 year) by default. After the retention period, messages can be archived to forensic JSON (with SHA-256 hash chain) and optionally purged from the active database.

## Regulatory Retention Presets

Configure `retention_days` in your `grey-rock-config.json` to match your regulatory requirement:

| Standard | Retention | Days | Applies To |
|---|---|---|---|
| **Standard** | 1 year | 365 | General business communications |
| **CCPA** (California) | 12 months min | 365 | Consumer data (California) |
| **EEOC** (Employment) | 1-3 years | 365-1095 | Employment-related records |
| **General Litigation Hold** | Case duration + appeals | 2555+ | Active/anticipated litigation |
| **SEC Rule 17a-4** | 3-6 years | 1095-2190 | Broker-dealer communications |
| **SOX** (Sarbanes-Oxley) | 7 years | 2555 | Financial records |
| **IRS** | 7 years | 2555 | Tax-related records |
| **GDPR** (EU) | Purpose-limited | Varies | Personal data of EU residents |

## Configuration

In your `grey-rock-config.json`:

```json
"retention": {
  "days": 2555,
  "preset": "litigation-hold",
  "_presets": {
    "standard": 365,
    "ccpa": 365,
    "eeoc": 1095,
    "litigation-hold": 2555,
    "sec": 2190,
    "sox": 2555
  }
}
```

Or use the CLI: `grey-rock-memory archive-messages --before 2023-01-01`

## Archive Lifecycle

1. **Active** (0 to retention_days): Messages in SQLite, fully searchable, real-time escalation scoring
2. **Archive** (after retention_days): Export to forensic JSON with SHA-256 chain verification
3. **Purge** (optional): Remove from active database after archival
4. **Reimport** (as needed): Import verified archives back for historical/forensic/legal analysis

Archives are self-verifying — any tampered archive is rejected on import.

## Legal Disclaimer

Retention periods are provided as general guidance only. Consult a licensed attorney to determine applicable retention requirements for your specific jurisdiction and regulatory context. This software does not guarantee compliance with any specific regulation.
