# 03: Releases publish signed installers and latest.json

**What to build:** A `v*` tag release uploads the signed updater artifacts and `latest.json` next to the installers, so ticket 01's updater endpoint resolves. The bundle config turns on updater artifacts. The release workflow passes `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` from repo secrets to tauri-action. A local `tauri build` with no signing key must keep working, or the README must say how to build locally without one. See spec: Mechanism (Release workflow).

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] The bundle config creates updater artifacts
- [ ] The release workflow passes both signing secrets to tauri-action and publishes `latest.json`
- [ ] A local `tauri build` without the signing key works, or the README documents how to build locally without one
- [ ] The owner steps (generate the key, add the two secrets, commit the public key, keep an off-repo backup of the private key) are written down where ticket 04 can follow them
