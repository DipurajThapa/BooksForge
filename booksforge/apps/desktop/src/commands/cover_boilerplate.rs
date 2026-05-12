//! Stage 6 (Format & Ship) cover-image + boilerplate-page commands.
//!
//! Persistence convention:
//!   - Cover set       → `book:cover_set` memory entry (JSON of CoverSet)
//!   - Boilerplate     → `book:boilerplate_pages` memory entry
//!                       (JSON of `Vec<BoilerplatePage>`)
//!
//! Cover images live on disk under `<bundle>/assets/cover-<slot>.<ext>`.
//! The memory entry stores the relative path so opens on a different
//! machine still find the file.
//!
//! Why memory entries (not a dedicated table): boilerplate is a
//! small, JSON-shaped payload (≤32 KB total in practice) and the
//! memory_list infrastructure already handles upsert + UI loading
//! without a new migration.

use booksforge_domain::{BoilerplatePage, CoverAsset, CoverSet, MemoryEntry, MemoryScope};
use booksforge_ipc::{
    BoilerplatePageDto, BoilerplateSaveInput, BoilerplateSaveResult, BooksForgeError,
    CoverImportInput, CoverRemoveInput, CoverSetDto,
};
use booksforge_storage::StorageRepository;
use chrono::Utc;
use std::path::{Path, PathBuf};
use tauri::State;
use ulid::Ulid;

use crate::commands::agents::require_open_project;
use crate::state::AppState;

const COVER_SET_KEY: &str = "cover_set";
const BOILERPLATE_KEY: &str = "boilerplate_pages";

// ── helpers ────────────────────────────────────────────────────────────────

fn read_cover_set(value: serde_json::Value) -> CoverSet {
    serde_json::from_value(value).unwrap_or_default()
}

fn read_boilerplate(value: serde_json::Value) -> Vec<BoilerplatePage> {
    serde_json::from_value(value).unwrap_or_default()
}

const SUPPORTED_COVER_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "tif", "tiff"];
const MAX_COVER_BYTES: u64 = 50 * 1024 * 1024;
const MAX_BOILERPLATE_PAGES: usize = 50;
const MAX_BOILERPLATE_BODY_BYTES: usize = 10_000;

fn validate_slot(slot: &str) -> Result<&'static str, BooksForgeError> {
    match slot {
        "front" => Ok("front"),
        "back" => Ok("back"),
        "spine" => Ok("spine"),
        other => Err(BooksForgeError::validation(format!(
            "unknown cover slot {other:?}; expected one of front | back | spine",
        ))),
    }
}

fn detect_mime(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "tif" | "tiff" => "image/tiff",
        _ => "application/octet-stream",
    }
}

/// Pure validation of a cover source file. Returns `(lowercased ext,
/// byte size)` on success or a typed validation error. Split out from
/// the Tauri command so its checks are unit-testable without a Tauri
/// runtime or a real `State<'_, AppState>`.
fn validate_cover_source(source: &Path) -> Result<(String, u64), BooksForgeError> {
    if !source.is_file() {
        return Err(BooksForgeError::validation(format!(
            "source path is not a regular file: {}",
            source.display(),
        )));
    }
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .ok_or_else(|| BooksForgeError::validation("cover file has no extension".to_owned()))?;
    if !SUPPORTED_COVER_EXTENSIONS.contains(&ext.as_str()) {
        return Err(BooksForgeError::validation(format!(
            "cover format .{ext} not supported; use jpg / png / webp / tiff",
        )));
    }
    let meta = source
        .metadata()
        .map_err(|e| BooksForgeError::internal(format!("read source metadata: {e}")))?;
    if meta.len() > MAX_COVER_BYTES {
        return Err(BooksForgeError::validation(format!(
            "cover file is {} MB; cap is {} MB",
            meta.len() / 1024 / 1024,
            MAX_COVER_BYTES / 1024 / 1024,
        )));
    }
    Ok((ext, meta.len()))
}

