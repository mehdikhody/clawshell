# 04: Rename, Open folder, Delete

**What to build:** Each account row gets a ⋯ menu with three actions:
- **Rename:** changes only the label. The folder and uuid never change.
- **Open folder:** opens the profile folder in Explorer.
- **Delete:** asks for confirmation, then removes the profile folder and the account's metadata. Delete is refused while that account is running.

An account whose folder disappeared outside clawshell is flagged and offered for removal. See spec: user stories 22–27, 83.

**Blocked by:** 03

**Status:** resolved

- [x] Rename updates the label everywhere, and it persists across restarts
- [x] Open folder opens the right `clawshell-{uuid}` folder
- [x] Delete asks for confirmation, then removes the folder and the metadata
- [x] Delete is blocked, with a message, while the account is running
- [x] A missing folder is detected and offered for removal
- [x] Core and UI tests cover rename, delete-confirm, the delete-while-running refusal and the missing folder
