use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
#[cfg(windows)]
use tauri::WebviewWindow;
use tauri::webview::Url;
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tmc_model::settings::{OverlayAnchor, OverlayMode, OverlaySettings};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const MAIN_WINDOW_TITLE: &str = "TMC — The Multipurpose Companion";
pub const OVERLAY_WINDOW_LABEL: &str = "overlay";
pub const OVERLAY_WINDOW_TITLE: &str = "TMC Overlay";
pub const OVERLAY_MARGIN_PX: i32 = 16;

const ADDITIONAL_BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,Translate,SpareRendererForSitePerProcess,AudioServiceOutOfProcess,StorageServiceOutOfProcess --enable-features=NetworkServiceInProcess2 --renderer-process-limit=1 --in-process-gpu --disable-gpu --disable-gpu-compositing --disable-gpu-shader-disk-cache --num-raster-threads=1 --js-flags=--max-old-space-size=32";

pub type DraggedPos = Arc<Mutex<Option<(i32, i32)>>>;

struct OverlayMoveGuard {
    unlocked: AtomicBool,
    programmatic: AtomicBool,
}

impl Default for OverlayMoveGuard {
    fn default() -> Self {
        Self {
            unlocked: AtomicBool::new(false),
            programmatic: AtomicBool::new(false),
        }
    }
}

pub fn create_or_focus_main_window(app: &AppHandle, local_url: &Url) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        #[cfg(windows)]
        set_webview_low_memory(&window, false);
        return Ok(());
    }

    let mut window_url = local_url.clone();
    window_url.set_query(Some("shell=desktop"));
    let window =
        WebviewWindowBuilder::new(app, MAIN_WINDOW_LABEL, WebviewUrl::External(window_url))
            .title(MAIN_WINDOW_TITLE)
            .inner_size(1600.0, 900.0)
            .additional_browser_args(ADDITIONAL_BROWSER_ARGS)
            .build()?;

    #[cfg(windows)]
    set_webview_low_memory(&window, false);

    let win_clone = window.clone();
    let was_minimized = Arc::new(AtomicBool::new(false));
    window.on_window_event(move |event| {
        if let WindowEvent::Resized(_) = event {
            let is_min = win_clone.is_minimized().unwrap_or(false);
            let prev = was_minimized.swap(is_min, Ordering::SeqCst);
            if is_min != prev {
                #[cfg(windows)]
                set_webview_low_memory(&win_clone, is_min);
            }
        }
    });

    Ok(())
}

/// Dimensiones lógicas del overlay según modo (`Minimal`/`Tactical`) y estado de bloqueo.
/// Cuando está desbloqueado (`click_through == false`), añade 40 px de alto para la barra de agarre.
pub fn overlay_dimensions(mode: OverlayMode, click_through: bool) -> (f64, f64) {
    let extra_drag_bar = if click_through { 0.0 } else { 40.0 };
    match mode {
        OverlayMode::Minimal => (436.0, 48.0 + extra_drag_bar),
        OverlayMode::Tactical => (416.0, 184.0 + extra_drag_bar),
    }
}

/// Calcula las coordenadas físicas `(x, y)` del overlay dentro de la geometría del monitor.
pub fn compute_overlay_position(
    anchor: OverlayAnchor,
    custom_x: Option<i32>,
    custom_y: Option<i32>,
    mon_pos: (i32, i32),
    mon_size: (u32, u32),
    win_size: (u32, u32),
) -> (i32, i32) {
    if anchor == OverlayAnchor::Custom
        && let (Some(x), Some(y)) = (custom_x, custom_y)
    {
        return (x, y);
    }

    let (mx, my) = mon_pos;
    let mw = i32::try_from(mon_size.0).unwrap_or(1920);
    let mh = i32::try_from(mon_size.1).unwrap_or(1080);
    let ww = i32::try_from(win_size.0).unwrap_or(420);
    let wh = i32::try_from(win_size.1).unwrap_or(184);

    let max_x = mx + (mw - ww).max(0);
    let max_y = my + (mh - wh).max(0);
    let clamp = |x: i32, y: i32| (x.clamp(mx, max_x), y.clamp(my, max_y));

    match anchor {
        OverlayAnchor::TopLeft => clamp(mx + OVERLAY_MARGIN_PX, my + OVERLAY_MARGIN_PX),
        OverlayAnchor::TopCenter => clamp(mx + (mw - ww) / 2, my + OVERLAY_MARGIN_PX),
        OverlayAnchor::TopRight | OverlayAnchor::Custom => {
            clamp(mx + mw - ww - OVERLAY_MARGIN_PX, my + OVERLAY_MARGIN_PX)
        }
        OverlayAnchor::BottomLeft => {
            clamp(mx + OVERLAY_MARGIN_PX, my + mh - wh - OVERLAY_MARGIN_PX)
        }
        OverlayAnchor::BottomRight => clamp(
            mx + mw - ww - OVERLAY_MARGIN_PX,
            my + mh - wh - OVERLAY_MARGIN_PX,
        ),
    }
}

