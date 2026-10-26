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
    // The modules that export the widgets (the editor module: its code view and editor).
    let source: String = [
        "ui/mod.rs",
        "ui/tabs/mod.rs",
        "ui/popup/mod.rs",
        "openapi/ui/mod.rs",
        "ide/mod.rs",
        "action_system/mod.rs",
        "wm/mod.rs",
        "notification/mod.rs",
    ]
    .iter()
    .map(|module| fs::read_to_string(Path::new("src").join(module)).expect("module source"))
    .chain([fs::read_to_string("src/editor/mod.rs")
        .expect("editor module")
        .lines()
        .filter(|line| line.starts_with("pub use code_"))
        .collect::<Vec<_>>()
        .join("\n")])
    .collect::<Vec<_>>()
    .join("\n");
    let components: BTreeSet<&str> = source
        .lines()
        .filter_map(|line| line.strip_prefix("pub use "))
        .filter_map(|line| line.split_once('{').map(|(_, names)| names))
        .flat_map(|names| names.trim_end_matches("};").split(','))
        .map(str::trim)
        .filter(|name| name.chars().next().is_some_and(char::is_uppercase))
        .collect();
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
