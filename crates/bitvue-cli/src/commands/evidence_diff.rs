//! Diff two Evidence Bundle directories (per `docs/UX_PARITY_MATRIX.md` §7's diff contract).
//!
//! First real caller of `bitvue_engine::parity_harness::compare_evidence_bundle_dirs` -- until
//! now the diff logic only had unit-test callers (see `docs/PARITY_CHECKLIST.md` EVB-01), since
//! its natural UI host (CompareWorkspace's toolbar) is itself unmounted. Bundles are produced by
//! Bitvue's Electron "Export Evidence Bundle" menu item; this reads two exported directories.

use anyhow::Result;
use bitvue_engine::parity_harness::{
    compare_evidence_bundle_dirs, DiffSeverity, EvidenceDiffConfig,
};
use std::path::PathBuf;

pub fn run(bundle_a: PathBuf, bundle_b: PathBuf, strict: bool) -> Result<()> {
    for dir in [&bundle_a, &bundle_b] {
        if !dir.join("bundle_manifest.json").exists() {
            anyhow::bail!(
                "{} does not look like an evidence bundle directory (no bundle_manifest.json)",
                dir.display()
            );
        }
    }

    let config = EvidenceDiffConfig::default();
    let result = compare_evidence_bundle_dirs(&bundle_a, &bundle_b, &config).map_err(|e| {
        anyhow::anyhow!(
            "comparing {} vs {}: {e}",
            bundle_a.display(),
            bundle_b.display()
        )
    })?;

    println!("A: {}", bundle_a.display());
    println!("B: {}", bundle_b.display());

    if result.matches {
        println!("Status: MATCH — no differences in compared fields");
    } else {
        for diff in &result.differences {
            let label = match diff.severity {
                DiffSeverity::Breaking => "BREAKING",
                DiffSeverity::Warning => "WARNING ",
                DiffSeverity::Info => "INFO    ",
            };
            println!(
                "{label} {}: '{}' -> '{}'",
                diff.field, diff.a_value, diff.b_value
            );
        }
        println!(
            "\nStatus: DIFFERS ({} difference(s)), ABI compatible: {}",
            result.differences.len(),
            result.abi_compatible
        );
    }

    for ignored in &result.ignored_changes {
        println!("(ignored) {ignored}");
    }

    if strict && !result.matches {
        anyhow::bail!(
            "evidence bundles differ ({} field(s))",
            result.differences.len()
        );
    }

    Ok(())
}
