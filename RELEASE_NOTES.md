# ProxyPal v0.4.39

**Released:** 2026-06-05

## Antigravity/Claude Model Overlap Fix

When both **Antigravity OAuth** and **Claude** credentials coexisted, the sidecar sorted providers alphabetically — meaning `antigravity` took priority over `claude` for overlapping `gemini-claude-*` models. This caused the wrong provider to handle requests for these models.

**Fix:** ProxyPal now detects when both credential types are present and injects an `oauth-excluded-models` section into `proxy-config.yaml`, preventing Antigravity from registering `gemini-claude-*` model patterns. The sidecar already supported this config field — it just wasn't being populated.

Closes [#224](https://github.com/heyhuynhgiabuu/proxypal/issues/224).

## Antigravity Gemini 3.1 & 3.5 Model Support

- Added missing Antigravity model mappings for Gemini 3.1 and 3.5 model series
- Updated bundled sidecar binary with expanded model registry entries

## Multi-File OAuth Auth File Upload

The **Auth Files** page now supports uploading **multiple JSON auth files** at once:

- File dialog changed to allow multi-file selection
- Uploads each file sequentially with **per-file progress indicator**
- Shows **summary toasts** — success count + per-file error details
- Provider detection extracted into reusable helper `detectProviderFromFilename`
- New i18n messages for en, vi, zh-CN

Closes [#223](https://github.com/heyhuynhgiabuu/proxypal/issues/223).

## CI & Maintenance

- Fixed 20 oxlint errors in `AuthFiles.tsx` (missing curly braces after `if` statements, property sort ordering)
- Fixed i18n object property sort ordering in `en.ts`, `vi.ts`, `zh-CN.ts`
- Version bumped to **v0.4.39**