/// Pure validation of a boilerplate-page list. Enforces the same
/// caps as the Tauri command (≤50 pages, ≤10 000 bytes per body) so
/// the `boilerplate_save` command becomes a thin wrapper.
fn validate_boilerplate_list(pages: &[BoilerplatePage]) -> Result<(), BooksForgeError> {
    if pages.len() > MAX_BOILERPLATE_PAGES {
        return Err(BooksForgeError::validation(format!(
            "boilerplate list is {} entries; cap is {MAX_BOILERPLATE_PAGES}",
            pages.len(),
        )));
    }
    for p in pages {
        if p.body_md.len() > MAX_BOILERPLATE_BODY_BYTES {
            return Err(BooksForgeError::validation(format!(
                "boilerplate page {:?} body is {} bytes; cap is {MAX_BOILERPLATE_BODY_BYTES}",
                p.title,
                p.body_md.len(),
            )));
        }
    }
    Ok(())
}

async fn upsert_cover_set(
    storage: &dyn StorageRepository,
    set: &CoverSet,
) -> Result<(), BooksForgeError> {
    let value = serde_json::to_value(set)
        .map_err(|e| BooksForgeError::internal(format!("serialise cover_set: {e}")))?;
    let now = Utc::now();
    let entry = MemoryEntry {
        id: Ulid::new(),
        scope: MemoryScope::Book,
        key: COVER_SET_KEY.to_owned(),
        value_json: value,
        agent_id: "cover-boilerplate".to_owned(),
        created_at: now,
        updated_at: now,
    };
    storage
        .memory_upsert(&entry)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))
}

async fn load_cover_set_inner(
    storage: &dyn StorageRepository,
) -> Result<CoverSet, BooksForgeError> {
    let entry = storage
        .memory_get(MemoryScope::Book, COVER_SET_KEY)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(entry
        .map(|e| read_cover_set(e.value_json))
        .unwrap_or_default())
}

async fn load_boilerplate_inner(
    storage: &dyn StorageRepository,
) -> Result<Vec<BoilerplatePage>, BooksForgeError> {
    let entry = storage
        .memory_get(MemoryScope::Book, BOILERPLATE_KEY)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(entry
        .map(|e| read_boilerplate(e.value_json))
        .unwrap_or_default())
}

// ── cover_import ───────────────────────────────────────────────────────────

/// Copy a source image into `<bundle>/assets/cover-<slot>.<ext>` and
/// persist its `CoverAsset` metadata into `book:cover_set` memory.
/// Returns the full updated CoverSet so the UI can refresh in one
/// round-trip.
///
/// Validation: source must be a regular file under 50 MB with a
/// recognised image extension (jpg / jpeg / png / webp / tiff). Pixel
/// dimensions are not measured here — the export gate's per-target
/// validators handle DPI / aspect at run-time, where they belong.
#[tauri::command]
pub async fn cover_import(
    input: CoverImportInput,
    state: State<'_, AppState>,
) -> Result<CoverSetDto, BooksForgeError> {
    let project = require_open_project(&state).await?;
    let set = cover_import_inner(
        &input.slot,
        Path::new(&input.source_path),
        &project.bundle.assets(),
        project.storage.as_ref(),
    )
    .await?;
    Ok(CoverSetDto::from(&set))
}

