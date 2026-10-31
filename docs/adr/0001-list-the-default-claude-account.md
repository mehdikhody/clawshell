# List Claude's own profile as the default account

clawshell first left the original Claude Desktop sign-in alone and unlisted (spec story 37), so it never touched data it didn't create. We now list that profile as a fixed first account, `default`, labelled "Claude", because most users keep a subscription there and want its limits and cost beside the others. It stays hands-off where it matters: the profile folder is only read (its token is never refreshed), it can never be deleted, it gets no badge, shortcuts or taskbar identity of its own, and clawshell starts it without `--user-data-dir`, exactly as Claude's own Start-menu entry does. It can be renamed and given launch options, which are stored in clawshell's records under the reserved id `default`.

## Consequences

- The folder is not under clawshell's control, so it is found, not created: current Claude keeps it in `%LOCALAPPDATA%\Claude-Data\{folder}` (for the Store build, the package family name in lower case), older versions in `%APPDATA%\Claude`, which the Store build sees in its package's `LocalCache\Roaming`. Whichever holds a `config.json` wins. If Claude moves it again, detection needs updating.
- A Claude process inside the install with no `--user-data-dir` now counts as the default account running. It used to be ignored.
- A Claude started directly from the Store build's `WindowsApps` folder still runs inside its package, so clawshell can start the default account by its exe path and it opens the same profile as the Start-menu entry.
