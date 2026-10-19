//! Evidence bundle export

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::parity_harness::{OrderType, RenderSnapshot, SelectionSnapshot};

/// Current `EvidenceBundleManifest` schema version ("MAJOR.MINOR"). All 16 fields have existed
/// since the manifest's introduction (`eb2cee3`, 2026-01-30) with no schema changes since, so
/// there's no real removal/rename history to encode yet -- this constant plus the policy below
/// exist so the *next* schema change has a place to record itself instead of silently drifting.
///
/// ABI compatibility policy (per `docs/UX_PARITY_MATRIX.md` §7):
/// - **Additions** are backward-compatible and need no MINOR/MAJOR bump: the struct-level
///   `#[serde(default)]` below (backed by `Default for EvidenceBundleManifest`) means a bundle
///   written before a field existed just gets that field's default on read, and an unrecognized
///   field in a newer bundle is silently ignored by an older reader (serde's default behavior,
///   no `deny_unknown_fields`).
/// - **Removals** need a MAJOR bump. `check_bundle_schema_compatibility` compares MAJOR only --
///   a mismatch means the schema had a breaking change and a diff between two such bundles may
///   silently misinterpret repurposed fields, so callers should surface that prominently rather
///   than diffing quietly.
/// - **Renames** should keep the old name working via `#[serde(alias = "old_name")]` on the new
///   field for at least 2 MINOR versions before the MAJOR bump that finally drops the alias.
pub const CURRENT_BUNDLE_SCHEMA_VERSION: &str = "1.0";

/// Result of comparing a bundle's declared `bundle_version` against another (either another
/// bundle's, or [`CURRENT_BUNDLE_SCHEMA_VERSION`]) per the policy on [`CURRENT_BUNDLE_SCHEMA_VERSION`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BundleSchemaCompatibility {
    /// Same MAJOR version -- safe to diff/compare field values directly.
    Compatible,
    /// Different MAJOR version -- a breaking schema change (field removal/repurposing) happened
    /// between these versions.
    IncompatibleMajorVersion { a_major: u32, b_major: u32 },
    /// `bundle_version` wasn't parseable as `MAJOR.MINOR` (pre-versioning or corrupt bundle).
    UnparseableVersion(String),
}

fn parse_major_version(version: &str) -> Option<u32> {
    version.split('.').next()?.parse::<u32>().ok()
}

/// Compares two `bundle_version` strings per this module's ABI policy. Pass
/// [`CURRENT_BUNDLE_SCHEMA_VERSION`] as `b` to check a bundle against the running build's schema,
/// or two real bundles' versions to check whether they're safe to diff against each other.
pub fn check_bundle_schema_compatibility(a: &str, b: &str) -> BundleSchemaCompatibility {
    let (Some(a_major), Some(b_major)) = (parse_major_version(a), parse_major_version(b)) else {
        let bad = if parse_major_version(a).is_none() {
            a
        } else {
            b
        };
        return BundleSchemaCompatibility::UnparseableVersion(bad.to_string());
    };
    if a_major == b_major {
        BundleSchemaCompatibility::Compatible
    } else {
        BundleSchemaCompatibility::IncompatibleMajorVersion { a_major, b_major }
    }
}

/// Evidence bundle manifest (per export_evidence_bundle.schema.json)
///
/// `#[serde(default)]` (container-level, backed by this struct's `Default` impl below) means a
/// bundle missing a field a newer schema added deserializes fine instead of hard-failing -- see
/// [`CURRENT_BUNDLE_SCHEMA_VERSION`]'s doc for the full ABI policy this implements.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EvidenceBundleManifest {
    /// Bundle schema version
    pub bundle_version: String,
    /// Application version
    pub app_version: String,
    /// Git commit hash
    pub git_commit: String,
    /// Build profile (debug/release)
    pub build_profile: String,
    /// Operating system
    pub os: String,
    /// GPU information
    pub gpu: String,
    /// CPU information
    pub cpu: String,
    /// Backend used (e.g., "dav1d")
    pub backend: String,
    /// Plugin versions
    pub plugin_versions: HashMap<String, String>,
    /// Stream fingerprint (hash)
    pub stream_fingerprint: String,
    /// Order type (display/decode)
    pub order_type: OrderType,
    /// Current selection state
    pub selection_state: SelectionSnapshot,
    /// Active workspace
    pub workspace: String,
    /// Current mode
    pub mode: String,
    /// Any warnings
    pub warnings: Vec<String>,
    /// Artifact paths
    pub artifacts: Vec<String>,
}

