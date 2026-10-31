pub mod core;

mod adapters;
mod commands;
mod fatal;
mod link_registry;
mod shell_link;
mod taskbar;
mod tray;

use adapters::{
    AUTOSTART_ARG, DesktopTokens, SysinfoLister, SystemClock, Toast, UreqHttp, WindowsLocator,
    WindowsOs,
};
use core::link::link_argument;
use core::ports::{ClaudeLocator, ProcessLister, Roots};
use core::process::DesktopProcesses;
use core::shortcuts::account_to_open;
use core::{CloseAction, Core, OverrideAware, Ports};
use std::sync::Arc;
use tauri::{Manager, WindowEvent};
use tauri_plugin_notification::NotificationExt;

/// The core, and the locator it was built with, which the account icons need.
fn build_core(roots: Roots, app: tauri::AppHandle) -> (Arc<Core>, Arc<dyn ClaudeLocator>) {
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
    let core = Arc::new(Core::new(Ports {
        roots,
        http: Arc::new(UreqHttp::new()),
        clock: Arc::new(SystemClock),
        tokens: Arc::new(DesktopTokens),
        processes: Arc::new(DesktopProcesses::new(lister.clone(), locator.clone())),
        os: Arc::new(WindowsOs::new(locator.clone(), lister, toast)),
    }));
    (core, locator)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    fatal::install_panic_hook();
    let started = tauri::Builder::default()
        // Must be registered first: a second launch focuses the running window.
        // A shortcut click lands here too: it opens its account instead.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if !tray::route_from_command_line(app, &args)
                && !tray::open_from_command_line(app, &args)
            {
                tray::show_main_window(app);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
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
            let (core, locator) = build_core(
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
            let args: Vec<String> = std::env::args().collect();
            let boot = args.iter().any(|arg| arg == AUTOSTART_ARG);
            // A shortcut click opens its account, not clawshell's window.
            let clicked = account_to_open(&args).is_some();
            // Likewise a sign-in link that started clawshell.
            let linked = link_argument(&args).is_some();
            if !clicked && !linked && core.shows_window_at_launch(boot) {
                tray::show_main_window(app.handle());
            }
            // Take the link handler back if Claude rewrote it while clawshell was closed.
            core.keep_link_handler();
            app.manage(feed);
            app.manage(core);
            app.manage(locator);
            tray::open_from_command_line(app.handle(), &args);
            tray::route_from_command_line(app.handle(), &args);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            commands::account_icon,
            commands::add_account,
            commands::launch_account,
            commands::set_launch_options,
            commands::rename_account,
            commands::set_account_colour,
            commands::open_account_folder,
            commands::delete_account,
            commands::add_shortcuts,
            commands::refresh_usage,
            commands::month_cost,
            commands::heatmap,
            commands::get_settings,
            commands::update_settings,
            commands::set_sign_in_routing
        ])
        .run(tauri::generate_context!());
    if started.is_err() {
        fatal::exit_after_startup_failure();
    }
}
