//! Carga y resolución de variables de entorno con fallback a archivo `.env`.

use std::path::Path;

/// Resuelve una variable buscando primero en `std::env::var`, y si no está definida o está vacía,
/// buscando una definición `CLAVE=VALOR` en archivos `.env` locales.
pub fn env_var_or_file(key: &str) -> Option<String> {
    if let Ok(val) = std::env::var(key) {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    find_and_read_env_key(key)
}

/// Busca en directorios candidatos (cwd, padres de cwd, exe dir, padres de exe dir)
/// el archivo `.env` y extrae la clave solicitada.
fn find_and_read_env_key(key: &str) -> Option<String> {
    let mut visited = Vec::new();

    // 1. Directorio actual de trabajo y sus padres inmediatos
    if let Ok(cwd) = std::env::current_dir() {
        let mut curr = Some(cwd.as_path());
        for _ in 0..4 {
            if let Some(dir) = curr {
                let candidate = dir.join(".env");
                if !visited.contains(&candidate) {
                    if let Some(val) = read_key_from_env_file(&candidate, key) {
                        return Some(val);
                    }
                    visited.push(candidate);
                }
                curr = dir.parent();
            } else {
                break;
            }
        }
    }

    // 2. Directorio del ejecutable y sus padres inmediatos (en caso de binario compilado o portable)
    if let Ok(exe) = std::env::current_exe() {
        let mut curr = exe.parent();
        for _ in 0..4 {
            if let Some(dir) = curr {
                let candidate = dir.join(".env");
                if !visited.contains(&candidate) {
                    if let Some(val) = read_key_from_env_file(&candidate, key) {
                        return Some(val);
                    }
                    visited.push(candidate);
                }
                curr = dir.parent();
            } else {
                break;
            }
        }
    }

    None
}

/// Lee un archivo `.env` y busca una línea con `key=val`.
pub(crate) fn read_key_from_env_file(path: &Path, key: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    parse_key_from_env_str(&content, key)
}

/// Parsea el contenido de texto de un `.env` buscando `key=val`.
pub(crate) fn parse_key_from_env_str(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once('=')
            && k.trim() == key
        {
            let val = v.trim();
            let unquoted = if (val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\''))
            {
                if val.len() >= 2 {
                    &val[1..val.len() - 1]
                } else {
                    val
                }
            } else {
                val
            };
            let trimmed = unquoted.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn parses_plain_and_quoted_values() {
        let sample = r#"
# Comentario
TMC_CAPI_CLIENT_ID=client_123
INARA_DEV_API_KEY="inara_dev_secret_456"
OTHER_KEY='single_quoted'
SPACED_KEY =  spaced_val  
EMPTY_KEY=
"#;
        assert_eq!(
            parse_key_from_env_str(sample, "TMC_CAPI_CLIENT_ID"),
            Some("client_123".into())
        );
        assert_eq!(
            parse_key_from_env_str(sample, "INARA_DEV_API_KEY"),
            Some("inara_dev_secret_456".into())
        );
        assert_eq!(
            parse_key_from_env_str(sample, "OTHER_KEY"),
            Some("single_quoted".into())
        );
        assert_eq!(
            parse_key_from_env_str(sample, "SPACED_KEY"),
            Some("spaced_val".into())
        );
        assert_eq!(parse_key_from_env_str(sample, "EMPTY_KEY"), None);
        assert_eq!(parse_key_from_env_str(sample, "NON_EXISTENT"), None);
    }

    #[test]
    fn reads_from_actual_temp_file() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "INARA_DEV_API_KEY=test_key_from_file").unwrap();
        tmp.flush().unwrap();

        let val = read_key_from_env_file(tmp.path(), "INARA_DEV_API_KEY");
        assert_eq!(val, Some("test_key_from_file".into()));
    }
}
