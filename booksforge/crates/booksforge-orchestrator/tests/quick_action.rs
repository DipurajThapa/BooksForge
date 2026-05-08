#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::print_stdout, clippy::print_stderr, clippy::unimplemented)]

//! MZ-08 acceptance criteria for quick-action presets:
//!   • Cancellation aborts mid-stream and partial output is captured.
//!   • An audit row is written for **every** call (ok / cancelled / error).
//!   • `apply_quick_action` takes a `pre_ai` snapshot before mutating the scene.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use booksforge_domain::{
    AiCallStatus, Node, NodeKind, NodeStatus, QuickActionPreset, SnapshotTrigger,
};
use booksforge_fs::{BundleFilesystem, BundlePath, OsFilesystem};
use booksforge_ollama::{
    client::OllamaClient,
    types::{
        CancelToken, ChatOutcome, ChatRequest, GenerateOutcome, GenerateRequest, LocalModel,
        ModelInfo, OllamaVersion, ProgressSink, TokenSink,
    },
    OllamaError,
};
use booksforge_orchestrator::{
    quick_action::QuickActionOptions, ApplyOp, Orchestrator, OrchestratorConfig,
};
use booksforge_snapshot::SnapshotService;
use booksforge_storage::{open_pool, run_migrations, SqliteStorage, StorageRepository};
use chrono::Utc;
use ulid::Ulid;

// ── Custom mock Ollama client that honours CancelToken ────────────────────────

#[derive(Default)]
struct ChunkStreamMock {
    /// Tokens to emit one at a time before returning Ok.
    chunks:        Vec<String>,
    /// If `Some`, the mock cancels the supplied CancelToken after emitting
    /// this many chunks (inclusive), simulating user clicking Cancel.
    cancel_after:  Option<usize>,
}

impl ChunkStreamMock {
    fn with_chunks(chunks: Vec<&str>) -> Self {
        Self {
            chunks: chunks.into_iter().map(String::from).collect(),
            cancel_after: None,
        }
    }
    fn cancel_after(mut self, n: usize) -> Self {
        self.cancel_after = Some(n);
        self
    }
}

#[async_trait]
impl OllamaClient for ChunkStreamMock {
    async fn version(&self) -> Result<OllamaVersion, OllamaError> {
        Ok(OllamaVersion { version: "test".into() })
    }
    async fn list_local_models(&self) -> Result<Vec<LocalModel>, OllamaError> { Ok(vec![]) }
    async fn show(&self, model: &str) -> Result<ModelInfo, OllamaError> {
        Ok(ModelInfo {
            name: model.to_owned(), digest: None, family: None,
            parameter_size: None, quantization_level: None,
        })
    }
    async fn pull(&self, _model: &str, _progress: ProgressSink) -> Result<(), OllamaError> {
        Ok(())
    }

    async fn generate(
        &self,
        request: GenerateRequest,
        mut sink: TokenSink,
        cancel: CancelToken,
    ) -> Result<GenerateOutcome, OllamaError> {
        let mut emitted = 0usize;
        let mut full = String::new();
        for chunk in &self.chunks {
            if cancel.is_cancelled() {
                return Err(OllamaError::Cancelled);
            }
            sink(chunk);
            full.push_str(chunk);
            emitted += 1;
            if let Some(n) = self.cancel_after {
                if emitted == n {
                    cancel.cancel();
                    // The next loop iteration would observe cancellation.
                }
            }
        }
        if cancel.is_cancelled() {
            return Err(OllamaError::Cancelled);
        }
        Ok(GenerateOutcome {
            model:              request.model,
            response:           full,
            prompt_eval_count:  10,
            eval_count:         emitted as u32,
            total_duration_ns:  1_000_000,
        })
    }

    async fn chat(
        &self,
        _req: ChatRequest,
        _sink: TokenSink,
        _cancel: CancelToken,
    ) -> Result<ChatOutcome, OllamaError> {
        unimplemented!("not used by quick-action tests")
    }
}

