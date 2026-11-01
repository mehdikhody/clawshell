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

    /// Saves the current version.
    pub fn accept_disclaimer(&self) -> io::Result<()> {
        let mut settings = self.settings();
        settings.disclaimer_accepted = Some(DISCLAIMER_VERSION);
        settings.save(&self.ports.roots.local_app_data)
    }

    /// Runs `start` unless the background work has already started, then
    /// hands each held launch's arguments to `replay`, oldest first. The work
    /// counts as started only once `start` succeeds: when it fails, the held
    /// arguments stay queued and a retry runs it again. Every step runs under
    /// the lock, so a launch that arrives meanwhile queues behind the held
    /// ones, or is applied directly once they were replayed.
    pub fn start_background_once<E>(
        &self,
        start: impl FnOnce() -> Result<(), E>,
        mut replay: impl FnMut(Vec<String>),
    ) -> Result<(), E> {
        let mut deferred = self.deferred.lock().unwrap();
        if deferred.started {
            return Ok(());
        }
        start()?;
        deferred.started = true;
        std::mem::take(&mut deferred.held)
            .into_iter()
            .for_each(&mut replay);
        Ok(())
    }

    /// Keeps a launch's arguments for replay once the background work starts.
    /// False when it already has: apply them now.
    pub fn hold_launch_args(&self, args: Vec<String>) -> bool {
        let mut deferred = self.deferred.lock().unwrap();
        if deferred.started {
            return false;
        }
        deferred.held.push(args);
        true
    }
}
