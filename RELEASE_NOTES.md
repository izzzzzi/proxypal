# ProxyPal v0.4.41

**Released:** 2026-06-13

## Sidecar Upgrade: CLIProxyAPI v7.1.47 → v7.1.75

This release upgrades the bundled CLIProxyAPI sidecar from **v7.1.47** (June 6) to **v7.1.75** (June 13), picking up 28 upstream releases.

### Highlights (v7.1.48 → v7.1.75)

**Plugin system (v7.1.69 → v7.1.75):**

- **Plugin store** — new `/v0/management/plugin-store/*` endpoints for fetching and installing community plugins from latest GitHub releases (v7.1.69, v7.1.72)
- **Plugin delete endpoint** — new API to remove installed plugins (v7.1.75)
- **Plugin install timeout** — bounded plugin installation with timeout handling (v7.1.75)
- **Plugin unload handling** — preserves plugin config across updates (v7.1.70)
- **Plugin registry version optional** — `version` field in plugin manifests is now optional (v7.1.72)
- **CORS plugin support header** — exposes `X-CPA-SUPPORT-PLUGIN` header for cross-origin clients (v7.1.73)
- **Saved plugin config exposure** — `GET /v0/management/plugins/config` returns stored config (v7.1.72)

**HTML/JSON sanitization (v7.1.70):**

- New sanitization utilities integrated across plugins and APIs to prevent injection in plugin output

**Auto-updater (v7.1.66):**

- Refactored skip logic with unit tests for `autoUpdateSkipReason` — safer update decisions

**Antigravity (v7.1.74):**

- **Claude WebSearch bridged to native `googleSearch`** — Anthropic-format web search tool calls are now translated to Antigravity's native `googleSearch` tool, improving tool-call fidelity

**Translator (v7.1.75):**

- **Mid-conversation system messages consolidated** into initial system content for OpenAI/Codex responses — cleaner request payloads

**Translator (v7.1.69):**

- Usage token details now include cache input/output aggregation fields for finer-grained usage reporting

**Stability (v7.1.48 → v7.1.65):**

- 18 patch releases with internal model registry, antigravity executor, and management API fixes (sparse changelogs; recommend production smoke test)

### Compatibility

- All ProxyPal IPC endpoints (`/v1/models`, `/api/auth/status`, `/v0/management/auth-files`, usage sync) are stable across the v7.1.47 → v7.1.75 gap — **no code changes required**.
- New plugin store endpoints are not surfaced in the ProxyPal UI; they remain available in the sidecar's built-in management web UI for advanced users.

---

# ProxyPal v0.4.40

**Released:** 2026-06-06

## Sidecar Upgrade: CLIProxyAPI v7.1.44 → v7.1.47

This release upgrades the bundled CLIProxyAPI sidecar from **v7.1.44** (June 3) to **v7.1.47** (June 6), picking up 3 upstream releases with pluginhost capabilities, safemode example server, and file-backed request/response logging.

### What changed (v7.1.45 → v7.1.47)

**Plugin system (v7.1.47):**

- **pluginhost capabilities** — command-line flag handling and plugin execution for the plugin host subsystem

**Safemode (v7.1.46):**

- **Example API key warning server** — reference implementation for surfacing compromised-key warnings in safemode deployments

**Logging (v7.1.45):**

- **File-backed request/response sources** — enhanced API logging with persistent request/response capture

**Fixes (v7.1.45):**

- **xai orphaned tool_choice** — drops orphaned `tool_choice` when the Claude tools array is empty, preventing replay errors

---

## Notes

This is a sidecar-only bump — no ProxyPal UI or API surface changes.
