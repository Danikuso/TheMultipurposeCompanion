//! Localización y lectura de archivos locales del juego.

pub mod companion_reader;
pub mod files;
pub mod hud;
pub mod locate;
pub mod process;
pub mod status_poller;
pub mod tail;
pub mod watcher;

pub use companion_reader::read_companion;
pub use files::{is_journal_name, list_journals};
pub use hud::read_hud_theme;
pub use locate::{LocateError, default_journal_dir, graphics_override_path, resolve_journal_dir};
pub use process::is_game_running;
pub use status_poller::StatusPoller;
pub use tail::LineTailer;
pub use watcher::{WatchOptions, spawn_watcher};
