# ProxyPal v0.4.36

**Released:** 2026-05-23

## Sidecar Upgrade: CLIProxyAPI v7.1.11 → v7.1.20

This release upgrades the bundled CLIProxyAPI sidecar from **v7.1.11** (May 18) to **v7.1.20** (May 23), jumping **9 releases** with new model integrations, protocol fixes, and reliability improvements.

### Why this matters

Picking up Gemini 3.5 Flash, xAI reasoning.effort, OpenAI image model compatibility, and critical fixes for HTTP CONNECT proxying, streaming tool_use blocks, and Claude Code attribution handling in translations.

### What changed (v7.1.12 → v7.1.20)

**Models & Providers:**

- **Gemini 3.5 Flash** models added to registry with dynamic thinking levels (v7.1.18)
- **xAI reasoning.effort** support (v7.1.14)
- **OpenAI image model compatibility** — image-capable models proxied via OpenAI endpoint (v7.1.15)
- **Grok Build 0.1** model added to registry (v7.1.20)
- **Codex**: enhanced reasoning levels, Switch tool docs (v7.1.17–18)
- **Gemini max output tokens** capped (v7.1.15)

**Fixes:**

- **HTTP CONNECT dialer support** — fixes proxy tunneling (v7.1.12)
- **Streaming tool_use blocks stabilized** for OpenAI→Claude (v7.1.12)
- **Claude Code attribution stripped** from non-Anthropic translations (v7.1.12)
- **Claude request conversion**: system→developer role handled (v7.1.20)
- **Reduced Codex tool call ID length** (v7.1.12)
- **Antigravity**: project_id fixes, credits fallback gate, Gemini thought signatures (v7.1.15, v7.1.19)
- **Codex context length stream errors** fixed (v7.1.19)
- **Empty text parts** skipped in Claude request conversion (v7.1.18)

**Infrastructure:**

- **mTLS certificate bootstrap via JWT** for Home connections (v7.1.12)
- **HOME_ADDR / HOME_PASSWORD** env var fallbacks (v7.1.12)
- **Redis**: timeout handling + subscription failover (v7.1.16)
- **Home CA fingerprint verification** (v7.1.15)
- **Reasoning effort** added to usage events (v7.1.18)
- **Upstream response headers** tracked in logging & usage (v7.1.13)
- Auth import paths updated to v7 (v7.1.20)

### ProxyPal code changes

- `src-tauri/binaries/cli-proxy-api-aarch64-apple-darwin` — v7.1.11 → v7.1.20
- `src-tauri/Cargo.toml:3` — version 0.4.35 → 0.4.36
- `src-tauri/tauri.conf.json:4` — version 0.4.35 → 0.4.36
- `package.json` — version 0.4.35 → 0.4.36

### Breaking changes check

Zero breakage. Same CLI flags. No ProxyPal-touched config fields changed.

### Verification

- Binary: `CLIProxyAPI Version: 7.1.20, Commit: aaec9194, BuiltAt: 2026-05-23T20:45:33Z`
- `cargo check` + `tsc --noEmit` — clean

---

_Full CLIProxyAPI changelog: https://github.com/router-for-me/CLIProxyAPI/releases_
