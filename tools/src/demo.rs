//! Build a test bank: the real content plus the made-up demo files in
//! `fixtures/demo-science/`. Used by tests and for trying multi-subject
//! features before real content exists.

use std::path::Path;

/// Copy `content` to `out`, then overlay `demo`: a file whose name already
/// exists is appended to, any other file is copied. `out` is replaced.
pub fn build(content: &Path, demo: &Path, out: &Path) -> Result<(), String> {
    if out.exists() {
        std::fs::remove_dir_all(out).map_err(|e| format!("cannot clear {}: {e}", out.display()))?;
    }
    copy_dir(content, out)?;
    overlay(demo, out)
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(from).map_err(|e| format!("cannot read {}: {e}", from.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &dest)?;
        } else {
            std::fs::copy(&path, &dest).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn overlay(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(from).map_err(|e| format!("cannot read {}: {e}", from.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            overlay(&path, &dest)?;
        } else if path.extension().is_some_and(|x| x == "toml") {
            let extra = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let mut text = std::fs::read_to_string(&dest).unwrap_or_default();
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&extra);
            std::fs::write(&dest, text).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
