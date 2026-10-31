pub mod core;

mod adapters;
mod commands;

use adapters::{DesktopTokens, SysinfoLister, SystemClock, UreqHttp, WindowsLocator, WindowsOs};
use core::ports::{ProcessLister, Roots};
use core::process::DesktopProcesses;
use core::{Core, Ports};
use std::sync::Arc;
use tauri::Manager;

fn build_core(roots: Roots) -> Arc<Core> {
    let locator = Arc::new(WindowsLocator::new(roots.local_app_data.clone()));
    let lister: Arc<dyn ProcessLister> = Arc::new(SysinfoLister);
    Arc::new(Core::new(Ports {
        roots,
        http: Arc::new(UreqHttp::new()),
        clock: Arc::new(SystemClock),
        tokens: Arc::new(DesktopTokens),
        processes: Arc::new(DesktopProcesses::new(lister.clone(), locator.clone())),
        os: Arc::new(WindowsOs::new(locator, lister)),
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first: a second launch focuses the running window.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let paths = app.path();
            let core = build_core(Roots {
                local_app_data: paths.local_data_dir()?,
                home: paths.home_dir()?,
            });
            let feed = commands::AccountFeed::new(app.handle().clone(), core.clone());
            commands::watch_accounts(feed.clone());
            commands::poll_usage(core.clone(), feed.clone());
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
            commands::refresh_usage
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
