//! Keeps `docs/` honest: links and images resolve, every snapshot is shown on a page, and
//! every component has documentation. (The code blocks are compiled as doctests.)

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const DOCS: &str = "../../docs";

fn markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir)
        .expect("docs directory is readable")
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            markdown_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
}

fn pages() -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    markdown_files(Path::new(DOCS), &mut files);
    files.push(PathBuf::from("../../README.md"));
    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).expect("page is readable");
            (path, text)
        })
        .collect()
}

/// Relative targets of `](...)`, `src="..."` and `srcset="..."`.
fn targets(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (open, close) in [("](", ')'), ("src=\"", '"'), ("srcset=\"", '"')] {
        let mut rest = text;
        while let Some(start) = rest.find(open) {
            rest = &rest[start + open.len()..];
            if let Some(end) = rest.find(close) {
                let target = rest[..end].split('#').next().unwrap_or_default().trim();
                if !target.is_empty() && !target.contains("://") && !target.starts_with("mailto:") {
                    found.push(target.to_owned());
                }
            }
        }
    }
    found
}

#[test]
fn links_and_images_resolve() {
    let mut broken = Vec::new();
    for (path, text) in pages() {
        let dir = path.parent().expect("page has a directory");
        for target in targets(&text) {
            if !dir.join(&target).exists() {
                broken.push(format!("{}: {target}", path.display()));
            }
        }
    }
    assert!(broken.is_empty(), "broken links:\n{}", broken.join("\n"));
}

#[test]
fn every_snapshot_is_shown_in_the_docs() {
    let all_text: String = pages().into_iter().map(|(_, text)| text).collect();
    let mut unused = Vec::new();
    for entry in fs::read_dir(Path::new(DOCS).join("components/img"))
        .expect("img dir")
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().into_owned();
        let stale = [".old.png", ".new.png", ".diff.png"]
            .iter()
            .any(|s| name.ends_with(s));
        assert!(!stale, "leftover snapshot artifact {name}: delete it");
        if name.ends_with(".png") && !all_text.contains(&format!("img/{name}")) {
            unused.push(name);
        }
    }
    assert!(unused.is_empty(), "snapshots no page shows: {unused:?}");
}

#[test]
fn every_component_is_documented() {
    // The modules that export the widgets, with the start of the `pub use` statements that
    // count (the editor module: its code view and editor; the action system: its widgets, not
    // the action model).
    let modules = [
        ("ui/mod.rs", "pub use "),
        ("ui/tabs/mod.rs", "pub use "),
        ("ui/popup/mod.rs", "pub use "),
        ("openapi/ui/mod.rs", "pub use "),
        ("ide/mod.rs", "pub use "),
        ("wm/mod.rs", "pub use "),
        ("notification/mod.rs", "pub use "),
        ("editor/mod.rs", "pub use code_"),
        ("action_system/mod.rs", "pub use action_"),
    ];
    let mut components = BTreeSet::new();
    for (module, prefix) in modules {
        let text = fs::read_to_string(Path::new("src").join(module)).expect("module source");
        // Whole statements: `pub use a::B;` and `pub use a::{B, C};`, also wrapped over lines.
        let mut statement = String::new();
        for line in text.lines() {
            if !line.starts_with(prefix) && statement.is_empty() {
                continue;
            }
            statement.push_str(line);
            if !line.trim_end().ends_with(';') {
                continue;
            }
            let list = statement
                .trim_end()
                .trim_end_matches(';')
                .trim_end_matches('}');
            let names = list.split_once('{').map_or_else(
                || list.rsplit("::").next().unwrap_or(""),
                |(_, names)| names,
            );
            components.extend(
                names
                    .split(',')
                    .map(|name| name.trim().to_owned())
                    .filter(|name| name.chars().next().is_some_and(char::is_uppercase)),
            );
            statement.clear();
        }
    }
    let docs: String = fs::read_dir(Path::new(DOCS).join("components"))
        .expect("components docs")
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
        .map(|entry| fs::read_to_string(entry.path()).expect("page"))
        .collect();
    let missing: Vec<_> = components
        .iter()
        .filter(|name| {
            !docs.contains(&format!("`{name}"))
                && !docs.contains(&format!("# {name}"))
                && !docs.contains(&format!(", {name}"))
        })
        .collect();
    assert!(missing.is_empty(), "undocumented components: {missing:?}");
}
