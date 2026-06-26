//! File Index — a recursive, gitignore-aware walk that returns every file under `root`
//! as a root-relative path string.
//!
//! Used by the Go-to-file feature (AC-12…AC-15, AC-18, AC-19, AC-N1, AC-N2, AC-N5).
//! This is a separate walk from the Tree Model (ADR-0005): no depth limit, files only,
//! and the entire `.git` subtree is pruned via `filter_entry`.

use ignore::WalkBuilder;
use std::path::Path;

/// Return every file under `root` as a root-relative `String`, respecting `.gitignore`.
///
/// - Recursive (no depth limit) — AC-12.
/// - `.gitignore`-d files are excluded — AC-13.
/// - The `.git` subtree is pruned entirely — AC-14.
/// - Directories are not included, only files — AC-15.
/// - Every returned path is relative to `root` (no leading `/`, no `..`) — AC-N5.
/// - Each call performs a fresh walk; no cache — AC-18.
/// - Works in non-git directories without error (`require_git(false)`) — AC-19.
/// - Read-only: no filesystem or git mutations — AC-N1, AC-N2.
pub fn build(root: &Path) -> Vec<String> {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false) // include dotfiles (AC-17 depends on the index NOT hiding dotfiles)
        .parents(true) // honor ancestor .gitignore for correct nested semantics
        .git_global(false) // hermetic: ignore the user's global gitignore
        .ignore(false) // only git ignore sources, not generic .ignore files
        .require_git(false) // honor .gitignore even outside a git repo (AC-13, AC-19)
        .git_ignore(true)
        .git_exclude(true)
        .filter_entry(|e| e.file_name() != ".git"); // prune entire .git subtree — AC-14

    builder
        .build()
        .filter_map(Result::ok) // skip unreadable entries; traversal continues
        .filter(|e| e.file_type().is_some_and(|t| t.is_file())) // files only — AC-15
        .filter_map(|e| {
            e.path()
                .strip_prefix(root)
                .ok()
                .map(|rel| rel.to_string_lossy().into_owned())
        })
        .collect()
}
