//! The manual (docs/manual) stays complete: every command has its page, every page is in the index
//! and in the MCP server, every environment variable the programs read is documented, every switch
//! that can change a result is in the run log header, and every link resolves.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn read(p: &str) -> String {
    std::fs::read_to_string(Path::new(ROOT).join(p)).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// the strings `prefix"X"` in a text, e.g. all `Some("x")` arms of a match
fn quoted_after(s: &str, prefix: &str) -> Vec<String> {
    s.match_indices(prefix).filter_map(|(i, _)| {
        let r = &s[i + prefix.len()..];
        r.find('"').map(|j| r[..j].to_string())
    }).collect()
}

/// names like HLA_X in a text (quoted in Rust sources, bare in shell scripts)
fn env_names(s: &str) -> BTreeSet<String> {
    let b = s.as_bytes();
    let mut out = BTreeSet::new();
    for (i, _) in s.match_indices("HLA_") {
        if i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_') {
            continue;
        }
        let e = s[i..].find(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')).map_or(s.len(), |e| i + e);
        let v = s[i..e].trim_end_matches('_');
        // "HLA_*" in a comment names the family, not a variable
        if v.len() > 4 {
            out.insert(v.to_string());
        }
    }
    out
}

/// the documentation names a variable as `VAR`, `VAR=...` or `VAR,`
fn documented(doc: &str, v: &str) -> bool {
    ["`", "=", "`,", ","].iter().any(|t| doc.contains(&format!("`{v}{t}")))
}

fn rs_files(dir: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(Path::new(ROOT).join(dir)).unwrap().filter_map(|e| {
        let p = e.unwrap().path();
        (p.extension().map_or(false, |x| x == "rs")).then(|| std::fs::read_to_string(p).unwrap())
    }).collect();
    v.sort();
    v
}

fn manual_pages() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(Path::new(ROOT).join("docs/manual")).unwrap().filter_map(|e| {
        let n = e.unwrap().file_name().to_string_lossy().into_owned();
        n.strip_suffix(".md").map(|x| x.to_string())
    }).collect();
    v.sort();
    v
}

#[test]
fn every_command_has_a_page() {
    let main = read("src/main.rs");
    let index = read("docs/manual/README.md");
    let cmds = quoted_after(&main, "Some(\"");
    assert!(cmds.iter().any(|c| c == "build-db"), "no commands found in src/main.rs");
    for c in &cmds {
        if c.starts_with('-') {
            assert!(index.contains(&format!("`hla-wgs {c}`")) || index.contains(&format!("`{c}`")), "option {c} is not in docs/manual/README.md");
        } else {
            assert!(Path::new(ROOT).join(format!("docs/manual/{c}.md")).is_file(), "command {c} has no page docs/manual/{c}.md");
        }
    }
    // the typing itself has no command word
    assert!(Path::new(ROOT).join("docs/manual/typing.md").is_file());
}

#[test]
fn every_page_in_index_and_mcp() {
    let index = read("docs/manual/README.md");
    let mcp = read("src/mcp.rs");
    for p in manual_pages() {
        if p != "README" {
            assert!(index.contains(&format!("]({p}.md")), "docs/manual/{p}.md is not linked from docs/manual/README.md");
        }
        assert!(mcp.contains(&format!("docs/manual/{p}.md")), "docs/manual/{p}.md is not a resource of the MCP server (src/mcp.rs PAGES)");
    }
}

#[test]
fn every_switch_documented() {
    let doc = read("docs/manual/switches.md") + &read("docs/manual/start-file.md");
    let mut vars = BTreeSet::new();
    for s in rs_files("src").iter().chain([read("build.rs")].iter()) {
        vars.extend(env_names(s));
    }
    for f in std::fs::read_dir(Path::new(ROOT).join("start")).unwrap() {
        vars.extend(env_names(&std::fs::read_to_string(f.unwrap().path()).unwrap()));
    }
    assert!(vars.contains("HLA_TGRP"), "no variables found");
    for v in &vars {
        assert!(documented(&doc, v), "{v} is read by the program but not in docs/manual/switches.md or start-file.md");
    }
    let val = read("validation/README.md");
    for s in rs_files("src/bin/hla-wgs-validate") {
        for v in env_names(&s) {
            assert!(documented(&val, &v) || documented(&doc, &v), "{v} is read by hla-wgs-validate but not in validation/README.md");
        }
    }
}

