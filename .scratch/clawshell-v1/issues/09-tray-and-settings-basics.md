# 09: Tray and Settings basics

**What to build:**
- **Tray icon:** a menu with Open clawshell, the accounts (running ones ticked; clicking one launches or focuses it), Add account, and Quit.
- **Close button:** hides to the tray when "close to tray" is on, and quits when it's off.
- **Settings tab:**
  - Launch at boot switch (Windows autostart)
  - Start hidden switch (whether a boot launch shows the window or stays in the tray)
  - Close to tray switch
  - Claude.exe path override, used when detection fails, with validation
- **Persistence:** all settings are saved by the core and survive restarts.

See spec: Settings, user stories 7–11, 72–74, 79–82.

**Blocked by:** 03

**Status:** done

- [x] Tray menu items all work; running accounts are ticked
- [x] Close hides or quits according to the setting
- [x] Launch at boot registers and unregisters autostart; start hidden is respected at boot
- [x] A path override pointing at an invalid file is rejected; a valid one is used for launches
- [x] Settings persist across restarts
- [x] Core tests cover settings persistence and path validation; UI tests cover the switches and the path field
