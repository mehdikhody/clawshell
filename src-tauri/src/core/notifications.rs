//! Limit notifications: which Windows notification, if any, a usage reading
//! earns. The decisions are pure ([`observe`]); the memory of what already
//! fired lives in `clawshell/notifications.json` so a restart repeats nothing.
//!
//! A notification fires at most once per account per window. A window is
//! identified by its `resets_at`, and the flags below belong to it: when the
//! window changes, they start over.

use super::settings::Settings;
use super::usage::{UsageSnapshot, UsageSource, WindowUsage};
use super::{Core, Language};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

const APP_DATA_DIR: &str = "clawshell";
const FILE: &str = "notifications.json";

/// Two `resets_at` readings closer than this are the same window (the endpoint
/// can wobble by fractions of a second).
const SAME_WINDOW_MS: i64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowKind {
    FiveHour,
    Weekly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Event {
    Warning,
    Limited,
    /// The window has moved on to a new reset time.
    Reset,
}

/// What one window has already told the user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowNotes {
    /// The window's `resets_at`, as first seen; its identity.
    resets_at: String,
    resets_at_ms: i64,
    warned: bool,
    limit_notified: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AccountNotes {
    five_hour: Option<WindowNotes>,
    weekly: Option<WindowNotes>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct Fired {
    accounts: HashMap<String, AccountNotes>,
}

fn file(local_app_data: &Path) -> PathBuf {
    local_app_data.join(APP_DATA_DIR).join(FILE)
}

impl Fired {
    /// Missing or unreadable memory starts empty.
    pub(super) fn load(local_app_data: &Path) -> Self {
        std::fs::read_to_string(file(local_app_data))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn save(&self, local_app_data: &Path) -> io::Result<()> {
        let path = file(local_app_data);
        std::fs::create_dir_all(path.parent().expect("file sits in a folder"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(
            &tmp,
            serde_json::to_vec_pretty(self).map_err(io::Error::other)?,
        )?;
        std::fs::rename(tmp, path)
    }
}

/// The switches and thresholds in force for one decision.
struct Rules {
    warn_on: bool,
    limit_on: bool,
    reset_on: bool,
    warn_at: f64,
    limit_at: f64,
}

impl Rules {
    fn of(settings: &Settings) -> Self {
        Rules {
            warn_on: settings.notify_warning,
            limit_on: settings.notify_limited,
            reset_on: settings.notify_reset,
            warn_at: f64::from(settings.warning_threshold),
            limit_at: f64::from(settings.limit_threshold),
        }
    }
}

/// Folds one reading of a window into what it already told the user, and
/// returns the notifications that reading earns.
fn observe(notes: &mut Option<WindowNotes>, reading: &WindowUsage, rules: &Rules) -> Vec<Event> {
    let Some(resets_at) = reading.resets_at.as_deref() else {
        return Vec::new();
    };
    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(resets_at) else {
        return Vec::new();
    };
    let resets_at_ms = parsed.timestamp_millis();
    let utilization = reading.utilization;
    let mut events = Vec::new();

    let same_window = notes
        .as_ref()
        .is_some_and(|n| (n.resets_at_ms - resets_at_ms).abs() < SAME_WINDOW_MS);
    if !same_window {
        // Only a window seen before can have reset; the first reading after
        // launch with no memory of it says nothing either way.
        if rules.reset_on && notes.is_some() {
            events.push(Event::Reset);
        }
        *notes = Some(WindowNotes {
            resets_at: resets_at.to_string(),
            resets_at_ms,
            warned: false,
            limit_notified: false,
        });
    }
    let notes = notes.as_mut().expect("set above");

    if utilization >= rules.limit_at {
        if rules.limit_on && !notes.limit_notified {
            events.push(Event::Limited);
            notes.limit_notified = true;
            // Passing the warning on the way is not worth a second alert.
            notes.warned = true;
        } else if !rules.limit_on && rules.warn_on && !notes.warned {
            events.push(Event::Warning);
            notes.warned = true;
        }
    } else if utilization >= rules.warn_at && rules.warn_on && !notes.warned {
        events.push(Event::Warning);
        notes.warned = true;
    }
    events
}

impl Language {
    fn window_name(self, window: WindowKind) -> &'static str {
        match (self, window) {
            (Language::Fa, WindowKind::FiveHour) => "۵ ساعته",
            (Language::Fa, WindowKind::Weekly) => "هفتگی",
            (Language::Ar, WindowKind::FiveHour) => "٥ ساعات",
            (Language::Ar, WindowKind::Weekly) => "أسبوعية",
            (Language::Ru, WindowKind::FiveHour) => "5-часовое",
            (Language::Ru, WindowKind::Weekly) => "недельное",
            (Language::Zh, WindowKind::FiveHour) => "5 小时",
            (Language::Zh, WindowKind::Weekly) => "每周",
            (Language::En, WindowKind::FiveHour) => "5-hour",
            (Language::En, WindowKind::Weekly) => "weekly",
        }
    }
}

/// Notification text. Account labels only: never an email or a token.
fn text(
    language: Language,
    event: Event,
    window: WindowKind,
    label: &str,
    percent: f64,
) -> (String, String) {
    let name = language.window_name(window);
    let percent = percent.round() as i64;
    match (language, event) {
        (Language::Fa, Event::Warning) => (
            format!("{label} به سقف مصرف نزدیک می‌شود"),
            format!("مصرف {name} آن به {percent}٪ رسیده است."),
        ),
        (Language::Fa, Event::Limited) => (
            format!("{label} به سقف مصرف رسید"),
            format!("مصرف {name} آن به {percent}٪ رسیده است. به حساب دیگری بروید."),
        ),
        (Language::Fa, Event::Reset) => (
            format!("پنجرهٔ {name} {label} بازنشانی شد"),
            format!("مصرف آن اکنون به {percent}٪ رسیده است."),
        ),
        (Language::Ar, Event::Warning) => (
            format!("الحساب {label} يقترب من حدّه"),
            format!("بلغ استخدام نافذة {name} {percent}٪."),
        ),
        (Language::Ar, Event::Limited) => (
            format!("الحساب {label} بلغ حدّه"),
            format!("بلغ استخدام نافذة {name} {percent}٪. انتقل إلى حساب آخر."),
        ),
        (Language::Ar, Event::Reset) => (
            format!("أُعيد ضبط نافذة {name} للحساب {label}"),
            format!("عاد الاستخدام إلى {percent}٪."),
        ),
        (Language::Ru, Event::Warning) => (
            format!("{label}: лимит близко"),
            format!("Расход за {name} окно: {percent}%."),
        ),
        (Language::Ru, Event::Limited) => (
            format!("Лимит {label} исчерпан"),
            format!("Расход за {name} окно: {percent}%. Переключитесь на другой аккаунт."),
        ),
        (Language::Ru, Event::Reset) => (
            format!("Сброшено {name} окно: {label}"),
            format!("Теперь расход: {percent}%."),
        ),
        (Language::Zh, Event::Warning) => (
            format!("{label} 即将达到用量上限"),
            format!("{name}用量已达 {percent}%。"),
        ),
        (Language::Zh, Event::Limited) => (
            format!("{label} 已达用量上限"),
            format!("{name}用量已达 {percent}%。请切换到其他账户。"),
        ),
        (Language::Zh, Event::Reset) => (
            format!("{label}：{name}窗口已重置"),
            format!("用量已回到 {percent}%。"),
        ),
        (Language::En, Event::Warning) => (
            format!("{label} is nearing its limit"),
            format!("Its {name} usage is at {percent}%."),
        ),
        (Language::En, Event::Limited) => (
            format!("{label} hit its limit"),
            format!("Its {name} usage is at {percent}%. Switch to another account."),
        ),
        (Language::En, Event::Reset) => (
            format!("{label}'s {name} window has reset"),
            format!("It is back to {percent}% used."),
        ),
    }
}

impl Core {
    /// Sends the notifications `snapshot` earns for one account and remembers
    /// that they went out. Only a live reading counts: a stale or local one
    /// carries no `resets_at` news.
    pub(super) fn notify_usage(&self, id: &str, label: &str, snapshot: &UsageSnapshot) {
        if snapshot.source != UsageSource::Live {
            return;
        }
        let settings = self.settings();
        let rules = Rules::of(&settings);
        let mut sends = Vec::new();
        {
            let mut fired = self.fired.lock().unwrap();
            let before = fired.accounts.get(id).cloned();
            let account = fired.accounts.entry(id.to_string()).or_default();
            for (kind, reading, notes) in [
                (
                    WindowKind::FiveHour,
                    &snapshot.five_hour,
                    &mut account.five_hour,
                ),
                (WindowKind::Weekly, &snapshot.weekly, &mut account.weekly),
            ] {
                let Some(reading) = reading else { continue };
                for event in observe(notes, reading, &rules) {
                    sends.push(text(
                        settings.language,
                        event,
                        kind,
                        label,
                        reading.utilization,
                    ));
                }
            }
            if before.as_ref() != fired.accounts.get(id) {
                // A failed save only risks one repeat after a restart.
                let _ = fired.save(&self.ports.roots.local_app_data);
            }
        }
        for (title, body) in sends {
            self.ports.os.notify(&title, &body);
        }
    }
}

impl Core {
    /// Runs the readings already held through the current rules, so a changed
    /// switch or threshold takes effect without waiting for the next poll.
    pub(super) fn notify_held_usage(&self) {
        let Ok(accounts) = self.known_accounts() else {
            return;
        };
        for account in accounts {
            if let Some(snapshot) = &account.usage {
                self.notify_usage(&account.id, &account.label, snapshot);
            }
        }
    }
}