// ── Harness ───────────────────────────────────────────────────────────────────

struct Harness {
    pub orchestrator: Orchestrator,
    pub storage:      Arc<SqliteStorage>,
    pub node_id:      Ulid,
    pub _dir:         tempfile::TempDir,
}

async fn setup(ollama: Arc<dyn OllamaClient>) -> Harness {
    let dir = tempfile::tempdir().expect("tempdir");
    let bundle_root = dir.path().join("test.booksforge");
    std::fs::create_dir_all(bundle_root.join("snapshots/objects")).expect("mkdir");
    let bundle = BundlePath::new(&bundle_root);

    let pool = open_pool(&bundle.db()).await.expect("open_pool");
    run_migrations(&pool).await.expect("migrations");
    let storage = Arc::new(SqliteStorage::new(pool));

    // Seed one scene node.
    let node_id = Ulid::new();
    let now = Utc::now();
    storage.insert_node(&Node {
        id:           node_id,
        parent_id:    None,
        kind:         NodeKind::Scene,
        title:        "Test Scene".into(),
        position:     Node::DEFAULT_POSITION.into(),
        status:       NodeStatus::Drafting,
        pov:          None,
        beat:         None,
        target_words: None,
        created_at:   now,
        updated_at:   now,
        deleted_at:   None,
    }).await.expect("insert_node");

    let storage_trait: Arc<dyn StorageRepository> = storage.clone();
    let fs:           Arc<dyn BundleFilesystem>   = Arc::new(OsFilesystem);
    let snapshot = Arc::new(SnapshotService::new(storage_trait, fs, bundle));

    let orchestrator = Orchestrator::new(ollama, storage.clone(), OrchestratorConfig::default())
        .with_snapshot(snapshot);

    Harness { orchestrator, storage, node_id, _dir: dir }
}