/// Storage-and-fs core of `cover_import`. Takes the slot, the source
/// path, the assets directory, and a storage handle directly so this
/// can be unit-tested without a Tauri runtime. The Tauri command
/// above is a thin wrapper that pulls the assets dir and storage out
/// of `AppState`.
async fn cover_import_inner(
    slot_input: &str,
    source: &Path,
    assets_dir: &Path,
    storage: &dyn StorageRepository,
) -> Result<CoverSet, BooksForgeError> {
    let slot = validate_slot(slot_input)?;
    let (ext, size_bytes) = validate_cover_source(source)?;

    std::fs::create_dir_all(assets_dir)
        .map_err(|e| BooksForgeError::internal(format!("create assets dir: {e}")))?;
    let target_filename = format!("cover-{slot}.{ext}");
    let target_path: PathBuf = assets_dir.join(&target_filename);

    // Copy to a sibling temp file first; only rename it into place
    // after the storage upsert succeeds. Otherwise a failed upsert
    // leaves an orphaned image in `assets/` that the writer can't
    // see in the UI but is shipped in every export.
    let stage_path: PathBuf = assets_dir.join(format!(".cover-{slot}.{ext}.tmp"));
    std::fs::copy(source, &stage_path).map_err(|e| {
        BooksForgeError::internal(format!(
            "stage cover {} → {}: {e}",
            source.display(),
            stage_path.display(),
        ))
    })?;

    let asset = CoverAsset {
        bundle_path: format!("assets/{target_filename}"),
        original_filename: source
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(&target_filename)
            .to_owned(),
        size_bytes,
        mime_type: detect_mime(&ext).to_owned(),
        width_px: None,
        height_px: None,
        imported_at: Utc::now(),
    };

    let mut set = load_cover_set_inner(storage).await?;
    match slot {
        "front" => set.front = Some(asset),
        "back" => set.back = Some(asset),
        "spine" => set.spine = Some(asset),
        _ => unreachable!("validate_slot already filtered"),
    }
    if let Err(e) = upsert_cover_set(storage, &set).await {
        let _ = std::fs::remove_file(&stage_path);
        return Err(e);
    }
    std::fs::rename(&stage_path, &target_path).map_err(|e| {
        let _ = std::fs::remove_file(&stage_path);
        BooksForgeError::internal(format!(
            "promote staged cover {} → {}: {e}",
            stage_path.display(),
            target_path.display(),
        ))
    })?;
    Ok(set)
}

// ── cover_load ─────────────────────────────────────────────────────────────

/// Load the current cover set from memory. Returns an empty
/// `CoverSetDto` (all slots null) when nothing has been imported yet.
#[tauri::command]
pub async fn cover_load(state: State<'_, AppState>) -> Result<CoverSetDto, BooksForgeError> {
    let project = require_open_project(&state).await?;
    let set = load_cover_set_inner(project.storage.as_ref()).await?;
    Ok(CoverSetDto::from(&set))
}

// ── cover_remove ───────────────────────────────────────────────────────────

/// Clear a single cover slot. The image file on disk stays so the
/// writer can recover by re-importing manually; only the memory entry
/// is updated. (We intentionally don't delete files — that would
/// violate the "no destructive ops without explicit confirmation"
/// safety rule.)
#[tauri::command]
pub async fn cover_remove(
    input: CoverRemoveInput,
    state: State<'_, AppState>,
) -> Result<CoverSetDto, BooksForgeError> {
    let slot = validate_slot(&input.slot)?;
    let project = require_open_project(&state).await?;
    let mut set = load_cover_set_inner(project.storage.as_ref()).await?;
    match slot {
        "front" => set.front = None,
        "back" => set.back = None,
        "spine" => set.spine = None,
        _ => unreachable!(),
    }
    upsert_cover_set(project.storage.as_ref(), &set).await?;
    Ok(CoverSetDto::from(&set))
}

// ── boilerplate_load ───────────────────────────────────────────────────────

/// Load the full boilerplate-pages list. The list is stored as a
/// single JSON entry; we sort by `order` on the way out so the UI
/// renders deterministically.
#[tauri::command]
pub async fn boilerplate_load(
    state: State<'_, AppState>,
) -> Result<Vec<BoilerplatePageDto>, BooksForgeError> {
    let project = require_open_project(&state).await?;
    let mut pages = load_boilerplate_inner(project.storage.as_ref()).await?;
    pages.sort_by_key(|p| p.order);
    Ok(pages.iter().map(BoilerplatePageDto::from).collect())
}

// ── boilerplate_save ───────────────────────────────────────────────────────

/// Whole-list upsert. The new list replaces what was in memory; the
/// caller is responsible for the full payload (the UI keeps the
/// editable list in component state and posts it back on save).
#[tauri::command]
pub async fn boilerplate_save(
    input: BoilerplateSaveInput,
    state: State<'_, AppState>,
) -> Result<BoilerplateSaveResult, BooksForgeError> {
    let project = require_open_project(&state).await?;
    boilerplate_save_inner(input, project.storage.as_ref()).await
}