impl Default for EvidenceBundleManifest {
    fn default() -> Self {
        Self {
            bundle_version: CURRENT_BUNDLE_SCHEMA_VERSION.to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            git_commit: "unknown".to_string(),
            build_profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
            .to_string(),
            os: std::env::consts::OS.to_string(),
            gpu: "unknown".to_string(),
            cpu: std::env::consts::ARCH.to_string(),
            backend: "dav1d".to_string(),
            plugin_versions: HashMap::new(),
            stream_fingerprint: String::new(),
            order_type: OrderType::Display,
            selection_state: SelectionSnapshot {
                selected_entity: None,
                selected_byte_range: None,
                order_type: OrderType::Display,
            },
            workspace: "player".to_string(),
            mode: "normal".to_string(),
            warnings: Vec::new(),
            artifacts: Vec::new(),
        }
    }
}

/// Evidence bundle export request
#[derive(Debug, Clone)]
pub struct EvidenceBundleExportRequest {
    /// Output directory path
    pub output_dir: std::path::PathBuf,
    /// Include screenshots
    pub include_screenshots: bool,
    /// Include render snapshots
    pub include_render_snapshots: bool,
    /// Include interaction trace
    pub include_interaction_trace: bool,
    /// Include logs
    pub include_logs: bool,
    /// Stream fingerprint
    pub stream_fingerprint: String,
    /// Current selection
    pub selection_state: SelectionSnapshot,
    /// Active workspace
    pub workspace: String,
    /// Current mode
    pub mode: String,
    /// Order type
    pub order_type: OrderType,
}

impl Default for EvidenceBundleExportRequest {
    fn default() -> Self {
        Self {
            output_dir: std::path::PathBuf::from("."),
            include_screenshots: true,
            include_render_snapshots: true,
            include_interaction_trace: false,
            include_logs: false,
            stream_fingerprint: String::new(),
            selection_state: SelectionSnapshot {
                selected_entity: None,
                selected_byte_range: None,
                order_type: OrderType::Display,
            },
            workspace: "player".to_string(),
            mode: "normal".to_string(),
            order_type: OrderType::Display,
        }
    }
}

/// Evidence bundle export result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceBundleExportResult {
    /// Export success
    pub success: bool,
    /// Bundle directory path
    pub bundle_path: Option<String>,
    /// Files created
    pub files_created: Vec<String>,
    /// Total bytes written
    pub total_bytes: usize,
    /// Error message (if any)
    pub error: Option<String>,
}

impl EvidenceBundleExportResult {
    /// Create success result
    pub fn success(bundle_path: String, files: Vec<String>, bytes: usize) -> Self {
        Self {
            success: true,
            bundle_path: Some(bundle_path),
            files_created: files,
            total_bytes: bytes,
            error: None,
        }
    }

    /// Create error result
    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            bundle_path: None,
            files_created: Vec::new(),
            total_bytes: 0,
            error: Some(message.to_string()),
        }
    }
}

