//! Windows path comparison: case-insensitive, either separator.

use std::path::Path;

fn norm(p: &Path) -> String {
    p.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

pub(crate) fn same_path(a: &Path, b: &Path) -> bool {
    norm(a) == norm(b)
}

/// True when `path` is `root` itself or lies below it.
pub(crate) fn is_under(path: &Path, root: &Path) -> bool {
    let (path, root) = (norm(path), norm(root));
    path == root || path.starts_with(&format!("{root}\\"))
}