/// Storage core of `boilerplate_save`. Pulled out so the validation
/// + persistence path is testable against an in-memory SqliteStorage
/// without spinning up a Tauri runtime.
async fn boilerplate_save_inner(
    input: BoilerplateSaveInput,
    storage: &dyn StorageRepository,
) -> Result<BoilerplateSaveResult, BooksForgeError> {
    let pages: Result<Vec<BoilerplatePage>, String> = input
        .pages
        .iter()
        .map(BoilerplatePageDto::to_domain)
        .collect();
    let pages = pages.map_err(BooksForgeError::validation)?;
    validate_boilerplate_list(&pages)?;

    let value = serde_json::to_value(&pages)
        .map_err(|e| BooksForgeError::internal(format!("serialise boilerplate: {e}")))?;
    let now = Utc::now();
    let entry = MemoryEntry {
        id: Ulid::new(),
        scope: MemoryScope::Book,
        key: BOILERPLATE_KEY.to_owned(),
        value_json: value,
        agent_id: "cover-boilerplate".to_owned(),
        created_at: now,
        updated_at: now,
    };
    storage
        .memory_upsert(&entry)
        .await
        .map_err(|e| BooksForgeError::internal(e.to_string()))?;
    Ok(BoilerplateSaveResult {
        saved_count: pages.len() as u32,
    })
}

// Tests use the inner storage-and-fs helpers (`cover_import_inner`,
// `boilerplate_save_inner`, `validate_*`) against a real SqliteStorage
// + tempdir so no Tauri runtime is needed.

#[cfg(test)]
mod tests {
    use super::*;
    use booksforge_domain::BoilerplateKind;
    use booksforge_storage::{open_pool, run_migrations, SqliteStorage};
    use std::sync::Arc;

    // ── shared fixtures ────────────────────────────────────────────────

    async fn fresh_storage() -> (Arc<SqliteStorage>, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let pool = open_pool(&dir.path().join("test.db"))
            .await
            .expect("open_pool");
        run_migrations(&pool).await.expect("migrations");
        (Arc::new(SqliteStorage::new(pool)), dir)
    }

