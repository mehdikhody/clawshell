//! The tray icon and its menu. What the menu offers is decided by the core
//! (`Core::tray_entries`); this only draws it and routes clicks back.

use crate::core::link::link_argument;
use crate::core::shortcuts::account_to_open;
use crate::core::{Core, TrayEntry};
use std::sync::Arc;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

const TRAY_ID: &str = "main";
const OPEN: &str = "open";
const ADD: &str = "add";
const QUIT: &str = "quit";
const ACCOUNT_PREFIX: &str = "account:";

/// Brings the main window forward from the tray, a second launch or a click.
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Opens the account a shortcut click names on the command line, or brings it
/// forward when it is already running. Returns false when `args` name no
/// account. A failed launch shows the main window, as the tray does.
pub fn open_from_command_line(app: &AppHandle, args: &[String]) -> bool {
    if account_to_open(args).is_none() {
        return false;
    }
    let (app, args) = (app.clone(), args.to_vec());
    std::thread::spawn(move || {
        let core = app.state::<Arc<Core>>().inner().clone();
        if matches!(core.open_requested(&args), Some(Err(_))) {
            show_main_window(&app);
        }
        refresh(&app, &core);
    });
    true
}

/// Forwards the `claude://` link a launch carries (Windows runs clawshell
/// with the link as its argument) to the account that asked for it. Returns
/// false when `args` carry no link. A refused or failed link is dropped
/// without a trace: nothing about it is logged or shown.
pub fn route_from_command_line(app: &AppHandle, args: &[String]) -> bool {
    let Some(link) = link_argument(args).map(str::to_string) else {
        return false;
    };
    let app = app.clone();
    std::thread::spawn(move || {
        let core = app.state::<Arc<Core>>().inner().clone();
        let _ = core.route_link(&link);
        refresh(&app, &core);
    });
    true
}

pub fn build(app: &AppHandle, core: &Core) -> tauri::Result<()> {
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("clawshell")
        .menu(&menu(app, core)?)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu_click(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Redraws the menu so the ticks and the account list follow reality.
pub fn refresh(app: &AppHandle, core: &Core) {
    let app = app.clone();
    let Ok(menu_now) = menu(&app, core) else {
        return;
    };
    let target = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = target.tray_by_id(TRAY_ID) {
            let _ = tray.set_menu(Some(menu_now));
        }
    });
}

fn menu(app: &AppHandle, core: &Core) -> tauri::Result<Menu<tauri::Wry>> {
    // Without a readable list the menu still offers the fixed items.
    let entries = core.tray_entries().unwrap_or_else(|_| {
        vec![
            TrayEntry::Open,
            TrayEntry::Separator,
            TrayEntry::AddAccount,
            TrayEntry::Quit,
        ]
    });
    let labels = core.tray_labels();
    let mut items: Vec<Box<dyn IsMenuItem<tauri::Wry>>> = Vec::new();
    for entry in entries {
        items.push(match entry {
            TrayEntry::Open => Box::new(MenuItem::with_id(
                app,
                OPEN,
                labels.open,
                true,
                None::<&str>,
            )?),
            TrayEntry::Separator => Box::new(PredefinedMenuItem::separator(app)?),
            TrayEntry::Account { id, label, running } => Box::new(CheckMenuItem::with_id(
                app,
                format!("{ACCOUNT_PREFIX}{id}"),
                label,
                true,
                running,
                None::<&str>,
            )?),
            TrayEntry::AddAccount => Box::new(MenuItem::with_id(
                app,
                ADD,
                labels.add_account,
                true,
                None::<&str>,
            )?),
            TrayEntry::Quit => Box::new(MenuItem::with_id(
                app,
                QUIT,
                labels.quit,
                true,
                None::<&str>,
            )?),
        });
    }
    let refs: Vec<&dyn IsMenuItem<tauri::Wry>> = items.iter().map(|i| i.as_ref()).collect();
    Menu::with_items(app, &refs)
}

fn on_menu_click(app: &AppHandle, id: &str) {
    match id {
        OPEN => show_main_window(app),
        QUIT => app.exit(0),
        ADD => {
            let app = app.clone();
            std::thread::spawn(move || {
                let core = app.state::<Arc<Core>>().inner().clone();
                // The window explains a missing Claude Desktop.
                // The first Add also asks the window to offer sign-in routing.
                let added = core.add_account().is_ok();
                let offered = added && crate::commands::offer_routing_if_due(&app, &core);
                if !added || offered {
                    show_main_window(&app);
                }
                refresh(&app, &core);
            });
        }
        _ => {
            let Some(account) = id.strip_prefix(ACCOUNT_PREFIX) else {
                return;
            };
            let (app, account) = (app.clone(), account.to_string());
            std::thread::spawn(move || {
                let core = app.state::<Arc<Core>>().inner().clone();
                if core.launch_account(&account).is_err() {
                    show_main_window(&app);
                }
                // Clicking a ticked row unticks it on screen; put it right.
                refresh(&app, &core);
            });
        }
    }
}