/// Sincroniza la existencia, tamaño, posición, opacidad y click-through de la ventana `"overlay"`.
pub fn sync_overlay_window(
    app: &AppHandle,
    local_url: &Url,
    settings: &OverlaySettings,
    dragged_pos: &DraggedPos,
) -> tauri::Result<()> {
    if !settings.enabled {
        if let Some(existing) = app.get_webview_window(OVERLAY_WINDOW_LABEL) {
            let _ = existing.close();
        }
        return Ok(());
    }

    if app.try_state::<Arc<OverlayMoveGuard>>().is_none() {
        app.manage(Arc::new(OverlayMoveGuard::default()));
    }
    let guard = app
        .try_state::<Arc<OverlayMoveGuard>>()
        .map(|s| Arc::clone(&*s));

    let (lw, lh) = overlay_dimensions(settings.mode, settings.click_through);
    let mut created = false;
    let window = if let Some(existing) = app.get_webview_window(OVERLAY_WINDOW_LABEL) {
        existing
    } else {
        created = true;
        let mut overlay_url = local_url.clone();
        overlay_url.set_query(Some("shell=overlay"));
        WebviewWindowBuilder::new(app, OVERLAY_WINDOW_LABEL, WebviewUrl::External(overlay_url))
            .title(OVERLAY_WINDOW_TITLE)
            .inner_size(lw, lh)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .shadow(false)
            .visible(true)
            .additional_browser_args(ADDITIONAL_BROWSER_ARGS)
            .build()?
    };

    if created {
        let win_drag = dragged_pos.clone();
        let move_guard = guard.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::Moved(pos) = event
                && let Some(ref g) = move_guard
                && g.unlocked.load(Ordering::SeqCst)
                && !g.programmatic.load(Ordering::SeqCst)
                && let Ok(mut slot) = win_drag.lock()
            {
                *slot = Some((pos.x, pos.y));
            }
        });
    }

    if let Some(ref g) = guard {
        g.unlocked.store(!settings.click_through, Ordering::SeqCst);
        g.programmatic.store(true, Ordering::SeqCst);
    }

    let _ = window.set_size(tauri::LogicalSize::new(lw, lh));

    let monitor = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .and_then(|w| w.current_monitor().ok().flatten())
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());

    if let Some(mon) = monitor {
        let scale = mon.scale_factor();
        let pw = (lw * scale).round().max(1.0) as u32;
        let ph = (lh * scale).round().max(1.0) as u32;

        // Si el piloto está moviendo libremente (desbloqueado) y ya arrastró manualmente,
        // respetamos su posición aunque cambie la opacidad o modo antes de fijar.
        let preserved_drag = if !settings.click_through && settings.anchor != OverlayAnchor::Custom
        {
            dragged_pos.lock().ok().and_then(|d| *d)
        } else {
            None
        };

        if settings.click_through
            && settings.anchor != OverlayAnchor::Custom
            && let Ok(mut d) = dragged_pos.lock()
        {
            *d = None;
        }

        let (px, py) = if let Some(pos) = preserved_drag {
            pos
        } else {
            compute_overlay_position(
                settings.anchor,
                settings.custom_x,
                settings.custom_y,
                (mon.position().x, mon.position().y),
                (mon.size().width, mon.size().height),
                (pw, ph),
            )
        };
        let _ = window.set_position(PhysicalPosition::new(px, py));
    }

    if let Some(ref g) = guard {
        g.programmatic.store(false, Ordering::SeqCst);
    }

    let _ = window.set_ignore_cursor_events(settings.click_through);
    #[cfg(windows)]
    apply_overlay_win32_styles(&window, settings.opacity, settings.click_through);

    Ok(())
}

/// Inicia el arrastre nativo de ventana sobre el overlay cuando está desbloqueado.
pub fn start_overlay_drag(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(OVERLAY_WINDOW_LABEL) {
        let _ = window.start_dragging();
    }
}

