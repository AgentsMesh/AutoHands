//! File system tool implementations.

use std::path::{Path, PathBuf};

use autohands_protocols::error::ToolError;

mod read_file;
mod write_file;
mod edit_file;
mod list_dir;
mod create_dir;
mod delete_file;
mod move_file;

pub use read_file::ReadFileTool;
pub use write_file::WriteFileTool;
pub use edit_file::EditFileTool;
pub use list_dir::ListDirectoryTool;
pub use create_dir::CreateDirectoryTool;
pub use delete_file::DeleteFileTool;
pub use move_file::MoveFileTool;

/// Resolve a user-supplied path relative to `work_dir`, then verify it does not
/// escape the sandbox via `..` segments or symlinks.
///
/// Returns the canonicalized path on success, or `ToolError::ExecutionFailed`
/// with "Path traversal denied" when the resolved path falls outside `work_dir`.
///
/// For paths whose target does not yet exist (write / create_dir), we
/// canonicalize the longest existing ancestor and append the remaining suffix.
pub(crate) fn resolve_path_safe(path: &str, work_dir: &Path) -> Result<PathBuf, ToolError> {
    // Canonicalize work_dir first (must exist).
    let canon_work = work_dir
        .canonicalize()
        .map_err(|e| ToolError::ExecutionFailed(format!("Cannot resolve work_dir: {}", e)))?;

    let raw = PathBuf::from(path);
    // Join relative paths against canonicalized work_dir to avoid symlink mismatch.
    let joined = if raw.is_absolute() { raw } else { canon_work.join(raw) };

    // Try to canonicalize the full path. If it exists, great.
    if let Ok(canon) = joined.canonicalize() {
        return if canon.starts_with(&canon_work) {
            Ok(canon)
        } else {
            Err(ToolError::ExecutionFailed("Path traversal denied".to_string()))
        };
    }

    // Path does not exist yet -- walk up to the nearest existing ancestor,
    // canonicalize it, then re-attach remaining components after normalizing
    // them to strip `.` and `..`.
    let normalized = normalize_path(&joined);

    // Walk up the normalized path to find the longest existing prefix,
    // canonicalize that prefix (resolves symlinks), and rebuild the tail.
    let mut existing = normalized.as_path();
    let mut tail: Vec<&std::ffi::OsStr> = Vec::new();
    loop {
        if existing.exists() {
            break;
        }
        match (existing.file_name(), existing.parent()) {
            (Some(name), Some(parent)) => {
                tail.push(name);
                existing = parent;
            }
            _ => break,
        }
    }
    let mut resolved = existing
        .canonicalize()
        .map_err(|e| ToolError::ExecutionFailed(format!("Cannot resolve path: {}", e)))?;
    for part in tail.into_iter().rev() {
        resolved.push(part);
    }

    if resolved.starts_with(&canon_work) {
        Ok(resolved)
    } else {
        Err(ToolError::ExecutionFailed("Path traversal denied".to_string()))
    }
}

/// Normalize a path by resolving `.` and `..` components without touching the filesystem.
/// This is used for paths that do not yet exist.
fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => { /* skip `.` */ }
            Component::ParentDir => {
                // Pop the last component if possible; otherwise keep the `..`
                if !result.pop() {
                    result.push(component);
                }
            }
            other => result.push(other),
        }
    }
    result
}

#[cfg(test)]
mod path_traversal_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_safe_relative_path() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("hello.txt"), "hi").unwrap();
        let resolved = resolve_path_safe("hello.txt", tmp.path()).unwrap();
        assert!(resolved.starts_with(tmp.path().canonicalize().unwrap()));
    }

    #[test]
    fn test_resolve_safe_absolute_path_within() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("a.txt");
        std::fs::write(&file, "data").unwrap();
        let resolved = resolve_path_safe(file.to_str().unwrap(), tmp.path()).unwrap();
        assert!(resolved.starts_with(tmp.path().canonicalize().unwrap()));
    }

    #[test]
    fn test_reject_dot_dot_traversal() {
        let tmp = TempDir::new().unwrap();
        let result = resolve_path_safe("../../../etc/passwd", tmp.path());
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Path traversal denied"));
    }

    #[test]
    fn test_reject_absolute_outside_workdir() {
        let tmp = TempDir::new().unwrap();
        let result = resolve_path_safe("/etc/passwd", tmp.path());
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Path traversal denied"));
    }

    #[test]
    fn test_reject_complex_traversal() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("a/b")).unwrap();
        let result = resolve_path_safe("a/b/../../../etc/passwd", tmp.path());
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Path traversal denied"));
    }

    #[test]
    fn test_allow_nonexistent_path_within_workdir() {
        let tmp = TempDir::new().unwrap();
        // Path does not exist yet but is within work_dir
        let resolved = resolve_path_safe("new_dir/new_file.txt", tmp.path()).unwrap();
        assert!(resolved.starts_with(tmp.path().canonicalize().unwrap()));
    }

    #[test]
    fn test_reject_traversal_via_nonexistent_intermediate() {
        let tmp = TempDir::new().unwrap();
        let result = resolve_path_safe("nonexistent/../../etc/passwd", tmp.path());
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Path traversal denied"));
    }

    #[cfg(unix)]
    #[test]
    fn test_reject_symlink_escape() {
        let tmp = TempDir::new().unwrap();
        let link_path = tmp.path().join("escape_link");
        std::os::unix::fs::symlink("/etc", &link_path).unwrap();
        let result = resolve_path_safe("escape_link/passwd", tmp.path());
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Path traversal denied"));
    }
}
