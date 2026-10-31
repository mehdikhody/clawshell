pub mod core;

mod adapters;
mod commands;
mod tray;

use adapters::{
    AUTOSTART_ARG, DesktopTokens, SysinfoLister, SystemClock, Toast, UreqHttp, WindowsLocator,
    WindowsOs,
};
use core::ports::{ClaudeLocator, ProcessLister, Roots};
use core::process::DesktopProcesses;
use core::{CloseAction, Core, OverrideAware, Ports};
use std::sync::Arc;
use tauri::{Manager, WindowEvent};
use tauri_plugin_notification::NotificationExt;

fn build_core(roots: Roots, app: tauri::AppHandle) -> Arc<Core> {
    let detected = Arc::new(WindowsLocator::new(roots.local_app_data.clone()));
    // Detection plus the saved Claude.exe override, shared by launching and
    // by the running check.
    let locator: Arc<dyn ClaudeLocator> =
        Arc::new(OverrideAware::new(detected, roots.local_app_data.clone()));
    let lister: Arc<dyn ProcessLister> = Arc::new(SysinfoLister);
    // A toast that cannot be shown (notifications blocked in Windows) is dropped.
    let toast: Toast = Box::new(move |title, body| {
        let _ = app.notification().builder().title(title).body(body).show();
    });
    Arc::new(Core::new(Ports {
        roots,
        http: Arc::new(UreqHttp::new()),
        clock: Arc::new(SystemClock),
        tokens: Arc::new(DesktopTokens),
        processes: Arc::new(DesktopProcesses::new(lister.clone(), locator.clone())),
        os: Arc::new(WindowsOs::new(locator, lister, toast)),
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first: a second launch focuses the running window.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "main"
            {
                match window.app_handle().state::<Arc<Core>>().close_action() {
                    CloseAction::HideToTray => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    CloseAction::Quit => window.app_handle().exit(0),
                }
            }
        })
        .setup(|app| {
            let paths = app.path();
            let core = build_core(
                Roots {
                    local_app_data: paths.local_data_dir()?,
                    home: paths.home_dir()?,
                },
                app.handle().clone(),
            );
            let feed = commands::AccountFeed::new(app.handle().clone(), core.clone());
            commands::watch_accounts(feed.clone());
            commands::poll_usage(core.clone(), feed.clone());
            tray::build(app.handle(), &core)?;
            // The window starts hidden; a boot launch may keep it that way.
            let boot = std::env::args().any(|arg| arg == AUTOSTART_ARG);
            if core.shows_window_at_launch(boot) {
                tray::show_main_window(app.handle());
            }
            app.manage(feed);
            app.manage(core);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            commands::add_account,
            commands::launch_account,
            commands::rename_account,
            commands::open_account_folder,
            commands::delete_account,
            commands::refresh_usage,
            commands::month_cost,
            commands::heatmap,
            commands::get_settings,
            commands::update_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