#[cfg(windows)]
fn apply_overlay_win32_styles(window: &WebviewWindow, opacity: u8, click_through: bool) {
    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_TRANSPARENT: isize = 0x0000_0020;
    const WS_EX_TOOLWINDOW: isize = 0x0000_0080;
    const WS_EX_NOACTIVATE: isize = 0x0800_0000;
    const WS_EX_LAYERED: isize = 0x0008_0000;
    const LWA_ALPHA: u32 = 0x0000_0002;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_FRAMECHANGED: u32 = 0x0020;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetWindowLongPtrW(hwnd: isize, n_index: i32) -> isize;
        fn SetWindowLongPtrW(hwnd: isize, n_index: i32, dw_new_long: isize) -> isize;
        fn SetLayeredWindowAttributes(hwnd: isize, cr_key: u32, b_alpha: u8, dw_flags: u32) -> i32;
        fn SetWindowPos(
            hwnd: isize,
            hwnd_insert_after: isize,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            u_flags: u32,
        ) -> i32;
    }

    let Ok(raw_hwnd) = window.hwnd() else {
        return;
    };
    let hwnd = raw_hwnd.0 as isize;
    let alpha_byte = ((u16::from(opacity.clamp(35, 95)) * 255) / 100) as u8;

    unsafe {
        let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        ex_style |= WS_EX_LAYERED | WS_EX_TOOLWINDOW;
        if click_through {
            ex_style |= WS_EX_TRANSPARENT | WS_EX_NOACTIVATE;
        } else {
            ex_style &= !(WS_EX_TRANSPARENT | WS_EX_NOACTIVATE);
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
        SetLayeredWindowAttributes(hwnd, 0, alpha_byte, LWA_ALPHA);
        SetWindowPos(
            hwnd,
            0,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

pub fn open_or_focus_async(app: AppHandle, local_url: Url) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        #[cfg(windows)]
        set_webview_low_memory(&window, false);
        return;
    }

    std::thread::spawn(move || {
        if let Err(err) = create_or_focus_main_window(&app, &local_url) {
            tracing::error!("failed to recreate main window: {err}");
        }
    });
}

#[cfg(windows)]
fn set_webview_low_memory(window: &WebviewWindow, low_memory: bool) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, ICoreWebView2_19,
    };
    use windows_core::Interface;

    let _ = window.with_webview(move |webview| {
        let controller = webview.controller();
        unsafe {
            let _ = controller.SetIsVisible(!low_memory);
            if let Ok(core) = controller.CoreWebView2()
                && let Ok(core19) = core.cast::<ICoreWebView2_19>()
            {
                let _ =
                    core19.SetMemoryUsageTargetLevel(COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_dimensions_expand_height_when_unlocked_for_drag_bar() {
        let min_locked = overlay_dimensions(OverlayMode::Minimal, true);
        let min_unlocked = overlay_dimensions(OverlayMode::Minimal, false);
        assert_eq!(min_locked, (436.0, 48.0));
        assert_eq!(min_unlocked, (436.0, 88.0));

        let tac_locked = overlay_dimensions(OverlayMode::Tactical, true);
        let tac_unlocked = overlay_dimensions(OverlayMode::Tactical, false);
        assert_eq!(tac_locked, (416.0, 184.0));
        assert_eq!(tac_unlocked, (416.0, 224.0));
    }

    #[test]
    fn compute_overlay_position_handles_all_anchors_and_secondary_monitors() {
        let mon_pos = (0, 0);
        let mon_size = (1920, 1080);
        let win_size = (416, 184);

        assert_eq!(
            compute_overlay_position(
                OverlayAnchor::TopLeft,
                None,
                None,
                mon_pos,
                mon_size,
                win_size
            ),
            (16, 16)
        );
        assert_eq!(
            compute_overlay_position(
                OverlayAnchor::TopCenter,
                None,
                None,
                mon_pos,
                mon_size,
                win_size
            ),
            ((1920 - 416) / 2, 16)
        );
        assert_eq!(
            compute_overlay_position(
                OverlayAnchor::TopRight,
                None,
                None,
                mon_pos,
                mon_size,
                win_size
            ),
            (1920 - 416 - 16, 16)
        );
        assert_eq!(
            compute_overlay_position(
                OverlayAnchor::BottomLeft,
                None,
                None,
                mon_pos,
                mon_size,
                win_size
            ),
            (16, 1080 - 184 - 16)
        );
        assert_eq!(
            compute_overlay_position(
                OverlayAnchor::BottomRight,
                None,
                None,
                mon_pos,
                mon_size,
                win_size
            ),
            (1920 - 416 - 16, 1080 - 184 - 16)
        );
        // Custom con coordenadas válidas preserva posición exacta incluso en monitor izquierdo (-x)
        assert_eq!(
            compute_overlay_position(
                OverlayAnchor::Custom,
                Some(-500),
                Some(120),
                mon_pos,
                mon_size,
                win_size
            ),
            (-500, 120)
        );
        // Custom sin coordenadas cae en TopRight del monitor
        assert_eq!(
            compute_overlay_position(
                OverlayAnchor::Custom,
                None,
                None,
                (-1920, 0),
                mon_size,
                win_size
            ),
            (-1920 + 1920 - 416 - 16, 16)
        );
    }
}
