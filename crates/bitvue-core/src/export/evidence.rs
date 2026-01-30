//! Evidence bundle export

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::parity_harness::{OrderType, RenderSnapshot, SelectionSnapshot};

/// Evidence bundle manifest (per export_evidence_bundle.schema.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            bundle_version: "1.0".to_string(),
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
pub fn export_evidence_bundle(
    request: &EvidenceBundleExportRequest,
    render_snapshots: &[RenderSnapshot],
) -> EvidenceBundleExportResult {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let bundle_name = format!("bitvue_evidence_{}", timestamp);
    let bundle_dir = request.output_dir.join(&bundle_name);

    // Create bundle directory
    if let Err(e) = std::fs::create_dir_all(&bundle_dir) {
        return EvidenceBundleExportResult::error(&format!(
            "Failed to create bundle directory: {}",
            e
        ));
    }

    let mut files_created = Vec::new();
    let mut total_bytes = 0;

    // Create manifest
    let manifest = EvidenceBundleManifest {
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
    };

    // Write bundle_manifest.json
    let manifest_path = bundle_dir.join("bundle_manifest.json");
    match write_json_file(&manifest_path, &manifest) {
        Ok(bytes) => {
            files_created.push("bundle_manifest.json".to_string());
            total_bytes += bytes;
        }
        Err(e) => {
            return EvidenceBundleExportResult::error(&format!("Failed to write manifest: {}", e))
        }
    }

    // Write env.json
    let env_info = EnvInfo {
        os: manifest.os.clone(),
        arch: manifest.cpu.clone(),
        gpu: manifest.gpu.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    let env_path = bundle_dir.join("env.json");
    match write_json_file(&env_path, &env_info) {
        Ok(bytes) => {
            files_created.push("env.json".to_string());
            total_bytes += bytes;
        }
        Err(e) => {
            return EvidenceBundleExportResult::error(&format!("Failed to write env.json: {}", e))
        }
    }

    // Write version.json
    let version_info = VersionInfo {
        app_version: manifest.app_version.clone(),
        git_commit: manifest.git_commit.clone(),
        build_profile: manifest.build_profile.clone(),
        bundle_version: manifest.bundle_version.clone(),
    };
    let version_path = bundle_dir.join("version.json");
    match write_json_file(&version_path, &version_info) {
        Ok(bytes) => {
            files_created.push("version.json".to_string());
            total_bytes += bytes;
        }
        Err(e) => {
            return EvidenceBundleExportResult::error(&format!(
                "Failed to write version.json: {}",
                e
            ))
        }
    }

    // Write selection_state.json
    let selection_path = bundle_dir.join("selection_state.json");
    match write_json_file(&selection_path, &request.selection_state) {
        Ok(bytes) => {
            files_created.push("selection_state.json".to_string());
            total_bytes += bytes;
        }
        Err(e) => {
            return EvidenceBundleExportResult::error(&format!(
                "Failed to write selection_state.json: {}",
                e
            ))
        }
    }

    // Write order_type.json
    let order_type_info = OrderTypeInfo {
        order_type: request.order_type,
    };
    let order_type_path = bundle_dir.join("order_type.json");
    match write_json_file(&order_type_path, &order_type_info) {
        Ok(bytes) => {
            files_created.push("order_type.json".to_string());
            total_bytes += bytes;
        }
        Err(e) => {
            return EvidenceBundleExportResult::error(&format!(
                "Failed to write order_type.json: {}",
                e
            ))
        }
    }

    // Write backend_fingerprint.json
    let backend_info = BackendInfo {
        backend: manifest.backend.clone(),
        plugin_versions: manifest.plugin_versions.clone(),
    };
    let backend_path = bundle_dir.join("backend_fingerprint.json");
    match write_json_file(&backend_path, &backend_info) {
        Ok(bytes) => {
            files_created.push("backend_fingerprint.json".to_string());
            total_bytes += bytes;
        }
        Err(e) => {
            return EvidenceBundleExportResult::error(&format!(
                "Failed to write backend_fingerprint.json: {}",
                e
            ))
        }
    }

    // Write warnings.json
    let warnings_path = bundle_dir.join("warnings.json");
    match write_json_file(&warnings_path, &manifest.warnings) {
        Ok(bytes) => {
            files_created.push("warnings.json".to_string());
            total_bytes += bytes;
        }
        Err(e) => {
            return EvidenceBundleExportResult::error(&format!(
                "Failed to write warnings.json: {}",
                e
            ))
        }
    }

    // Write render snapshots if requested
    if request.include_render_snapshots && !render_snapshots.is_empty() {
        let snapshots_dir = bundle_dir.join("render_snapshots");
        if let Err(e) = std::fs::create_dir_all(&snapshots_dir) {
            return EvidenceBundleExportResult::error(&format!(
                "Failed to create snapshots directory: {}",
                e
            ));
        }

        for (idx, snapshot) in render_snapshots.iter().enumerate() {
            let snapshot_path = snapshots_dir.join(format!("snapshot_{:04}.json", idx));
            match write_json_file(&snapshot_path, snapshot) {
                Ok(bytes) => {
                    files_created.push(format!("render_snapshots/snapshot_{:04}.json", idx));
                    total_bytes += bytes;
                }
                Err(e) => {
                    return EvidenceBundleExportResult::error(&format!(
                        "Failed to write snapshot: {}",
                        e
                    ))
                }
            }
        }
    }

    EvidenceBundleExportResult::success(
        bundle_dir.to_string_lossy().to_string(),
        files_created,
        total_bytes,
    )
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
