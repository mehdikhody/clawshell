# 02: Update dialog: release notes, download progress, install and restart

**What to build:** Clicking the header badge opens a dialog built on the existing Dialog component. It shows the new version in the title and the release notes as scrollable plain text, with Later and Update buttons. Later closes the dialog and the badge stays. Update downloads the signed installer and shows a Progress bar: a percentage when the download size is known, indeterminate otherwise. It then installs and restarts clawshell. If the download or install fails, the dialog shows an inline error and a Retry button, and the current version keeps running. Every string is in English and Persian, and the dialog is right-to-left in Persian. See spec: Dialog, Strings.

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] The badge opens the dialog with the version and release notes; Later closes it and the badge remains
- [ ] Update moves the shared state through downloading (with progress) and installing, then calls relaunch
- [ ] Progress shows a percentage when the content length is known and is indeterminate when it isn't
- [ ] A failure shows the error and Retry; Retry starts the download again
- [ ] `update_title` (with version), `update_notes`, `update_now`, `update_later`, `update_downloading`, `update_installing`, `update_failed` and `update_retry` exist in English and Persian
- [ ] Unit tests cover the state moving through progress to install and to failure
- [ ] Component tests cover opening the dialog, progress then relaunch, failure with Retry, and the dialog rendering right to left in Persian
