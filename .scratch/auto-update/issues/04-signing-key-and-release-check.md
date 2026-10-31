# 04: Signing key and end-to-end release check

**What to build:** The owner turns on real signed updates and checks them end to end. Generate the signing key pair, add the private key and its password as repo secrets, and replace ticket 01's placeholder with the real public key. Keep a backup of the private key outside the repo. Then tag a test release and confirm an older install updates itself. See spec: Mechanism (One-time owner step), Testing Decisions (Manual release check), Further Notes.

**Blocked by:** 02, 03

**Status:** ready-for-human

- [ ] Key pair generated; private key backed up outside the repo; both secrets added; real public key committed
- [ ] A test tag's release has `latest.json` and `.sig` files next to the installers
- [ ] An install of the previous version shows the badge; the dialog shows the version and notes; Update downloads, installs and restarts into the new version
- [ ] Claude windows opened from clawshell keep running across the update
