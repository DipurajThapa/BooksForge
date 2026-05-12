#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Phase 4 — validator-ledger persistence round-trip.

use std::sync::Arc;

use booksforge_domain::{Severity, ValidatorIssue, ValidatorRun, ValidatorRunStatus};
use booksforge_storage::{open_pool, run_migrations, SqliteStorage, StorageRepository};
use chrono::Utc;
use ulid::Ulid;

async fn fresh() -> (Arc<SqliteStorage>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let pool = open_pool(&dir.path().join("test.db")).await.unwrap();
    run_migrations(&pool).await.unwrap();
    (Arc::new(SqliteStorage::new(pool)), dir)
}

fn run(status: ValidatorRunStatus) -> ValidatorRun {
    ValidatorRun {
        id: Ulid::new(),
        validator_id: "batch:all".into(),
        ran_at: Utc::now(),
        status,
        duration_ms: 42,
        scope_hash: "abc".into(),
    }
}

fn issue(severity: Severity, code: &str) -> ValidatorIssue {
    ValidatorIssue {
        validator_id: "test".into(),
        code: code.into(),
        severity,
        message: format!("issue {code}"),
        node_id: None,
        offset_from: None,
        offset_to: None,
        auto_fixable: false,
    }
}

#[tokio::test]
async fn run_and_issues_persist_atomically() {
    let (storage, _dir) = fresh().await;
    let r = run(ValidatorRunStatus::Warnings);
    let issues = vec![issue(Severity::Warning, "W1"), issue(Severity::Info, "I1")];

    storage.validator_run_persist(&r, &issues).await.unwrap();

    let latest = storage.latest_validator_run().await.unwrap().unwrap();
    assert_eq!(latest.id, r.id);
    assert_eq!(latest.status, ValidatorRunStatus::Warnings);

    let stored = storage.list_validator_issues_for_run(r.id).await.unwrap();
    assert_eq!(stored.len(), 2);
    // Severity DESC ordering: Warning before Info.
    assert_eq!(stored[0].severity, Severity::Warning);
    assert_eq!(stored[1].severity, Severity::Info);
}

#[tokio::test]
async fn empty_issue_list_is_persisted_as_clean_run() {
    let (storage, _dir) = fresh().await;
    let r = run(ValidatorRunStatus::Ok);
    storage.validator_run_persist(&r, &[]).await.unwrap();
    let latest = storage.latest_validator_run().await.unwrap().unwrap();
    assert_eq!(latest.status, ValidatorRunStatus::Ok);
    let stored = storage.list_validator_issues_for_run(r.id).await.unwrap();
    assert!(stored.is_empty());
}

#[tokio::test]
async fn latest_returns_newest_run() {
    let (storage, _dir) = fresh().await;
    let r1 = run(ValidatorRunStatus::Ok);
    storage.validator_run_persist(&r1, &[]).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let r2 = run(ValidatorRunStatus::Errors);
    storage
        .validator_run_persist(&r2, &[issue(Severity::Error, "E1")])
        .await
        .unwrap();

    let latest = storage.latest_validator_run().await.unwrap().unwrap();
    assert_eq!(latest.id, r2.id);
    assert_eq!(latest.status, ValidatorRunStatus::Errors);
}
