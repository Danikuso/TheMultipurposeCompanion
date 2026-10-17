use std::sync::Mutex;
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::webview::Url;
use tauri::{AppHandle, Manager};
use tmc_app::Running;
use tmc_i18n::I18n;

use crate::window::{MAIN_WINDOW_TITLE, open_or_focus_async};

pub struct RunningState(pub Mutex<Option<Running>>);

pub fn setup_tray(app: &AppHandle, language: &str, local_url: Url) -> tauri::Result<()> {
    let i18n = I18n::new(language);
    let open_label = i18n.t("tray-open", &[]);
    let quit_label = i18n.t("tray-quit", &[]);

    let open_item = MenuItemBuilder::with_id("tray-open", open_label).build(app)?;
    let quit_item = MenuItemBuilder::with_id("tray-quit", quit_label).build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open_item, &quit_item])
        .build()?;

    let url_for_menu = local_url.clone();
    let url_for_click = local_url;

    let mut builder = TrayIconBuilder::new()
        .tooltip(MAIN_WINDOW_TITLE)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "tray-open" => {
                open_or_focus_async(app.clone(), url_for_menu.clone());
            }
            "tray-quit" => {
                if let Some(state) = app.try_state::<RunningState>()
                    && let Ok(mut guard) = state.0.lock()
                    && let Some(running) = guard.take()
                {
                    running.server.shutdown();
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                open_or_focus_async(tray.app_handle().clone(), url_for_click.clone());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}