    fn write_fixture(dir: &Path, name: &str, contents: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, contents).expect("write fixture");
        path
    }

    fn page(id: &str, kind: BoilerplateKind, body: &str) -> BoilerplatePageDto {
        BoilerplatePageDto::from(&BoilerplatePage {
            id: id.into(),
            kind,
            title: kind.default_heading().to_owned(),
            body_md: body.into(),
            order: 0,
            include_in_export: true,
        })
    }

    // ── validate_slot ──────────────────────────────────────────────────

    #[test]
    fn slot_front_back_spine_are_accepted() {
        assert_eq!(validate_slot("front").unwrap(), "front");
        assert_eq!(validate_slot("back").unwrap(), "back");
        assert_eq!(validate_slot("spine").unwrap(), "spine");
    }

    #[test]
    fn slot_rejects_uppercase() {
        // The Tauri JSON contract is snake_case; the UI is the
        // single source of slot strings, so we don't accept variants.
        assert!(validate_slot("Front").is_err());
        assert!(validate_slot("FRONT").is_err());
    }

    #[test]
    fn slot_rejects_empty_and_unknown() {
        assert!(validate_slot("").is_err());
        assert!(validate_slot("cover").is_err());
        assert!(validate_slot("interior").is_err());
    }

    // ── validate_cover_source ──────────────────────────────────────────

    #[test]
    fn cover_source_accepts_every_supported_extension() {
        let dir = tempfile::tempdir().unwrap();
        for ext in SUPPORTED_COVER_EXTENSIONS {
            let path = write_fixture(dir.path(), &format!("cover.{ext}"), b"\x89PNG");
            let (got_ext, size) = validate_cover_source(&path).expect("accepted");
            assert_eq!(got_ext, *ext);
            assert_eq!(size, 4);
        }
    }

    #[test]
    fn cover_source_lowercases_extensions() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_fixture(dir.path(), "COVER.JPG", b"\xff\xd8");
        let (ext, _) = validate_cover_source(&path).expect("accepted");
        assert_eq!(ext, "jpg", "extension must normalise to lowercase");
    }

    #[test]
    fn cover_source_rejects_banned_extensions() {
        let dir = tempfile::tempdir().unwrap();
        for ext in ["exe", "txt", "pdf", "gif", "bmp", "heic"] {
            let path = write_fixture(dir.path(), &format!("cover.{ext}"), b"x");
            let err = validate_cover_source(&path).expect_err("must reject");
            assert!(
                format!("{err:?}").to_lowercase().contains("not supported"),
                "ext {ext} → {err:?}"
            );
        }
    }

    #[test]
    fn cover_source_rejects_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.jpg");
        let err = validate_cover_source(&path).expect_err("must reject");
        assert!(format!("{err:?}").contains("not a regular file"));
    }

    #[test]
    fn cover_source_rejects_directory() {
        // A directory passed in place of a file must fail the
        // is_file() check, not be silently treated as one.
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a-folder");
        std::fs::create_dir(&nested).unwrap();
        let err = validate_cover_source(&nested).expect_err("must reject");
        assert!(format!("{err:?}").contains("not a regular file"));
    }

    #[test]
    fn cover_source_rejects_file_without_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_fixture(dir.path(), "cover", b"x");
        let err = validate_cover_source(&path).expect_err("must reject");
        assert!(format!("{err:?}").contains("no extension"));
    }

    #[test]
    fn cover_source_accepts_exactly_at_size_cap() {
        // 50 MB is the inclusive ceiling — `> MAX_COVER_BYTES` rejects,
        // so a file of exactly MAX_COVER_BYTES must be accepted.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.jpg");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_COVER_BYTES).unwrap();
        let (ext, size) = validate_cover_source(&path).expect("accepted at boundary");
        assert_eq!(ext, "jpg");
        assert_eq!(size, MAX_COVER_BYTES);
    }

    #[test]
    fn cover_source_rejects_over_size_cap() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("toobig.jpg");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_COVER_BYTES + 1).unwrap();
        let err = validate_cover_source(&path).expect_err("must reject");
        assert!(format!("{err:?}").contains("cap is 50 MB"));
    }

    // ── detect_mime ────────────────────────────────────────────────────

    #[test]
    fn mime_detection_covers_every_supported_extension() {
        assert_eq!(detect_mime("jpg"), "image/jpeg");
        assert_eq!(detect_mime("JPEG"), "image/jpeg");
        assert_eq!(detect_mime("png"), "image/png");
        assert_eq!(detect_mime("webp"), "image/webp");
        assert_eq!(detect_mime("tif"), "image/tiff");
        assert_eq!(detect_mime("tiff"), "image/tiff");
    }

    #[test]
    fn mime_unknown_extension_falls_back_to_octet_stream() {
        // detect_mime is called after validate_cover_source narrows
        // to the allowlist, but defensive fallback for anything else.
        assert_eq!(detect_mime("xyz"), "application/octet-stream");
        assert_eq!(detect_mime(""), "application/octet-stream");
    }

    // ── validate_boilerplate_list ──────────────────────────────────────

    #[test]
    fn boilerplate_empty_list_is_accepted() {
        validate_boilerplate_list(&[]).expect("empty is OK");
    }

    #[test]
    fn boilerplate_exactly_at_page_cap_is_accepted() {
        let pages: Vec<BoilerplatePage> = (0..MAX_BOILERPLATE_PAGES)
            .map(|i| {
                BoilerplatePage::new(format!("p-{i}"), BoilerplateKind::Other, i as u32)
            })
            .collect();
        validate_boilerplate_list(&pages).expect("cap-inclusive");
    }

    #[test]
    fn boilerplate_over_page_cap_is_rejected() {
        let pages: Vec<BoilerplatePage> = (0..MAX_BOILERPLATE_PAGES + 1)
            .map(|i| {
                BoilerplatePage::new(format!("p-{i}"), BoilerplateKind::Other, i as u32)
            })
            .collect();
        let err = validate_boilerplate_list(&pages).expect_err("must reject");
        assert!(format!("{err:?}").contains("cap is 50"));
    }

    #[test]
    fn boilerplate_body_exactly_at_byte_cap_is_accepted() {
        let mut p = BoilerplatePage::new("p", BoilerplateKind::Acknowledgments, 0);
        p.body_md = "a".repeat(MAX_BOILERPLATE_BODY_BYTES);
        validate_boilerplate_list(std::slice::from_ref(&p)).expect("at-cap body");
    }

    #[test]
    fn boilerplate_body_over_byte_cap_is_rejected() {
        let mut p = BoilerplatePage::new("p", BoilerplateKind::Acknowledgments, 0);
        p.body_md = "a".repeat(MAX_BOILERPLATE_BODY_BYTES + 1);
        let err =
            validate_boilerplate_list(std::slice::from_ref(&p)).expect_err("must reject");
        assert!(format!("{err:?}").contains("cap is 10000"));
    }

    #[test]
    fn boilerplate_body_oversize_reports_offending_title() {
        let mut p =
            BoilerplatePage::new("p", BoilerplateKind::Acknowledgments, 0);
        p.title = "My Long Page".into();
        p.body_md = "x".repeat(MAX_BOILERPLATE_BODY_BYTES + 1);
        let err =
            validate_boilerplate_list(std::slice::from_ref(&p)).expect_err("must reject");
        // The error format quotes the title with serde's Debug.
        assert!(format!("{err:?}").contains("My Long Page"));
    }

    // ── cover_import_inner E2E ─────────────────────────────────────────

    #[tokio::test]
    async fn cover_import_round_trips_through_storage_and_disk() {
        let (storage, dir) = fresh_storage().await;
        let assets_dir = dir.path().join("assets");
        let fixture_bytes: &[u8] = b"\xff\xd8jpegbytes";
        let src = write_fixture(dir.path(), "my-front.jpg", fixture_bytes);

        let set =
            cover_import_inner("front", &src, &assets_dir, storage.as_ref())
                .await
                .expect("import succeeds");

        // CoverSet now has a front asset whose metadata matches.
        let front = set.front.as_ref().expect("front populated");
        assert_eq!(front.bundle_path, "assets/cover-front.jpg");
        assert_eq!(front.original_filename, "my-front.jpg");
        assert_eq!(front.size_bytes as usize, fixture_bytes.len());
        assert_eq!(front.mime_type, "image/jpeg");

        // File was actually copied into the assets dir.
        let dest = assets_dir.join("cover-front.jpg");
        assert!(dest.is_file(), "destination file exists");
        assert_eq!(std::fs::read(dest).unwrap(), fixture_bytes);

        // Re-load through storage and confirm round-trip equivalence.
        let loaded = load_cover_set_inner(storage.as_ref()).await.unwrap();
        assert_eq!(loaded.front.as_ref().unwrap().bundle_path, front.bundle_path);
    }

    #[tokio::test]
    async fn cover_import_replaces_slot_does_not_leak_previous_asset() {
        let (storage, dir) = fresh_storage().await;
        let assets_dir = dir.path().join("assets");

        let first = write_fixture(dir.path(), "first.jpg", b"first");
        cover_import_inner("front", &first, &assets_dir, storage.as_ref())
            .await
            .unwrap();

        let second = write_fixture(dir.path(), "second.png", b"second");
        let set =
            cover_import_inner("front", &second, &assets_dir, storage.as_ref())
                .await
                .unwrap();

        // Slot reflects the second import, not the first.
        let front = set.front.as_ref().unwrap();
        assert_eq!(front.bundle_path, "assets/cover-front.png");
        assert_eq!(front.original_filename, "second.png");
        assert_eq!(front.mime_type, "image/png");
        // The other slots remain empty — replacement is per-slot, not
        // a wipe.
        assert!(set.back.is_none());
        assert!(set.spine.is_none());
    }

    #[tokio::test]
    async fn cover_import_three_slots_coexist() {
        let (storage, dir) = fresh_storage().await;
        let assets_dir = dir.path().join("assets");

        let f = write_fixture(dir.path(), "f.jpg", b"f");
        let b = write_fixture(dir.path(), "b.png", b"b");
        let s = write_fixture(dir.path(), "s.webp", b"s");
        cover_import_inner("front", &f, &assets_dir, storage.as_ref())
            .await
            .unwrap();
        cover_import_inner("back", &b, &assets_dir, storage.as_ref())
            .await
            .unwrap();
        let set =
            cover_import_inner("spine", &s, &assets_dir, storage.as_ref())
                .await
                .unwrap();

        assert!(set.front.is_some());
        assert!(set.back.is_some());
        assert!(set.spine.is_some());
        assert_eq!(set.front.as_ref().unwrap().mime_type, "image/jpeg");
        assert_eq!(set.back.as_ref().unwrap().mime_type, "image/png");
        assert_eq!(set.spine.as_ref().unwrap().mime_type, "image/webp");
    }

    #[tokio::test]
    async fn cover_import_propagates_validation_errors() {
        let (storage, dir) = fresh_storage().await;
        let assets_dir = dir.path().join("assets");
        let bad = write_fixture(dir.path(), "cover.exe", b"nope");
        let err =
            cover_import_inner("front", &bad, &assets_dir, storage.as_ref())
                .await
                .expect_err("must reject");
        assert!(format!("{err:?}").contains("not supported"));
    }

    // ── boilerplate_save_inner E2E ─────────────────────────────────────

    #[tokio::test]
    async fn boilerplate_save_persists_and_loads_back() {
        let (storage, _dir) = fresh_storage().await;
        let input = BoilerplateSaveInput {
            pages: vec![
                page("a", BoilerplateKind::Copyright, "© 2026"),
                page("b", BoilerplateKind::Dedication, "for E."),
            ],
        };
        let r = boilerplate_save_inner(input, storage.as_ref())
            .await
            .expect("save");
        assert_eq!(r.saved_count, 2);

        let loaded = load_boilerplate_inner(storage.as_ref()).await.unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, "a");
        assert_eq!(loaded[1].id, "b");
    }

    #[tokio::test]
    async fn boilerplate_save_replaces_previous_list() {
        let (storage, _dir) = fresh_storage().await;
        boilerplate_save_inner(
            BoilerplateSaveInput {
                pages: vec![page("a", BoilerplateKind::Copyright, "v1")],
            },
            storage.as_ref(),
        )
        .await
        .unwrap();

        boilerplate_save_inner(
            BoilerplateSaveInput {
                pages: vec![page("b", BoilerplateKind::Dedication, "v2")],
            },
            storage.as_ref(),
        )
        .await
        .unwrap();

        let loaded = load_boilerplate_inner(storage.as_ref()).await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "b");
    }

    #[tokio::test]
    async fn boilerplate_save_empty_list_is_a_no_op() {
        let (storage, _dir) = fresh_storage().await;
        let r = boilerplate_save_inner(
            BoilerplateSaveInput { pages: vec![] },
            storage.as_ref(),
        )
        .await
        .expect("empty save");
        assert_eq!(r.saved_count, 0);
        let loaded = load_boilerplate_inner(storage.as_ref()).await.unwrap();
        assert!(loaded.is_empty());
    }

    #[tokio::test]
    async fn boilerplate_save_rejects_unknown_kind() {
        let (storage, _dir) = fresh_storage().await;
        let bad = BoilerplateSaveInput {
            pages: vec![BoilerplatePageDto {
                id: "p".into(),
                kind: "not_a_real_kind".into(),
                title: "x".into(),
                body_md: "y".into(),
                order: 0,
                include_in_export: true,
            }],
        };
        let err = boilerplate_save_inner(bad, storage.as_ref())
            .await
            .expect_err("must reject");
        assert!(format!("{err:?}").to_lowercase().contains("unknown"));
    }

    #[tokio::test]
    async fn boilerplate_save_rejects_oversize_list() {
        let (storage, _dir) = fresh_storage().await;
        let pages: Vec<BoilerplatePageDto> = (0..MAX_BOILERPLATE_PAGES + 1)
            .map(|i| page(&format!("p-{i}"), BoilerplateKind::Other, ""))
            .collect();
        let err = boilerplate_save_inner(BoilerplateSaveInput { pages }, storage.as_ref())
            .await
            .expect_err("must reject");
        assert!(format!("{err:?}").contains("cap is 50"));
    }
}
