//! C12 / PD-12: ningún test de `tmc-sync`, `tmc-net` o `tmc-app` construye `ReqwestTransport`
//! (salvo el test explícito de construcción en `transport.rs`) ni incrusta literales de
//! endpoints reales de envío de Inara, EDSM o EDDN en regiones de test.

use std::fs;
use std::path::{Path, PathBuf};

const UPLOAD_ENDPOINTS: &[&str] = &[
    "https://inara.cz/inapi",
    "https://www.edsm.net/api-logs-v1",
    "edsm.net/api-journal-v1",
    "edsm.net/api-commander-v1",
    "eddn.edcd.io:4430",
];

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn is_whole_test_file(rel: &str) -> bool {
    let norm = rel.replace('\\', "/");
    norm.contains("/src/tests/") || norm.ends_with("/e2e_tests.rs") || norm.ends_with("_tests.rs")
}

/// Extrae `(num_linea, linea)` que pertenecen a código de test en el archivo.
fn test_lines(rel: &str, content: &str) -> Vec<(usize, String)> {
    if is_whole_test_file(rel) {
        return content
            .lines()
            .enumerate()
            .map(|(i, l)| (i + 1, l.to_string()))
            .collect();
    }
    let lines: Vec<&str> = content.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with("#[cfg(test)]") || trimmed.starts_with("#[cfg(all(test") {
            let mut j = i;
            let mut depth: i32 = 0;
            let mut saw_brace = false;
            while j < lines.len() {
                let line = lines[j];
                out.push((j + 1, line.to_string()));
                for ch in line.chars() {
                    if ch == '{' {
                        depth += 1;
                        saw_brace = true;
                    } else if ch == '}' {
                        depth -= 1;
                    }
                }
                j += 1;
                if saw_brace && depth <= 0 {
                    break;
                }
                if !saw_brace && line.trim_end().ends_with(';') && j > i + 1 {
                    break;
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

fn is_allowed_transport_builder_test(rel: &str, all_lines: &[&str], line_no: usize) -> bool {
    let norm = rel.replace('\\', "/");
    if !norm.ends_with("crates/tmc-net/src/transport.rs") {
        return false;
    }
    let line = all_lines
        .get(line_no.saturating_sub(1))
        .copied()
        .unwrap_or("");
    let prev = line_no
        .checked_sub(2)
        .and_then(|idx| all_lines.get(idx).copied())
        .unwrap_or("");
    line.contains("ReqwestTransport::new().is_ok()")
        && prev.contains("reqwest_transport_builds_without_network")
}

#[test]
fn no_real_network_in_tests() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");

    let crates = ["tmc-sync", "tmc-net", "tmc-app"];
    let mut violations = Vec::new();

    for crate_name in crates {
        let src_dir = workspace_root.join("crates").join(crate_name).join("src");
        let mut files = Vec::new();
        collect_rs_files(&src_dir, &mut files);
        files.sort();

        for file in files {
            let rel = file
                .strip_prefix(workspace_root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");
            // Este propio test contiene las cadenas buscadas como constantes de comprobación.
            if rel.ends_with("crates/tmc-sync/src/tests/no_real_network.rs") {
                continue;
            }
            let content = fs::read_to_string(&file).expect("read rs file");
            let all_lines: Vec<&str> = content.lines().collect();
            for (line_no, line) in test_lines(&rel, &content) {
                if line.contains("ReqwestTransport")
                    && !is_allowed_transport_builder_test(&rel, &all_lines, line_no)
                {
                    violations.push(format!("{rel}:{line_no}: usa ReqwestTransport en test"));
                }
                for endpoint in UPLOAD_ENDPOINTS {
                    if line.contains(endpoint) {
                        violations.push(format!(
                            "{rel}:{line_no}: literal de endpoint real `{endpoint}` en test"
                        ));
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Se encontraron violaciones de PD-12 / C12:\n{}",
        violations.join("\n")
    );
}
