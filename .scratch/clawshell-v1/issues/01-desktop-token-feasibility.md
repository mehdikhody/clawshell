# 01: Feasibility check — Desktop token works for usage and profile

**What to build:** A developer-only command that, given one Claude Desktop profile folder, reads `oauth:tokenCacheV2` from its `config.json`, gets the AES key by DPAPI-unwrapping `os_crypt.encrypted_key` from its `Local State`, decrypts the token cache in memory, extracts the access token and its expiry, and calls the OAuth profile endpoint and the OAuth usage endpoint (with the `oauth-2025-04-20` beta header) once each. It prints only HTTP status codes, response key names, and whether the token had expired. It never prints the token, email or values. The outcome decides whether the spec's identity and usage design holds or the local-history and claude.ai-cache fallback is needed. See spec: Identity and usage.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Decrypts `oauth:tokenCacheV2` for a real signed-in profile on this machine without writing secrets anywhere
- [x] Documents the decrypted token-cache structure (key names only), including where the access token and expiry live
- [x] Reports the HTTP status of the profile and usage endpoints, and the response key names
- [x] Findings, including whether the token is accepted, are appended to the spec's Further Notes; if it is rejected, a follow-up note says which fallback replaces it
- [x] No token refresh is attempted