/// Export evidence bundle to directory
///
/// Creates a bundle directory containing:
/// - bundle_manifest.json
/// - env.json
/// - selection_state.json
/// - order_type.json
/// - warnings.json
/// - screenshots/ (optional)
/// - render_snapshots/ (optional)
///
/// Per export_entrypoints.json, this must be reachable from:
/// - MainMenu > File > Export > Evidence Bundle
/// - BottomBar > Export
/// - ContextMenu > Export Evidence Bundle
/// - CompareWorkspace > Toolbar > Export Diff Bundle
///
/// `screenshots` is raw PNG bytes, one entry per captured image -- decoding a base64/data-URL
/// screenshot (e.g. from Electron's `capturePage()`) is the caller's job, matching how
/// `render_snapshots` is likewise already-decoded data by the time it reaches this function.
pub fn export_evidence_bundle(
    request: &EvidenceBundleExportRequest,
    render_snapshots: &[RenderSnapshot],
    screenshots: &[Vec<u8>],
) -> EvidenceBundleExportResult {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let bundle_name = format!("bitvue_evidence_{}", timestamp);
    let bundle_dir = request.output_dir.join(&bundle_name);

    if let Err(e) = std::fs::create_dir_all(&bundle_dir) {
        return EvidenceBundleExportResult::error(&format!(
            "Failed to create bundle directory: {}",
            e
        ));
    }

    let manifest = build_manifest(request);
    let mut writer = BundleWriter::new(&bundle_dir);
    let written = write_metadata_files(&mut writer, request, &manifest)
        .and_then(|()| write_screenshots(&mut writer, request, screenshots))
        .and_then(|()| write_render_snapshots(&mut writer, request, render_snapshots));
    match written {
        Ok(()) => EvidenceBundleExportResult::success(
            bundle_dir.to_string_lossy().to_string(),
            writer.files,
            writer.total_bytes,
        ),
        Err(message) => EvidenceBundleExportResult::error(&message),
    }
}

/// Manifest for this build and request. `warnings`/`artifacts` start empty; the export does not
/// collect any.
fn build_manifest(request: &EvidenceBundleExportRequest) -> EvidenceBundleManifest {
    EvidenceBundleManifest {
        bundle_version: "1.0".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        git_commit: option_env!("GIT_COMMIT").unwrap_or("unknown").to_string(),
        build_profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
        .to_string(),
        os: std::env::consts::OS.to_string(),
        gpu: "unknown".to_string(),
        cpu: std::env::consts::ARCH.to_string(),
        backend: "dav1d".to_string(),
        plugin_versions: HashMap::new(),
        stream_fingerprint: request.stream_fingerprint.clone(),
        order_type: request.order_type,
        selection_state: request.selection_state.clone(),
        workspace: request.workspace.clone(),
        mode: request.mode.clone(),
        warnings: Vec::new(),
        artifacts: Vec::new(),
    }
}

/// Files written into a bundle directory so far, and their combined size.
struct BundleWriter<'a> {
    dir: &'a Path,
    files: Vec<String>,
    total_bytes: usize,
}

impl<'a> BundleWriter<'a> {
    fn new(dir: &'a Path) -> Self {
        Self {
            dir,
            files: Vec::new(),
            total_bytes: 0,
        }
    }

    /// Writes `data` as pretty JSON to `relative_path`; `what` names the file in the error.
    fn json<T: Serialize>(
        &mut self,
        relative_path: &str,
        data: &T,
        what: &str,
    ) -> Result<(), String> {
        let bytes = write_json_file(&self.dir.join(relative_path), data)
            .map_err(|e| format!("Failed to write {}: {}", what, e))?;
        self.files.push(relative_path.to_string());
        self.total_bytes += bytes;
        Ok(())
    }

    fn create_dir(&self, name: &str, what: &str) -> Result<std::path::PathBuf, String> {
        let dir = self.dir.join(name);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create {} directory: {}", what, e))?;
        Ok(dir)
    }
}

