use anyhow::Result;
use std::fs;
use std::path::Path;

/// Write content to a file, creating parent directories as needed.
pub fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

/// Read a file and split at the ROMANCE:CUSTOM marker.
/// Returns (before_marker, custom_block) where custom_block includes the marker line.
pub fn read_with_custom_block(path: &Path) -> Option<(String, String)> {
    let content = fs::read_to_string(path).ok()?;
    let marker = "// === ROMANCE:CUSTOM ===";
    if let Some(pos) = content.find(marker) {
        Some((content[..pos].to_string(), content[pos..].to_string()))
    } else {
        None
    }
}

/// Write generated content, preserving custom block if file already exists.
pub fn write_generated(path: &Path, generated: &str) -> Result<()> {
    let content = if let Some((_, custom_block)) = read_with_custom_block(path) {
        format!("{}{}", generated, custom_block)
    } else {
        generated.to_string()
    };
    write_file(path, &content)
}

/// Insert a line before a named marker in a file.
///
/// Returns an error if the marker is not found in the file.
pub fn insert_at_marker(path: &Path, marker: &str, line: &str) -> Result<()> {
    let content = fs::read_to_string(path)?;
    if content.contains(line) {
        return Ok(());
    }
    if !content.contains(marker) {
        anyhow::bail!(
            "Marker '{}' not found in {}",
            marker,
            path.display()
        );
    }
    let new_content = content.replace(marker, &format!("{}\n{}", line, marker));
    fs::write(path, new_content)?;
    Ok(())
}

/// Pluralize an English word (same rules as the Tera `plural` filter).
pub fn pluralize(s: &str) -> String {
    if s.ends_with('s') || s.ends_with('x') || s.ends_with("ch") || s.ends_with("sh") {
        format!("{}es", s)
    } else if s.ends_with('y')
        && !s.ends_with("ay")
        && !s.ends_with("ey")
        && !s.ends_with("oy")
        && !s.ends_with("uy")
    {
        format!("{}ies", &s[..s.len() - 1])
    } else {
        format!("{}s", s)
    }
}

/// Rust reserved keywords that must be escaped with `r#` when used as identifiers.
pub const RUST_RESERVED_WORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
    "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
    "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
    "trait", "true", "type", "unsafe", "use", "where", "while", "yield",
    // Reserved for future use
    "abstract", "become", "box", "do", "final", "macro", "override", "priv", "try",
    "typeof", "unsized", "virtual",
];

/// Escape a field name for use as a Rust identifier.
/// Adds `r#` prefix if the name is a Rust reserved word.
pub fn rust_ident(name: &str) -> String {
    if RUST_RESERVED_WORDS.contains(&name) {
        format!("r#{}", name)
    } else {
        name.to_string()
    }
}

/// Pretty CLI output helpers using the `colored` crate.
pub mod ui {
    use colored::Colorize;

    /// Print a "create" action (green)
    pub fn created(path: &str) {
        println!("  {} {}", "create".green(), path);
    }

    /// Print an "update" action (cyan)
    pub fn updated(path: &str) {
        println!("  {} {}", "update".cyan(), path);
    }

    /// Print a "skip" action (yellow)
    pub fn skipped(path: &str, reason: &str) {
        println!("  {} {} ({})", "skip".yellow(), path, reason);
    }

    /// Print a "remove" action (red)
    pub fn removed(path: &str) {
        println!("  {} {}", "remove".red(), path);
    }

    /// Print an "inject" action (magenta)
    pub fn injected(target: &str, what: &str) {
        println!("  {} {} → {}", "inject".magenta(), what, target);
    }

    /// Print a section header (bold)
    pub fn section(title: &str) {
        println!("\n{}", title.bold());
    }

    /// Print a success message (green bold)
    pub fn success(msg: &str) {
        println!("\n{}", msg.green().bold());
    }

    /// Print a warning (yellow)
    pub fn warn(msg: &str) {
        println!("  {} {}", "warn".yellow(), msg);
    }

    /// Print an error (red)
    pub fn error(msg: &str) {
        eprintln!("  {} {}", "error".red(), msg);
    }

    /// Print a check result (pass)
    pub fn check_pass(msg: &str) {
        println!("  {} {}", "✓".green(), msg);
    }

