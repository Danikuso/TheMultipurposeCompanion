//! Localización de los diarios y de la configuración gráfica del juego.

use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocateError {
    NotFound,
    NotADirectory(PathBuf),
}

impl fmt::Display for LocateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "journal directory not found"),
            Self::NotADirectory(path) => write!(f, "not a directory: {}", path.display()),
        }
    }
}

impl std::error::Error for LocateError {}

pub fn default_journal_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Shell::FOLDERID_SavedGames;
        let dir = known_folder(&FOLDERID_SavedGames)?
            .join("Frontier Developments")
            .join("Elite Dangerous");
        dir.is_dir().then_some(dir)
    }
    #[cfg(target_os = "linux")]
    {
        linux_journal_dir(&PathBuf::from(std::env::var_os("HOME")?))
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        None
    }
}

pub fn resolve_journal_dir(configured: Option<&Path>) -> Result<PathBuf, LocateError> {
    if let Some(path) = configured {
        return if path.is_dir() {
            Ok(path.to_path_buf())
        } else {
            Err(LocateError::NotADirectory(path.to_path_buf()))
        };
    }
    default_journal_dir().ok_or(LocateError::NotFound)
}

pub fn graphics_override_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        Some(PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join(
            "Frontier Developments/Elite Dangerous/Options/Graphics/GraphicsConfigurationOverride.xml",
        ))
    }
    #[cfg(target_os = "linux")]
    {
        linux_graphics_override_path(&PathBuf::from(std::env::var_os("HOME")?))
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        None
    }
}

#[cfg(windows)]
fn known_folder(id: &windows::core::GUID) -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{KF_FLAG_DEFAULT, SHGetKnownFolderPath};

    // SAFETY: el GUID vive durante la llamada y no se suplanta ningún usuario.
    let allocated = unsafe { SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None) }.ok()?;
    // SAFETY: la API devuelve una cadena UTF-16 terminada en NUL; copiamos antes de liberarla.
    let path = PathBuf::from(OsString::from_wide(unsafe { allocated.as_wide() }));
    // SAFETY: SHGetKnownFolderPath asigna con el allocator COM; liberamos una sola vez.
    unsafe { CoTaskMemFree(Some(allocated.0.cast())) };
    Some(path)
}

#[cfg(any(target_os = "linux", test))]
fn proton_prefixes(home: &Path) -> [PathBuf; 3] {
    [
        ".steam/steam/steamapps/compatdata/359320/pfx",
        ".local/share/Steam/steamapps/compatdata/359320/pfx",
        ".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata/359320/pfx",
    ]
    .map(|path| home.join(path))
}

#[cfg(any(target_os = "linux", test))]
fn linux_journal_dir(home: &Path) -> Option<PathBuf> {
    proton_prefixes(home)
        .into_iter()
        .map(|p| {
            p.join("drive_c/users/steamuser/Saved Games/Frontier Developments/Elite Dangerous")
        })
        .find(|p| p.is_dir())
}

#[cfg(any(target_os = "linux", test))]
fn linux_graphics_override_path(home: &Path) -> Option<PathBuf> {
    proton_prefixes(home).into_iter().find(|p| p.is_dir()).map(|p| p.join(
        "drive_c/users/steamuser/AppData/Local/Frontier Developments/Elite Dangerous/Options/Graphics/GraphicsConfigurationOverride.xml",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn configured_dir_wins_and_must_exist() {
        let dir = tempdir().unwrap();
        let configured = dir.path().join("Partidas guardadas ñ é");
        fs::create_dir(&configured).unwrap();
        assert_eq!(resolve_journal_dir(Some(&configured)), Ok(configured));
        let missing = dir.path().join("missing");
        assert_eq!(
            resolve_journal_dir(Some(&missing)),
            Err(LocateError::NotADirectory(missing))
        );
        let file = dir.path().join("file");
        fs::write(&file, "").unwrap();
        assert_eq!(
            resolve_journal_dir(Some(&file)),
            Err(LocateError::NotADirectory(file))
        );
    }

    #[test]
    fn proton_candidates_match_steam_and_flatpak_locations() {
        let dir = tempdir().unwrap();
        let expected = [
            ".steam/steam/steamapps/compatdata/359320/pfx",
            ".local/share/Steam/steamapps/compatdata/359320/pfx",
            ".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata/359320/pfx",
        ]
        .map(|p| dir.path().join(p));
        assert_eq!(proton_prefixes(dir.path()), expected);
    }

    #[test]
    fn linux_finds_first_existing_journal_directory() {
        let home = tempdir().unwrap();
        assert_eq!(linux_journal_dir(home.path()), None);
        let dirs = proton_prefixes(home.path()).map(|p| {
            p.join("drive_c/users/steamuser/Saved Games/Frontier Developments/Elite Dangerous")
        });
        // Un archivo en un candidato no debe impedir encontrar el siguiente.
        fs::create_dir_all(dirs[0].parent().unwrap()).unwrap();
        fs::write(&dirs[0], "").unwrap();
        fs::create_dir_all(&dirs[2]).unwrap();
        assert_eq!(linux_journal_dir(home.path()), Some(dirs[2].clone()));
        fs::create_dir_all(&dirs[1]).unwrap();
        assert_eq!(linux_journal_dir(home.path()), Some(dirs[1].clone()));
    }

    #[test]
    fn linux_graphics_path_uses_first_existing_proton_prefix() {
        let home = tempdir().unwrap();
        assert_eq!(linux_graphics_override_path(home.path()), None);
        let prefixes = proton_prefixes(home.path());
        fs::create_dir_all(&prefixes[2]).unwrap();
        let suffix = "drive_c/users/steamuser/AppData/Local/Frontier Developments/Elite Dangerous/Options/Graphics/GraphicsConfigurationOverride.xml";
        assert_eq!(
            linux_graphics_override_path(home.path()),
            Some(prefixes[2].join(suffix))
        );
        fs::create_dir_all(&prefixes[0]).unwrap();
        assert_eq!(
            linux_graphics_override_path(home.path()),
            Some(prefixes[0].join(suffix))
        );
    }
}
