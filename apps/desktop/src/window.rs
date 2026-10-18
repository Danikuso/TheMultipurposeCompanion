use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use tauri::WebviewWindow;
use tauri::webview::Url;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const MAIN_WINDOW_TITLE: &str = "TMC — The Multipurpose Companion";

pub fn create_or_focus_main_window(app: &AppHandle, local_url: &Url) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        #[cfg(windows)]
        set_webview_low_memory(&window, false);
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        MAIN_WINDOW_LABEL,
        WebviewUrl::External(local_url.clone()),
    )
    .title(MAIN_WINDOW_TITLE)
    .inner_size(1600.0, 900.0)
    .additional_browser_args(
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,Translate,SpareRendererForSitePerProcess,AudioServiceOutOfProcess,StorageServiceOutOfProcess --enable-features=NetworkServiceInProcess2 --renderer-process-limit=1 --in-process-gpu --disable-gpu --disable-gpu-compositing --disable-gpu-shader-disk-cache --num-raster-threads=1 --js-flags=--max-old-space-size=32",
    )
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