    /// Print a check result (fail)
    pub fn check_fail(msg: &str) {
        println!("  {} {}", "✗".red(), msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ── pluralize ─────────────────────────────────────────────────────

    #[test]
    fn pluralize_regular_word() {
        assert_eq!(pluralize("post"), "posts");
        assert_eq!(pluralize("user"), "users");
        assert_eq!(pluralize("product"), "products");
    }

    #[test]
    fn pluralize_ending_in_s() {
        assert_eq!(pluralize("bus"), "buses");
        assert_eq!(pluralize("class"), "classes");
    }

    #[test]
    fn pluralize_ending_in_x() {
        assert_eq!(pluralize("box"), "boxes");
        assert_eq!(pluralize("tax"), "taxes");
    }

    #[test]
    fn pluralize_ending_in_ch() {
        assert_eq!(pluralize("match"), "matches");
        assert_eq!(pluralize("church"), "churches");
    }

    #[test]
    fn pluralize_ending_in_sh() {
        assert_eq!(pluralize("dish"), "dishes");
        assert_eq!(pluralize("wish"), "wishes");
    }

    #[test]
    fn pluralize_consonant_y() {
        assert_eq!(pluralize("category"), "categories");
        assert_eq!(pluralize("city"), "cities");
        assert_eq!(pluralize("company"), "companies");
    }

    #[test]
    fn pluralize_vowel_y_preserved() {
        assert_eq!(pluralize("day"), "days");
        assert_eq!(pluralize("key"), "keys");
        assert_eq!(pluralize("boy"), "boys");
        assert_eq!(pluralize("guy"), "guys");
    }

    // ── rust_ident ────────────────────────────────────────────────────

    #[test]
    fn rust_ident_regular_name() {
        assert_eq!(rust_ident("title"), "title");
        assert_eq!(rust_ident("name"), "name");
        assert_eq!(rust_ident("author_id"), "author_id");
    }

    #[test]
    fn rust_ident_reserved_word() {
        assert_eq!(rust_ident("type"), "r#type");
        assert_eq!(rust_ident("match"), "r#match");
        assert_eq!(rust_ident("fn"), "r#fn");
        assert_eq!(rust_ident("struct"), "r#struct");
        assert_eq!(rust_ident("impl"), "r#impl");
        assert_eq!(rust_ident("use"), "r#use");
        assert_eq!(rust_ident("mod"), "r#mod");
        assert_eq!(rust_ident("async"), "r#async");
        assert_eq!(rust_ident("await"), "r#await");
        assert_eq!(rust_ident("yield"), "r#yield");
    }

    #[test]
    fn rust_ident_future_reserved() {
        assert_eq!(rust_ident("abstract"), "r#abstract");
        assert_eq!(rust_ident("try"), "r#try");
        assert_eq!(rust_ident("final"), "r#final");
    }

    // ── write_file ────────────────────────────────────────────────────

    #[test]
    fn write_file_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/c/test.txt");
        write_file(&path, "hello").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    // ── insert_at_marker ──────────────────────────────────────────────

    #[test]
    fn insert_at_marker_basic() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "// header").unwrap();
        writeln!(tmp, "// === ROMANCE:MODS ===").unwrap();
        writeln!(tmp, "// footer").unwrap();
        tmp.flush().unwrap();

        insert_at_marker(tmp.path(), "// === ROMANCE:MODS ===", "pub mod post;").unwrap();

        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("pub mod post;\n// === ROMANCE:MODS ==="));
    }

    #[test]
    fn insert_at_marker_idempotent() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "// header").unwrap();
        writeln!(tmp, "// === ROMANCE:MODS ===").unwrap();
        tmp.flush().unwrap();

        insert_at_marker(tmp.path(), "// === ROMANCE:MODS ===", "pub mod post;").unwrap();
        insert_at_marker(tmp.path(), "// === ROMANCE:MODS ===", "pub mod post;").unwrap();

        let content = std::fs::read_to_string(tmp.path()).unwrap();
        // Should appear exactly once
        assert_eq!(content.matches("pub mod post;").count(), 1);
    }

    #[test]
    fn insert_at_marker_multiple_lines() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "// === ROMANCE:MODS ===").unwrap();
        tmp.flush().unwrap();

        insert_at_marker(tmp.path(), "// === ROMANCE:MODS ===", "pub mod post;").unwrap();
        insert_at_marker(tmp.path(), "// === ROMANCE:MODS ===", "pub mod user;").unwrap();

        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.contains("pub mod post;"));
        assert!(content.contains("pub mod user;"));
        // Both should be before the marker
        let marker_pos = content.find("// === ROMANCE:MODS ===").unwrap();
        let post_pos = content.find("pub mod post;").unwrap();
        let user_pos = content.find("pub mod user;").unwrap();
        assert!(post_pos < marker_pos);
        assert!(user_pos < marker_pos);
    }

    #[test]
    fn insert_at_marker_missing_marker_errors() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "// header").unwrap();
        writeln!(tmp, "// no marker here").unwrap();
        tmp.flush().unwrap();

        let result = insert_at_marker(tmp.path(), "// === ROMANCE:MODS ===", "pub mod post;");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Marker"));
        assert!(err_msg.contains("ROMANCE:MODS"));
    }

    // ── read_with_custom_block ────────────────────────────────────────

    #[test]
    fn read_with_custom_block_splits_correctly() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "generated code\n// === ROMANCE:CUSTOM ===\nuser code\n").unwrap();
        tmp.flush().unwrap();

        let (generated, custom) = read_with_custom_block(tmp.path()).unwrap();
        assert_eq!(generated, "generated code\n");
        assert!(custom.starts_with("// === ROMANCE:CUSTOM ==="));
        assert!(custom.contains("user code"));
    }

    #[test]
    fn read_with_custom_block_no_marker() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "just some code without marker\n").unwrap();
        tmp.flush().unwrap();

        assert!(read_with_custom_block(tmp.path()).is_none());
    }

    #[test]
    fn read_with_custom_block_nonexistent_file() {
        let path = Path::new("/tmp/romance_test_nonexistent_file_12345.rs");
        assert!(read_with_custom_block(path).is_none());
    }

    // ── write_generated ───────────────────────────────────────────────

    #[test]
    fn write_generated_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new.rs");

        write_generated(&path, "generated content\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "generated content\n");
    }

    #[test]
    fn write_generated_preserves_custom_block() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "old generated\n// === ROMANCE:CUSTOM ===\nmy custom code\n").unwrap();
        tmp.flush().unwrap();

        write_generated(tmp.path(), "new generated\n").unwrap();

        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(content.starts_with("new generated\n"));
        assert!(content.contains("// === ROMANCE:CUSTOM ==="));
        assert!(content.contains("my custom code"));
        // Old generated content should be gone
        assert!(!content.contains("old generated"));
    }

    #[test]
    fn write_generated_no_custom_block_replaces_entirely() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "old content without custom marker\n").unwrap();
        tmp.flush().unwrap();

        write_generated(tmp.path(), "new content\n").unwrap();

        let content = std::fs::read_to_string(tmp.path()).unwrap();
        assert_eq!(content, "new content\n");
    }
}
