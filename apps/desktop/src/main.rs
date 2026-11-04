//! Aplicación de escritorio Tauri 2 para TMC.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tray;
mod window;

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;
use tauri::webview::Url;
use tauri_plugin_window_state::StateFlags;
use tmc_app::{Config, data_dir, init_logging, start};

use crate::tray::{RunningState, setup_tray};
use crate::window::{create_or_focus_main_window, open_or_focus_async};

fn main() -> anyhow::Result<()> {
    let mut config_file: Option<PathBuf> = None;
    let mut journal_dir: Option<PathBuf> = None;
    let mut ui_dir: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => config_file = args.next().map(PathBuf::from),
            "--journal-dir" => journal_dir = args.next().map(PathBuf::from),
            "--ui-dir" => ui_dir = args.next().map(PathBuf::from),
            _ => {}
        }
    }

    let mut config = match &config_file {
        Some(path) => Config::load_from_or_default(path),
        None => Config::load_or_default(),
    };
    // Los guardados automáticos van al archivo indicado con --config.
    config.config_file = config_file;
    if journal_dir.is_some() {
        config.journal_dir = journal_dir;
    }
    if ui_dir.is_some() {
        config.ui_dir = ui_dir;
    }

    let language = config.language.clone();
    let log_dir = config.data_dir.clone().unwrap_or_else(data_dir);
    let _log_guard = init_logging(&log_dir);
    _log_guard.application_started();

    let running = match tauri::async_runtime::block_on(start(config)) {
        Ok(running) => running,
        Err(err) => {
            tracing::error!("error al iniciar TMC: {err:?}");
            #[cfg(windows)]
            show_startup_error(&err);
            return Err(err);
        }
    };
    let local_url: Url = running.server.local_url.parse()?;
    let single_instance_url = local_url.clone();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(
            move |app, _argv, _cwd| {
                open_or_focus_async(app.clone(), single_instance_url.clone());
            },
        ))
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED)
                .build(),
        )
        .manage(RunningState(Mutex::new(Some(running))))
        .setup(move |app| {
            setup_tray(app.handle(), &language, local_url.clone())?;
            create_or_focus_main_window(app.handle(), &local_url)?;
            Ok(())
        })
        .build(tauri::generate_context!())?;

    app.run(|app_handle, event| match event {
        tauri::RunEvent::ExitRequested {
            code: None, api, ..
        } => {
            api.prevent_exit();
        }
        tauri::RunEvent::Exit => {
            if let Some(state) = app_handle.try_state::<RunningState>()
                && let Ok(mut guard) = state.0.lock()
                && let Some(running) = guard.take()
            {
                running.server.shutdown();
            }
        }
        _ => {}
    });

    Ok(())
}

#[cfg(windows)]
fn show_startup_error(err: &anyhow::Error) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let message = format!(
        "No se pudo iniciar TMC — The Multipurpose Companion:\n\n{err}\n\n\
        Si juegas en otro equipo y accedes a los registros por red compartida,\n\
        comprueba que la carpeta esté accesible y que 'journal_dir' en config.toml sea correcto."
    );

    let wide_title: Vec<u16> = OsStr::new("TMC — Error al iniciar")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let wide_msg: Vec<u16> = OsStr::new(&message)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(hwnd: isize, lpText: *const u16, lpCaption: *const u16, uType: u32) -> i32;
    }
    const MB_OK: u32 = 0x00000000;
    const MB_ICONERROR: u32 = 0x00000010;
    unsafe {
        MessageBoxW(
            0,
            wide_msg.as_ptr(),
            wide_title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