/// The seven fixed files, in the order they appear in `files_created`.
fn write_metadata_files(
    writer: &mut BundleWriter,
    request: &EvidenceBundleExportRequest,
    manifest: &EvidenceBundleManifest,
) -> Result<(), String> {
    writer.json("bundle_manifest.json", manifest, "manifest")?;

    let env_info = EnvInfo {
        os: manifest.os.clone(),
        arch: manifest.cpu.clone(),
        gpu: manifest.gpu.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    writer.json("env.json", &env_info, "env.json")?;

    let version_info = VersionInfo {
        app_version: manifest.app_version.clone(),
        git_commit: manifest.git_commit.clone(),
        build_profile: manifest.build_profile.clone(),
        bundle_version: manifest.bundle_version.clone(),
    };
    writer.json("version.json", &version_info, "version.json")?;

    writer.json(
        "selection_state.json",
        &request.selection_state,
        "selection_state.json",
    )?;

    let order_type_info = OrderTypeInfo {
        order_type: request.order_type,
    };
    writer.json("order_type.json", &order_type_info, "order_type.json")?;

    let backend_info = BackendInfo {
        backend: manifest.backend.clone(),
        plugin_versions: manifest.plugin_versions.clone(),
    };
    writer.json(
        "backend_fingerprint.json",
        &backend_info,
        "backend_fingerprint.json",
    )?;

    writer.json("warnings.json", &manifest.warnings, "warnings.json")
}

/// `screenshots/screenshot_NNNN.png`, only when requested and non-empty.
fn write_screenshots(
    writer: &mut BundleWriter,
    request: &EvidenceBundleExportRequest,
    screenshots: &[Vec<u8>],
) -> Result<(), String> {
    if !request.include_screenshots || screenshots.is_empty() {
        return Ok(());
    }
    let dir = writer.create_dir("screenshots", "screenshots")?;
    for (idx, png_bytes) in screenshots.iter().enumerate() {
        std::fs::write(dir.join(format!("screenshot_{:04}.png", idx)), png_bytes)
            .map_err(|e| format!("Failed to write screenshot: {}", e))?;
        writer
            .files
            .push(format!("screenshots/screenshot_{:04}.png", idx));
        writer.total_bytes += png_bytes.len();
    }
    Ok(())
}

/// `render_snapshots/snapshot_NNNN.json`, only when requested and non-empty.
fn write_render_snapshots(
    writer: &mut BundleWriter,
    request: &EvidenceBundleExportRequest,
    render_snapshots: &[RenderSnapshot],
) -> Result<(), String> {
    if !request.include_render_snapshots || render_snapshots.is_empty() {
        return Ok(());
    }
    writer.create_dir("render_snapshots", "snapshots")?;
    for (idx, snapshot) in render_snapshots.iter().enumerate() {
        let relative = format!("render_snapshots/snapshot_{:04}.json", idx);
        writer.json(&relative, snapshot, "snapshot")?;
    }
    Ok(())
}

/// Environment info for evidence bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EnvInfo {
    os: String,
    arch: String,
    gpu: String,
    timestamp: String,
}

/// Version info for evidence bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionInfo {
    app_version: String,
    git_commit: String,
    build_profile: String,
    bundle_version: String,
}

/// Order type info for evidence bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrderTypeInfo {
    order_type: OrderType,
}

/// Backend info for evidence bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackendInfo {
    backend: String,
    plugin_versions: HashMap<String, String>,
}

