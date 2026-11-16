use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::webview::Url;
use tauri::{AppHandle, Manager};
use tmc_app::{AppActions, Running};
use tmc_i18n::I18n;
use tmc_server::ActionHost;

use crate::window::{DraggedPos, MAIN_WINDOW_TITLE, OVERLAY_WINDOW_LABEL, open_or_focus_async};

pub struct RunningState(pub Mutex<Option<Running>>);

fn active_actions(app: &AppHandle) -> Option<Arc<AppActions>> {
    let state = app.try_state::<RunningState>()?;
    let guard = state.0.lock().ok()?;
    guard.as_ref().map(|running| running.actions.clone())
}

pub fn setup_tray(
    app: &AppHandle,
    language: &str,
    local_url: Url,
    dragged_pos: DraggedPos,
) -> tauri::Result<()> {
    let i18n = I18n::new(language);
    let open_label = i18n.t("tray-open", &[]);
    let overlay_toggle_label = i18n.t("tray-overlay-toggle", &[]);
    let overlay_lock_label = i18n.t("tray-overlay-lock", &[]);
    let quit_label = i18n.t("tray-quit", &[]);

    let open_item = MenuItemBuilder::with_id("tray-open", open_label).build(app)?;
    let overlay_toggle_item =
        MenuItemBuilder::with_id("tray-overlay-toggle", overlay_toggle_label).build(app)?;
    let overlay_lock_item =
        MenuItemBuilder::with_id("tray-overlay-lock", overlay_lock_label).build(app)?;
    let quit_item = MenuItemBuilder::with_id("tray-quit", quit_label).build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[
            &open_item,
            &overlay_toggle_item,
            &overlay_lock_item,
            &quit_item,
        ])
        .build()?;

    let url_for_menu = local_url.clone();
    let url_for_click = local_url;
    let dragged_for_tray = dragged_pos;

    let mut builder = TrayIconBuilder::new()
        .tooltip(MAIN_WINDOW_TITLE)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "tray-open" => {
                open_or_focus_async(app.clone(), url_for_menu.clone());
            }
            "tray-overlay-toggle" => {
                if let Some(actions) = active_actions(app) {
                    let _ = actions.run_action_with(
                        "overlay.toggle",
                        &serde_json::Value::Null,
                        &BTreeMap::new(),
                    );
                }
            }
            "tray-overlay-lock" => {
                if let Some(actions) = active_actions(app) {
                    let currently_locked = actions.subscribe_overlay().borrow().click_through;
                    let args = if !currently_locked
                        && let Some((x, y)) = dragged_for_tray.lock().ok().and_then(|d| *d)
                    {
                        serde_json::json!({
                            "click_through": true,
                            "anchor": "custom",
                            "custom_x": x,
                            "custom_y": y,
                        })
                    } else {
                        serde_json::json!({ "click_through": !currently_locked })
                    };
                    let _ = actions.run_action_with(
                        "overlay.set_click_through",
                        &args,
                        &BTreeMap::new(),
                    );
                }
            }
            "tray-quit" => {
                if let Some(overlay) = app.get_webview_window(OVERLAY_WINDOW_LABEL) {
                    let _ = overlay.close();
                }
                if let Some(state) = app.try_state::<RunningState>()
                    && let Ok(mut guard) = state.0.lock()
                    && let Some(running) = guard.take()
                {
                    tauri::async_runtime::block_on(running.shutdown());
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
