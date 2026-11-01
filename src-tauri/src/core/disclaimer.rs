//! The first-use disclaimer. Until it is accepted only the window runs; the
//! watchers, polling, tray and link handling wait, and launch arguments are
//! held for replay.

use super::Core;
use serde::Serialize;
use std::io;

/// The version of the disclaimer text. Bump it whenever the text changes in a
/// meaningful way: users who accepted an older version see the dialog again.
pub const DISCLAIMER_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DisclaimerStatus {
    pub accepted: bool,
    pub version: u32,
}

/// Whether the deferred background work has started, and the launch
/// arguments held until it does.
#[derive(Default)]
pub(super) struct Deferred {
    started: bool,
    held: Vec<Vec<String>>,
}

/// What accepting the disclaimer asks the shell to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accepted {
    /// True the first time only: the shell starts the background work.
    pub start_background: bool,
    /// The arguments held while the disclaimer was pending, oldest first.
    pub held: Vec<Vec<String>>,
}

impl Core {
    pub fn disclaimer_status(&self) -> DisclaimerStatus {
        DisclaimerStatus {
            accepted: self
                .settings()
                .disclaimer_accepted
                .is_some_and(|version| version >= DISCLAIMER_VERSION),
            version: DISCLAIMER_VERSION,
        }
    }

    /// Saves the current version and hands back what was waiting: whether
    /// the background work still has to start, and the held arguments.
    pub fn accept_disclaimer(&self) -> io::Result<Accepted> {
        let mut deferred = self.deferred.lock().unwrap();
        let mut settings = self.settings();
        settings.disclaimer_accepted = Some(DISCLAIMER_VERSION);
        settings.save(&self.ports.roots.local_app_data)?;
        Ok(Accepted {
            start_background: !std::mem::replace(&mut deferred.started, true),
            held: std::mem::take(&mut deferred.held),
        })
    }

    /// Claims the right to start the background work. True once per run.
    pub fn start_background_once(&self) -> bool {
        !std::mem::replace(&mut self.deferred.lock().unwrap().started, true)
    }

    /// Keeps a launch's arguments for replay after acceptance. False when
    /// the background work has already started: apply them now.
    pub fn hold_launch_args(&self, args: Vec<String>) -> bool {
        let mut deferred = self.deferred.lock().unwrap();
        if deferred.started {
            return false;
        }
        deferred.held.push(args);
        true
    }
}