/// Helper to write JSON file
fn write_json_file<T: Serialize>(path: &Path, data: &T) -> std::io::Result<usize> {
    let json = serde_json::to_string_pretty(data).map_err(std::io::Error::other)?;
    std::fs::write(path, &json)?;
    Ok(json.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parity_harness::{EntityRef, ViewportState};
    use std::path::PathBuf;

    const FIXED_FILES: [&str; 7] = [
        "bundle_manifest.json",
        "env.json",
        "version.json",
        "selection_state.json",
        "order_type.json",
        "backend_fingerprint.json",
        "warnings.json",
    ];

    fn request(dir: &Path) -> EvidenceBundleExportRequest {
        EvidenceBundleExportRequest {
            output_dir: dir.to_path_buf(),
            stream_fingerprint: "fp-abc".to_string(),
            workspace: "compare".to_string(),
            mode: "diff".to_string(),
            order_type: OrderType::Decode,
            selection_state: SelectionSnapshot {
                selected_entity: Some(EntityRef {
                    kind: "frame".to_string(),
                    id: "f7".to_string(),
                    frame_index: Some(7),
                    byte_offset: Some(1234),
                }),
                selected_byte_range: Some((10, 20)),
                order_type: OrderType::Decode,
            },
            ..EvidenceBundleExportRequest::default()
        }
    }

    fn snapshot(panel: &str) -> RenderSnapshot {
        RenderSnapshot {
            workspace: "player".to_string(),
            panel: panel.to_string(),
            mode: "normal".to_string(),
            codec: "av1".to_string(),
            order_type: OrderType::Display,
            viewport: ViewportState {
                x: 0.0,
                y: 0.0,
                width: 64.0,
                height: 32.0,
                zoom: 1.0,
                pan_x: 0.0,
                pan_y: 0.0,
            },
            selection_txn: "txn".to_string(),
            layer_stack: vec!["base".to_string()],
            objects: vec![],
            legend: None,
            warnings: vec![],
            backend_fingerprint: "dav1d".to_string(),
        }
    }

    fn json(bundle: &str, name: &str) -> serde_json::Value {
        let text = std::fs::read_to_string(Path::new(bundle).join(name)).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    fn export(
        req: &EvidenceBundleExportRequest,
        snapshots: &[RenderSnapshot],
        shots: &[Vec<u8>],
    ) -> EvidenceBundleExportResult {
        let result = export_evidence_bundle(req, snapshots, shots);
        assert!(result.success, "{:?}", result.error);
        result
    }

    #[test]
    fn minimal_export_writes_the_seven_fixed_files_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let result = export(&request(dir.path()), &[], &[]);
        assert_eq!(result.files_created, FIXED_FILES);
        assert_eq!(result.error, None);

        let bundle = PathBuf::from(result.bundle_path.clone().unwrap());
        assert_eq!(bundle.parent().unwrap(), dir.path());
        let name = bundle.file_name().unwrap().to_str().unwrap();
        // bitvue_evidence_YYYYMMDD_HHMMSS
        assert_eq!(name.len(), "bitvue_evidence_".len() + 15, "{name}");
        assert!(name.starts_with("bitvue_evidence_"));
        assert_eq!(name.as_bytes()["bitvue_evidence_".len() + 8], b'_');

        let on_disk: usize = FIXED_FILES
            .iter()
            .map(|f| std::fs::metadata(bundle.join(f)).unwrap().len() as usize)
            .sum();
        assert_eq!(result.total_bytes, on_disk);
        assert!(!bundle.join("screenshots").exists());
        assert!(!bundle.join("render_snapshots").exists());
        assert_eq!(std::fs::read_dir(&bundle).unwrap().count(), 7);
    }

    #[test]
    fn manifest_carries_the_request_fields_and_build_constants() {
        let dir = tempfile::tempdir().unwrap();
        let result = export(&request(dir.path()), &[], &[]);
        let b = result.bundle_path.unwrap();
        let m = json(&b, "bundle_manifest.json");
        assert_eq!(m["bundle_version"], CURRENT_BUNDLE_SCHEMA_VERSION);
        assert_eq!(m["app_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(m["backend"], "dav1d");
        assert_eq!(m["gpu"], "unknown");
        assert_eq!(m["os"], std::env::consts::OS);
        assert_eq!(m["cpu"], std::env::consts::ARCH);
        assert_eq!(
            m["build_profile"],
            if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
        );
        assert_eq!(m["stream_fingerprint"], "fp-abc");
        assert_eq!(m["workspace"], "compare");
        assert_eq!(m["mode"], "diff");
        assert_eq!(m["order_type"], "decode");
        assert_eq!(m["selection_state"]["selected_entity"]["id"], "f7");
        assert_eq!(m["selection_state"]["selected_byte_range"][1], 20);
        assert_eq!(m["warnings"], serde_json::json!([]));
        assert_eq!(m["artifacts"], serde_json::json!([]));
        assert_eq!(m["plugin_versions"], serde_json::json!({}));
        // Round-trips through the public type.
        let typed: EvidenceBundleManifest = serde_json::from_value(m).unwrap();
        assert_eq!(typed.order_type, OrderType::Decode);
    }

    #[test]
    fn side_files_repeat_the_manifest_values() {
        let dir = tempfile::tempdir().unwrap();
        let result = export(&request(dir.path()), &[], &[]);
        let b = result.bundle_path.unwrap();
        let m = json(&b, "bundle_manifest.json");

        let env = json(&b, "env.json");
        assert_eq!(env["os"], m["os"]);
        assert_eq!(env["arch"], m["cpu"]);
        assert_eq!(env["gpu"], m["gpu"]);
        assert!(chrono::DateTime::parse_from_rfc3339(env["timestamp"].as_str().unwrap()).is_ok());

        let version = json(&b, "version.json");
        for key in [
            "app_version",
            "git_commit",
            "build_profile",
            "bundle_version",
        ] {
            assert_eq!(version[key], m[key], "{key}");
        }

        assert_eq!(json(&b, "selection_state.json"), m["selection_state"]);
        assert_eq!(
            json(&b, "order_type.json"),
            serde_json::json!({"order_type": "decode"})
        );
        let backend = json(&b, "backend_fingerprint.json");
        assert_eq!(backend["backend"], "dav1d");
        assert_eq!(backend["plugin_versions"], serde_json::json!({}));
        assert_eq!(json(&b, "warnings.json"), serde_json::json!([]));
    }

    #[test]
    fn screenshots_are_written_verbatim_and_counted() {
        let dir = tempfile::tempdir().unwrap();
        let shots = vec![vec![1u8, 2, 3], vec![], vec![9u8; 100]];
        let result = export(&request(dir.path()), &[], &shots);
        let b = PathBuf::from(result.bundle_path.clone().unwrap());

        let mut expected: Vec<String> = FIXED_FILES.iter().map(|s| s.to_string()).collect();
        expected.extend((0..3).map(|i| format!("screenshots/screenshot_{i:04}.png")));
        assert_eq!(result.files_created, expected);
        for (i, bytes) in shots.iter().enumerate() {
            let on_disk =
                std::fs::read(b.join(format!("screenshots/screenshot_{i:04}.png"))).unwrap();
            assert_eq!(&on_disk, bytes);
        }
        let fixed: usize = FIXED_FILES
            .iter()
            .map(|f| std::fs::metadata(b.join(f)).unwrap().len() as usize)
            .sum();
        assert_eq!(result.total_bytes, fixed + 103);
    }

    #[test]
    fn render_snapshots_are_written_as_json_in_input_order() {
        let dir = tempfile::tempdir().unwrap();
        let snaps = vec![snapshot("main"), snapshot("side")];
        let result = export(&request(dir.path()), &snaps, &[]);
        let b = PathBuf::from(result.bundle_path.clone().unwrap());

        assert_eq!(
            &result.files_created[7..],
            [
                "render_snapshots/snapshot_0000.json",
                "render_snapshots/snapshot_0001.json"
            ]
        );
        assert_eq!(
            json(b.to_str().unwrap(), "render_snapshots/snapshot_0000.json")["panel"],
            "main"
        );
        assert_eq!(
            json(b.to_str().unwrap(), "render_snapshots/snapshot_0001.json")["panel"],
            "side"
        );
        let all: usize = result
            .files_created
            .iter()
            .map(|f| std::fs::metadata(b.join(f)).unwrap().len() as usize)
            .sum();
        assert_eq!(result.total_bytes, all);
    }

    #[test]
    fn screenshots_come_before_snapshots_in_the_file_list() {
        let dir = tempfile::tempdir().unwrap();
        let result = export(&request(dir.path()), &[snapshot("p")], &[vec![0u8]]);
        assert_eq!(result.files_created[7], "screenshots/screenshot_0000.png");
        assert_eq!(
            result.files_created[8],
            "render_snapshots/snapshot_0000.json"
        );
        assert_eq!(result.files_created.len(), 9);
    }

    #[test]
    fn include_flags_off_skip_the_optional_directories_even_with_data() {
        let dir = tempfile::tempdir().unwrap();
        let mut req = request(dir.path());
        req.include_screenshots = false;
        req.include_render_snapshots = false;
        let result = export(&req, &[snapshot("p")], &[vec![1u8]]);
        assert_eq!(result.files_created, FIXED_FILES);
        let b = PathBuf::from(result.bundle_path.unwrap());
        assert!(!b.join("screenshots").exists());
        assert!(!b.join("render_snapshots").exists());
    }

    #[test]
    fn each_optional_flag_is_independent() {
        let dir = tempfile::tempdir().unwrap();
        let mut req = request(dir.path());
        req.include_screenshots = false;
        let result = export(&req, &[snapshot("p")], &[vec![1u8]]);
        assert_eq!(
            result.files_created[7],
            "render_snapshots/snapshot_0000.json"
        );
        assert_eq!(result.files_created.len(), 8);

        let dir = tempfile::tempdir().unwrap();
        let mut req = request(dir.path());
        req.include_render_snapshots = false;
        let result = export(&req, &[snapshot("p")], &[vec![1u8]]);
        assert_eq!(result.files_created[7], "screenshots/screenshot_0000.png");
        assert_eq!(result.files_created.len(), 8);
    }

    #[test]
    fn interaction_trace_and_logs_flags_add_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let mut req = request(dir.path());
        req.include_interaction_trace = true;
        req.include_logs = true;
        let result = export(&req, &[], &[]);
        assert_eq!(result.files_created, FIXED_FILES);
    }

    #[test]
    fn output_dir_that_is_a_file_reports_the_bundle_directory_error() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("not_a_dir");
        std::fs::write(&blocker, b"x").unwrap();
        let result = export_evidence_bundle(&request(&blocker), &[], &[]);
        assert!(!result.success);
        assert_eq!(result.bundle_path, None);
        assert!(result.files_created.is_empty());
        assert_eq!(result.total_bytes, 0);
        assert!(result
            .error
            .unwrap()
            .starts_with("Failed to create bundle directory: "));
    }

    #[test]
    fn missing_output_dir_is_created_with_its_parents() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");
        let result = export(&request(&nested), &[], &[]);
        assert!(Path::new(&result.bundle_path.unwrap()).starts_with(&nested));
    }

    /// Pre-creating a directory where a file should go makes exactly that write fail.
    fn blocked(dir: &Path, name: &str) {
        std::fs::create_dir_all(dir.join(name)).unwrap();
    }

    fn run_writers(
        dir: &Path,
        req: &EvidenceBundleExportRequest,
        snaps: &[RenderSnapshot],
        shots: &[Vec<u8>],
    ) -> Result<Vec<String>, String> {
        let manifest = build_manifest(req);
        let mut writer = BundleWriter::new(dir);
        write_metadata_files(&mut writer, req, &manifest)?;
        write_screenshots(&mut writer, req, shots)?;
        write_render_snapshots(&mut writer, req, snaps)?;
        Ok(writer.files)
    }

    #[test]
    fn every_fixed_file_names_itself_in_its_write_error() {
        let labels = [
            ("bundle_manifest.json", "manifest", 0),
            ("env.json", "env.json", 1),
            ("version.json", "version.json", 2),
            ("selection_state.json", "selection_state.json", 3),
            ("order_type.json", "order_type.json", 4),
            ("backend_fingerprint.json", "backend_fingerprint.json", 5),
            ("warnings.json", "warnings.json", 6),
        ];
        for (file, label, written_before) in labels {
            let dir = tempfile::tempdir().unwrap();
            blocked(dir.path(), file);
            let req = request(dir.path());
            let manifest = build_manifest(&req);
            let mut writer = BundleWriter::new(dir.path());
            let err = write_metadata_files(&mut writer, &req, &manifest).unwrap_err();
            assert!(
                err.starts_with(&format!("Failed to write {label}: ")),
                "{file}: {err}"
            );
            // Files before the failing one were recorded; nothing after it was attempted.
            assert_eq!(writer.files, FIXED_FILES[..written_before], "{file}");
        }
    }

    #[test]
    fn screenshots_directory_blocked_by_a_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("screenshots"), b"x").unwrap();
        let err = run_writers(dir.path(), &request(dir.path()), &[], &[vec![1]])
            .err()
            .unwrap();
        assert!(
            err.starts_with("Failed to create screenshots directory: "),
            "{err}"
        );
    }

    #[test]
    fn a_failing_screenshot_stops_the_export_after_the_earlier_ones() {
        let dir = tempfile::tempdir().unwrap();
        blocked(&dir.path().join("screenshots"), "screenshot_0001.png");
        let req = request(dir.path());
        let manifest = build_manifest(&req);
        let mut writer = BundleWriter::new(dir.path());
        write_metadata_files(&mut writer, &req, &manifest).unwrap();
        let err = write_screenshots(&mut writer, &req, &[vec![1], vec![2], vec![3]]).unwrap_err();
        assert!(err.starts_with("Failed to write screenshot: "), "{err}");
        assert_eq!(
            writer.files.last().unwrap(),
            "screenshots/screenshot_0000.png"
        );
        assert_eq!(writer.files.len(), 8);
        assert!(!dir.path().join("screenshots/screenshot_0002.png").exists());
    }

    #[test]
    fn snapshots_directory_blocked_by_a_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("render_snapshots"), b"x").unwrap();
        let err = run_writers(dir.path(), &request(dir.path()), &[snapshot("p")], &[])
            .err()
            .unwrap();
        assert!(
            err.starts_with("Failed to create snapshots directory: "),
            "{err}"
        );
    }

    #[test]
    fn a_failing_snapshot_stops_the_export_after_the_earlier_ones() {
        let dir = tempfile::tempdir().unwrap();
        blocked(&dir.path().join("render_snapshots"), "snapshot_0001.json");
        let req = request(dir.path());
        let manifest = build_manifest(&req);
        let mut writer = BundleWriter::new(dir.path());
        write_metadata_files(&mut writer, &req, &manifest).unwrap();
        let snaps = [snapshot("a"), snapshot("b"), snapshot("c")];
        let err = write_render_snapshots(&mut writer, &req, &snaps).unwrap_err();
        assert!(err.starts_with("Failed to write snapshot: "), "{err}");
        assert_eq!(
            writer.files.last().unwrap(),
            "render_snapshots/snapshot_0000.json"
        );
        assert!(!dir
            .path()
            .join("render_snapshots/snapshot_0002.json")
            .exists());
    }

    #[test]
    fn skipped_writers_leave_no_trace() {
        let dir = tempfile::tempdir().unwrap();
        let mut req = request(dir.path());
        req.include_screenshots = false;
        req.include_render_snapshots = false;
        let files = run_writers(dir.path(), &req, &[snapshot("p")], &[vec![1]]).unwrap();
        assert_eq!(files, FIXED_FILES);
        assert!(!dir.path().join("screenshots").exists());
    }

    #[test]
    fn result_constructors_fill_every_field() {
        let ok = EvidenceBundleExportResult::success("p".into(), vec!["a".into()], 5);
        assert!(ok.success && ok.error.is_none());
        assert_eq!(ok.bundle_path.as_deref(), Some("p"));
        assert_eq!(
            (ok.files_created, ok.total_bytes),
            (vec!["a".to_string()], 5)
        );
        let err = EvidenceBundleExportResult::error("boom");
        assert!(!err.success && err.bundle_path.is_none() && err.files_created.is_empty());
        assert_eq!((err.total_bytes, err.error.as_deref()), (0, Some("boom")));
    }

    #[test]
    fn schema_compatibility_compares_major_only() {
        use BundleSchemaCompatibility::*;
        assert_eq!(check_bundle_schema_compatibility("1.0", "1.9"), Compatible);
        assert_eq!(
            check_bundle_schema_compatibility("1.0", "2.0"),
            IncompatibleMajorVersion {
                a_major: 1,
                b_major: 2
            }
        );
        assert_eq!(
            check_bundle_schema_compatibility("x", "1.0"),
            UnparseableVersion("x".into())
        );
        assert_eq!(
            check_bundle_schema_compatibility("1.0", ""),
            UnparseableVersion("".into())
        );
        // `a` is reported first when both are bad.
        assert_eq!(
            check_bundle_schema_compatibility("a", "b"),
            UnparseableVersion("a".into())
        );
    }

    #[test]
    fn write_json_file_returns_the_byte_count_and_fails_on_a_missing_parent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.json");
        let n = write_json_file(&path, &vec![1, 2]).unwrap();
        assert_eq!(n as u64, std::fs::metadata(&path).unwrap().len());
        assert!(write_json_file(&dir.path().join("no/such/f.json"), &1).is_err());
    }
}