fn drain_sink() -> (TokenSink, Arc<Mutex<String>>) {
    let buf = Arc::new(Mutex::new(String::new()));
    let buf2 = buf.clone();
    let sink: TokenSink = Box::new(move |t: &str| {
        buf2.lock().unwrap().push_str(t);
    });
    (sink, buf)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn ok_call_writes_audit_row_with_full_output() {
    let mock = Arc::new(ChunkStreamMock::with_chunks(vec!["The ", "river ", "ran."]));
    let h = setup(mock).await;
    let (sink, _) = drain_sink();

    let outcome = h.orchestrator.run_quick_action(
        h.node_id,
        QuickActionPreset::Sharpen,
        "Original passage.".into(),
        "test-model".into(),
        QuickActionOptions::default(),
        CancelToken::new(),
        sink,
    ).await.expect("run_quick_action");

    assert_eq!(outcome.status, AiCallStatus::Ok);
    assert_eq!(outcome.output_text, "The river ran.");

    let stored = h.storage.ai_call_get(outcome.ai_call_id).await
        .expect("ai_call_get").expect("row exists");
    assert_eq!(stored.status, AiCallStatus::Ok);
    assert_eq!(stored.output_text.as_deref(), Some("The river ran."));
    assert_eq!(stored.preset, QuickActionPreset::Sharpen);
    assert!(stored.duration_ms.is_some());
    assert!(stored.applied_at.is_none(), "must not be applied yet");
}

#[tokio::test]
async fn cancellation_mid_stream_writes_cancelled_row_with_partial_output() {
    let mock = Arc::new(
        ChunkStreamMock::with_chunks(vec!["alpha ", "beta ", "gamma ", "delta"])
            .cancel_after(2),
    );
    let h = setup(mock).await;
    let (sink, _) = drain_sink();

    let outcome = h.orchestrator.run_quick_action(
        h.node_id,
        QuickActionPreset::Rephrase,
        "scope".into(),
        "test-model".into(),
        QuickActionOptions::default(),
        CancelToken::new(),
        sink,
    ).await.expect("run_quick_action");

    assert_eq!(outcome.status, AiCallStatus::Cancelled);
    // Partial output (first two chunks) must survive.
    assert_eq!(outcome.output_text, "alpha beta ");

    let stored = h.storage.ai_call_get(outcome.ai_call_id).await
        .expect("ai_call_get").expect("row exists");
    assert_eq!(stored.status, AiCallStatus::Cancelled);
    assert_eq!(stored.output_text.as_deref(), Some("alpha beta "));
    assert!(stored.error_message.as_deref().unwrap_or("").contains("cancelled"));
}

#[tokio::test]
async fn apply_takes_pre_ai_snapshot_and_stamps_ledger() {
    let mock = Arc::new(ChunkStreamMock::with_chunks(vec!["polished prose"]));
    let h = setup(mock).await;
    let (sink, _) = drain_sink();

    let outcome = h.orchestrator.run_quick_action(
        h.node_id,
        QuickActionPreset::Sharpen,
        "rough prose".into(),
        "test-model".into(),
        QuickActionOptions::default(),
        CancelToken::new(),
        sink,
    ).await.unwrap();
    assert_eq!(outcome.status, AiCallStatus::Ok);

    let snapshots_before = h.storage.list_snapshots(None).await.unwrap().len();

    let result = h.orchestrator.apply_quick_action(
        outcome.ai_call_id,
        outcome.output_text.clone(),
        ApplyOp::Replace,
    ).await.expect("apply_quick_action");

    // 1. Audit row stamped with snapshot id + applied_at.
    let stored = h.storage.ai_call_get(outcome.ai_call_id).await.unwrap().unwrap();
    assert_eq!(stored.pre_edit_snapshot_id, Some(result.pre_snapshot_id));
    assert!(stored.applied_at.is_some());

    // 2. Snapshot exists and uses the `pre_ai` trigger.
    let snapshots_after = h.storage.list_snapshots(None).await.unwrap();
    assert_eq!(snapshots_after.len(), snapshots_before + 1);
    let snap = snapshots_after.iter().find(|s| s.id == result.pre_snapshot_id).unwrap();
    assert_eq!(snap.trigger, SnapshotTrigger::PreAi);

    // 3. Re-applying the same call is rejected (idempotency).
    let err = h.orchestrator.apply_quick_action(
        outcome.ai_call_id,
        "anything".into(),
        ApplyOp::Replace,
    ).await.unwrap_err();
    assert!(err.to_string().to_lowercase().contains("already applied"));
}

#[tokio::test]
async fn ok_continue_appends_to_existing_scene() {
    let mock = Arc::new(ChunkStreamMock::with_chunks(vec!["new paragraph"]));
    let h = setup(mock).await;

    // Seed existing scene content.
    let pm_doc = serde_json::json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "content": [{ "type": "text", "text": "First paragraph." }]
        }]
    });
    let bytes = serde_json::to_vec(&pm_doc).unwrap();
    h.storage.save_scene(&booksforge_domain::SceneContent {
        node_id:    h.node_id,
        pm_doc,
        word_count: 2,
        char_count: 16,
        hash:       blake3::hash(&bytes).to_hex().to_string(),
        updated_at: Utc::now(),
    }).await.unwrap();

    let (sink, _) = drain_sink();
    let outcome = h.orchestrator.run_quick_action(
        h.node_id,
        QuickActionPreset::Continue_,
        "First paragraph.".into(),
        "test-model".into(),
        QuickActionOptions::default(),
        CancelToken::new(),
        sink,
    ).await.unwrap();

    h.orchestrator.apply_quick_action(
        outcome.ai_call_id,
        outcome.output_text.clone(),
        ApplyOp::Append,
    ).await.unwrap();

    let scene = h.storage.load_scene(h.node_id).await.unwrap().unwrap();
    let blocks = scene.pm_doc.get("content").and_then(|v| v.as_array()).unwrap();
    assert_eq!(blocks.len(), 2, "append must add a second paragraph");
}
