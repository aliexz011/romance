use anyhow::Result;
use std::path::PathBuf;

/// A marker that should exist in a file — used for pre-validation.
pub struct MarkerCheck {
    pub path: PathBuf,
    pub marker: String,
}

/// Create a `MarkerCheck` from a path and marker string.
pub fn check(path: impl Into<PathBuf>, marker: &str) -> MarkerCheck {
    MarkerCheck {
        path: path.into(),
        marker: marker.to_string(),
    }
}

/// Validate that all expected markers exist in their respective files.
///
/// Returns `Err` listing all missing markers if any are absent.
pub fn validate_markers(checks: &[MarkerCheck]) -> Result<()> {
    let mut missing = Vec::new();

    for c in checks {
        if !c.path.exists() {
            missing.push(format!(
                "File '{}' does not exist (expected marker '{}')",
                c.path.display(),
                c.marker
            ));
            continue;
        }
        let content = std::fs::read_to_string(&c.path)?;
        if !content.contains(&c.marker) {
            missing.push(format!(
                "Marker '{}' not found in '{}'",
                c.marker,
                c.path.display()
            ));
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(
            "Pre-validation failed — {} missing marker(s):\n  {}",
            missing.len(),
            missing.join("\n  ")
        );
    }
}

/// Tracks newly created files during generation for rollback on failure.
pub struct GenerationTracker {
    created_files: Vec<PathBuf>,
}

impl GenerationTracker {
    pub fn new() -> Self {
        Self {
            created_files: Vec::new(),
        }
    }

    /// Record a file that was created during generation.
    pub fn track(&mut self, path: PathBuf) {
        self.created_files.push(path);
    }

    /// Delete all tracked files (best-effort rollback).
    pub fn rollback(&self) {
        for path in &self.created_files {
            if path.exists() {
                if let Err(e) = std::fs::remove_file(path) {
                    eprintln!(
                        "  Warning: failed to clean up '{}': {}",
                        path.display(),
                        e
                    );
                } else {
                    eprintln!("  Rolled back: {}", path.display());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn validate_markers_all_present() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "// === ROMANCE:MODS ===").unwrap();
        writeln!(tmp, "// === ROMANCE:ROUTES ===").unwrap();
        tmp.flush().unwrap();

        let checks = vec![
            check(tmp.path(), "// === ROMANCE:MODS ==="),
            check(tmp.path(), "// === ROMANCE:ROUTES ==="),
        ];
        assert!(validate_markers(&checks).is_ok());
    }

    #[test]
    fn validate_markers_missing_marker() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "// some content").unwrap();
        tmp.flush().unwrap();

        let checks = vec![check(tmp.path(), "// === ROMANCE:MODS ===")];
        let result = validate_markers(&checks);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ROMANCE:MODS"));
    }

    #[test]
    fn validate_markers_missing_file() {
        let checks = vec![check(
            std::path::Path::new("/tmp/romance_nonexistent_12345.rs"),
            "// === ROMANCE:MODS ===",
        )];
        let result = validate_markers(&checks);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn generation_tracker_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("generated.rs");
        std::fs::write(&file_path, "content").unwrap();
        assert!(file_path.exists());

        let mut tracker = GenerationTracker::new();
        tracker.track(file_path.clone());
        tracker.rollback();

        assert!(!file_path.exists());
    }

    #[test]
    fn generation_tracker_rollback_multiple_files() {
        let dir = tempfile::tempdir().unwrap();
        let file_a = dir.path().join("a.rs");
        let file_b = dir.path().join("b.rs");
        let file_c = dir.path().join("c.rs");
        std::fs::write(&file_a, "a").unwrap();
        std::fs::write(&file_b, "b").unwrap();
        std::fs::write(&file_c, "c").unwrap();

        let mut tracker = GenerationTracker::new();
        tracker.track(file_a.clone());
        tracker.track(file_b.clone());
        tracker.track(file_c.clone());
        tracker.rollback();

        assert!(!file_a.exists());
        assert!(!file_b.exists());
        assert!(!file_c.exists());
    }

    #[test]
    fn generation_tracker_rollback_already_deleted_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("gone.rs");
        // Don't create the file — simulate it being deleted already

        let mut tracker = GenerationTracker::new();
        tracker.track(file_path.clone());
        // Should not panic
        tracker.rollback();
    }

    #[test]
    fn validate_markers_reports_all_missing() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "// only this line").unwrap();
        tmp.flush().unwrap();

        let checks = vec![
            check(tmp.path(), "// === ROMANCE:MODS ==="),
            check(tmp.path(), "// === ROMANCE:ROUTES ==="),
        ];
        let result = validate_markers(&checks);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        // Both missing markers should be listed
        assert!(msg.contains("ROMANCE:MODS"), "Error should mention MODS");
        assert!(msg.contains("ROMANCE:ROUTES"), "Error should mention ROUTES");
        assert!(msg.contains("2 missing marker(s)"));
    }

    #[test]
    fn validate_markers_mixed_present_and_missing() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "// === ROMANCE:MODS ===").unwrap();
        tmp.flush().unwrap();

        let checks = vec![
            check(tmp.path(), "// === ROMANCE:MODS ==="),
            check(tmp.path(), "// === ROMANCE:ROUTES ==="),
        ];
        let result = validate_markers(&checks);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("1 missing marker(s)"));
        assert!(msg.contains("ROMANCE:ROUTES"));
        assert!(!msg.contains("ROMANCE:MODS"));
    }
}