#[test]
fn result_switches_in_log_header() {
    let doc = read("docs/manual/switches.md");
    let runlog = read("src/runlog.rs");
    let a = doc.find("## Switches that can change the result").expect("section missing");
    let b = a + doc[a..].find("\n## ").expect("next section missing");
    let mut n = 0;
    for line in doc[a..b].lines().filter(|l| l.starts_with("| `")) {
        let first = line.split('|').nth(1).unwrap();
        for v in first.split('`').filter(|x| x.starts_with("HLA_") || *x == "GENES") {
            let v = v.split('=').next().unwrap();
            n += 1;
            assert!(runlog.contains(&format!("(\"{v}\",")), "{v} can change the result but is not in KNOBS (src/runlog.rs), so the log header does not show its default");
        }
    }
    assert!(n > 20, "too few switches found");
}

#[test]
fn every_validate_command_documented() {
    let main = read("src/bin/hla-wgs-validate/main.rs");
    let val = read("validation/README.md");
    for c in quoted_after(&main, "Some(\"") {
        assert!(val.contains(&format!("`{c} ")) || val.contains(&format!("`{c}`")), "hla-wgs-validate {c} is not in validation/README.md");
    }
}

/// GitHub's anchor of a heading
fn slug(h: &str) -> String {
    h.trim().to_lowercase().chars().filter_map(|c| match c {
        ' ' => Some('-'),
        c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c),
        _ => None,
    }).collect()
}

fn anchors(text: &str) -> BTreeSet<String> {
    let mut fence = false;
    text.lines().filter_map(|l| {
        if l.starts_with("```") {
            fence = !fence;
        }
        (!fence && l.starts_with('#')).then(|| slug(l.trim_start_matches('#')))
    }).collect()
}

/// the targets of all [text](target) links outside code
fn links(text: &str) -> Vec<String> {
    let mut out = vec![];
    let mut fence = false;
    for l in text.lines() {
        if l.starts_with("```") {
            fence = !fence;
        }
        if fence || l.starts_with("    ") {
            continue;
        }
        for (i, _) in l.match_indices("](") {
            let r = &l[i + 2..];
            if let Some(j) = r.find(')') {
                out.push(r[..j].to_string());
            }
        }
    }
    out
}

#[test]
fn every_link_resolves() {
    let mut files: Vec<PathBuf> = vec!["README.md".into(), "validation/README.md".into(), "CHANGELOG.md".into()];
    for d in ["docs", "docs/manual"] {
        for e in std::fs::read_dir(Path::new(ROOT).join(d)).unwrap() {
            let p = e.unwrap().path();
            if p.extension().map_or(false, |x| x == "md") {
                files.push(p.strip_prefix(ROOT).unwrap().to_path_buf());
            }
        }
    }
    let mut checked = 0;
    for f in &files {
        let text = read(f.to_str().unwrap());
        let dir = Path::new(ROOT).join(f).parent().unwrap().to_path_buf();
        for t in links(&text) {
            if t.contains("://") || t.starts_with("mailto:") {
                continue;
            }
            let (path, anchor) = match t.split_once('#') {
                Some((p, a)) => (p.to_string(), Some(a.to_string())),
                None => (t.clone(), None),
            };
            let target = if path.is_empty() { Path::new(ROOT).join(f) } else { dir.join(&path) };
            assert!(target.exists(), "{}: link {t} leads nowhere", f.display());
            if let Some(a) = anchor {
                if target.extension().map_or(false, |x| x == "md") {
                    let have = anchors(&std::fs::read_to_string(&target).unwrap());
                    assert!(have.contains(&a), "{}: link {t}: no heading #{a} in {}", f.display(), target.display());
                }
            }
            checked += 1;
        }
    }
    assert!(checked > 30, "only {checked} links found");
}
