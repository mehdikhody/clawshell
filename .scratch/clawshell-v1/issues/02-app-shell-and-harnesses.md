# 02: App shell and test harnesses (prefactor)

**What to build:** Replace the Tauri greet template with clawshell's shell:
- **Window:** one frameless window, fixed 400×780, transparent for Windows 11 rounded corners, not resizable, no maximize.
- **Top bar:** a custom bar with a drag region, the app name, and minimize and close buttons.
- **Tabs:** a bottom tab bar with Usage, Accounts and Settings, each showing an empty screen.
- **Look and behaviour:** a dark-only theme in Claude Code colours, and single-instance behaviour (launching again focuses the existing window).

Stand up the Rust core module with its injected ports (filesystem roots, HTTP client, clock, token reader, process inspector, OS adapters). Add the two test harnesses from the spec:
- **Core harness:** builds temp fixture profile folders and a shared transcripts folder, with a scripted fake HTTP client and clock.
- **UI harness:** renders screens in Vitest browser mode with Tauri IPC mocked.

Use shadcn-svelte components first. See spec: Platform and stack, Testing Decisions.

**Blocked by:** None (can start immediately)

**Status:** done

- [x] App opens as a 400×780 frameless, rounded, non-resizable window that can be dragged by the top bar
- [x] Minimize works; close quits for now (close to tray comes in 09)
- [x] The three bottom tabs switch screens
- [x] Dark Claude Code theme tokens replace the scaffold theme
- [x] Launching a second copy focuses the first
- [x] Template greet code and examples are removed
- [x] Core harness: one passing example test drives a core operation against a fixture folder with fake HTTP and clock
- [x] UI harness: one passing example test renders the tab bar with IPC mocked
