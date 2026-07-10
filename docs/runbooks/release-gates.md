# Release Gates

Version: <!-- VERSION --> (semver, from `src-tauri/Cargo.toml`)
App identifier: `com.proxypal.app`
Build command: `pnpm tauri build` (Tauri v2)
Binary target: `cli-proxy-api-<target>` (downloaded from the pinned, checksum-verified CLIProxyAPI release into `src-tauri/binaries/` during CI/release builds)

> **How to use:** Release Manager runs through each gate in order. Every PASS/FAIL line
> is raw terminal output pasted into the Gate Log below. A gate is OPEN (not yet passed)
> until filled. All gates must be CLOSED before release is signed.

---

## Gate 0 — Preflight

Checklist before any build starts.

- [ ] Last commit on `main` has passing CI: `main` CI workflow green
- [ ] All `src-tauri/src/` compiles: `cd src-tauri && cargo check 2>&1`
- [ ] All `src/` type-checks: `pnpm tsc --noEmit 2>&1`
- [ ] Sidecar smoke test passed on every release target after the pinned binary download: `node scripts/sidecar-smoke.mjs --binary src-tauri/binaries/cli-proxy-api-<target>`
- [ ] Sidecar version matches app: `cat scripts/sidecar-version`
- [ ] RELEASE_NOTES.md updated with changelog for this version
- [ ] Tag pushed: `git tag v<VERSION> && git push origin v<VERSION>`

**Gate Log (raw output):**

```
$ pnpm tsc --noEmit 2>&1
<PASTE HERE>

$ cd src-tauri && cargo check 2>&1
<PASTE HERE>

$ node scripts/sidecar-smoke.mjs --binary src-tauri/binaries/cli-proxy-api-<target>
<PASTE HERE>

$ cat scripts/sidecar-version
<PASTE HERE>
```

---

## Gate 1 — Clean-install artifact matrix

Build on every supported platform + architecture. Artifacts are produced by the
GitHub release workflow `.github/workflows/release.yml` (triggered on tag push).
Per-platform matrix:

| Platform         | Target Arch | Runner OS        | Primary artifact                    | Also included      |
| ---------------- | ----------- | ---------------- | ----------------------------------- | ------------------ |
| macOS            | ARM64       | `macos-latest`   | `ProxyPal_<VERSION>_aarch64.dmg`    | `.tar.gz` variant  |
| macOS            | x86_64      | `macos-13`       | `ProxyPal_<VERSION>_x64.dmg`        | `.tar.gz` variant  |
| Windows          | x86_64      | `windows-latest` | `ProxyPal_<VERSION>_x64.msi`        | `.msi.zip`         |
| Linux (deb)      | amd64       | `ubuntu-latest`  | `ProxyPal_<VERSION>_amd64.deb`      | `AppImage`, `.rpm` |
| Linux (AppImage) | amd64       | `ubuntu-latest`  | `ProxyPal_<VERSION>_amd64.AppImage` | —                  |

**Check:** Download all artifacts from the release draft. On each platform, clean-install
(fresh VM / bare-metal, no prior ProxyPal):

```bash
# macOS (ARM)
curl -LO https://github.com/<owner>/proxypal/releases/download/v<VERSION>/ProxyPal_<VERSION>_aarch64.dmg
# mount + drag to Applications, open

# macOS (Intel)
curl -LO https://github.com/<owner>/proxypal/releases/download/v<VERSION>/ProxyPal_<VERSION>_x64.dmg
# mount + drag to Applications, open

# Windows
# Download ProxyPal_<VERSION>_x64.msi, double-click, install

# Linux (deb)
curl -LO https://github.com/<owner>/proxypal/releases/download/v<VERSION>/ProxyPal_<VERSION>_amd64.deb
sudo dpkg -i ./ProxyPal_<VERSION>_amd64.deb
# Or AppImage:
curl -LO https://github.com/<owner>/proxypal/releases/download/v<VERSION>/ProxyPal_<VERSION>_amd64.AppImage
chmod +x ./ProxyPal_<VERSION>_amd64.AppImage
./ProxyPal_<VERSION>_amd64.AppImage
```

**Pass criteria:** The app launches (splash/window appears) on every platform without
error dialogs, missing-library popups, or crash-on-start.

**Gate Log:**

```
=== macOS ARM64 ===
$ <paste raw download + open output>

=== macOS x86_64 ===
$ <paste raw download + open output>

=== Windows x86_64 ===
<paste MSI install log or screen>

=== Linux amd64 (deb) ===
$ sudo dpkg -i ./ProxyPal_<VERSION>_amd64.deb 2>&1
<PASTE HERE>
$ proxypal --version 2>&1 || echo "check app launch"
<PASTE HERE>

=== Linux (AppImage) ===
$ ./ProxyPal_<VERSION>_amd64.AppImage 2>&1
<PASTE HERE>
```

---

## Gate 2 — Sidecar smoke test

Run after install on each platform. The sidecar binary is bundled in the app
and spawned at first use.

```bash
cd scripts
node sidecar-smoke.mjs 2>&1
```

**Pass criteria:** Script exits 0. Output shows `[PASS]` for every probe
(version match, HTTP serve, health endpoint, shutdown).

**Gate Log:**

```
$ node scripts/sidecar-smoke.mjs 2>&1
<PASTE HERE>
```

---

## Gate 3 — OAuth / API-key smoke tests

Test every supported provider end-to-end. Excluding a provider requires a
justified comment (e.g. "Gemini OAuth down upstream").

Supported providers (source: `src/pages/AuthFiles.tsx`):

- **Antigravity** — API key
- **Claude** (Anthropic) — API key
- **Codex** — API key (OpenAI-compatible)
- **Gemini** (Google) — API key
- **Gemini CLI** — API key
- **Iflow** — API key
- **Kiro** — API key
- **Qwen** — API key
- **Vertex AI** (Google Cloud) — OAuth / service-account

| Provider    | Auth method        | Test action                                          |
| ----------- | ------------------ | ---------------------------------------------------- |
| Antigravity | API key            | Add key in Settings → connect -> send test prompt    |
| Claude      | API key            | Add key in Settings → connect -> send test prompt    |
| Codex       | API key            | Add key in Settings → connect -> send test prompt    |
| Gemini      | API key            | Add key in Settings → connect -> send test prompt    |
| Gemini CLI  | API key            | Same as Gemini; tests CLI-specific endpoint          |
| Iflow       | API key            | Add key in Settings → connect -> send test prompt    |
| Kiro        | API key            | Add key in Settings → connect -> send test prompt    |
| Qwen        | API key            | Add key in Settings → connect -> send test prompt    |
| Vertex AI   | OAuth (GCloud ADC) | Run `gcloud auth application-default login`; connect |

**Pass criteria:** For each provider: key accepted, proxy endpoint responds 2xx,
response decodes as valid JSON. Screenshot or terminal log for each.

**Gate Log (one block per provider):**

```
=== Antigravity ===
<key entry + test result>

=== Claude ===
<key entry + test result>

=== Codex ===
<key entry + test result>

=== Gemini ===
<key entry + test result>

=== Gemini CLI ===
<key entry + test result>

=== Iflow ===
<key entry + test result>

=== Kiro ===
<key entry + test result>

=== Qwen ===
<key entry + test result>

=== Vertex AI ===
<pre-auth: gcloud auth application-default login>
<test result>
```

**Custom upstream test** — Add a generic OpenAI-compatible endpoint:

```
Settings → Custom upstream → URL: https://api.example.com/v1
Key: <test key> → connect → send test prompt → verify response
```

---

## Gate 4 — Upgrade from prior release

Install the **current release** (v\<CURRENT-1\>), then upgrade to the candidate
(v\<CURRENT\>).

```bash
# 1. Install v<CURRENT-1> from official download
# 2. Launch, add one API key and send a test prompt
# 3. Quit the app
# 4. Install v<CURRENT> over the top (same platform)
#    - macOS: replace .app in /Applications
#    - Windows: MSI upgrade (automatic, same upgrade code)
#    - Linux: dpkg -i upgrade (automatic, same package name)
# 5. Launch again
```

**Pass criteria (all must hold):**

1. App launches without migration errors
2. Previously saved API keys are still present
3. Test prompt succeeds against at least Claude and Gemini
4. Window size/position state preserved (if persisted)
5. Settings (theme, locale) preserved

**Gate Log:**

```
=== Upgrade: macOS ARM64 ===
$ <paste install + upgrade log>

=== Upgrade: Windows x64 ===
<paste MSI install log>

=== Upgrade: Linux amd64 ===
$ sudo dpkg -i ./ProxyPal_<VERSION>_amd64.deb 2>&1
<PASTE HERE>
```

**Data-preservation check:**

```
$ <open app, verify keys via UI>
<PASTE RESULT>
```

---

## Gate 5 — Rollback

Expected rollback path for each platform:

| Platform | Rollback method                                                              |
| -------- | ---------------------------------------------------------------------------- |
| macOS    | Download prior .dmg; `rm -rf /Applications/ProxyPal.app`; re-install         |
| Windows  | Download prior .msi; run uninstall from Add/Remove Programs; install old MSI |
| Linux    | `sudo dpkg -r proxypal && sudo dpkg -i ./ProxyPal_<CURRENT-1>_amd64.deb`     |

**Check:** After rollback:

- [ ] App launches without errors
- [ ] Prior data directory is intact (config, keys, proxy history)
- [ ] API keys re-appear in the UI (config format backward-compatible)
- [ ] Re-upgrade to \<CURRENT\> succeeds

Data locations (for manual verification):

```bash
# macOS
ls ~/Library/Application\ Support/com.proxypal.app/
# Windows
dir %APPDATA%\com.proxypal.app\
# Linux
ls ~/.local/share/com.proxypal.app/
```

**Gate Log:**

```
$ <paste rollback commands + verify output>
```

---

## Gate 6 — Signing, notarization & artifact integrity

### 6a — macOS code signing + notarization

The release workflow codesigns and notarizes via `tauri-apps/tauri-action@v0`
when `APPLE_*` secrets are set.

```bash
# Verify signature on a macOS build machine
codesign -dvvv /Applications/ProxyPal.app 2>&1

# Verify notarization ticket
spctl --assess -vv /Applications/ProxyPal.app 2>&1

# Check hardened runtime
codesign -d --entitlements :- /Applications/ProxyPal.app 2>&1 | grep -i 'com.apple.security'
```

**Pass criteria:**

- `codesign -dvvv` output includes `Authority=Developer ID Application: <team>` and `Sealed Resources version=2`
- `spctl --assess -vv` prints `accepted`
- Hardened runtime is enabled (`com.apple.security.cs.allow-*` entitlements only as needed)

### 6b — Windows code signing

The action signs the MSI with the configured certificate.

```bash
# On Windows: verify digital signature
signtool verify /pa /v ProxyPal_<VERSION>_x64.msi
```

**Pass criteria:** Signature chain validates to a trusted CA, timestamp present.

### 6c — Linux (no codesigning, checksums only)

Linux builds are not codesigned. Verify by checksum.

```bash
sha256sum ProxyPal_<VERSION>_amd64.deb
sha256sum ProxyPal_<VERSION>_amd64.AppImage
```

### 6d — Updater manifest signature

The release workflow generates `updater.json` (or the Tauri v2 default manifest)
signed with `TAURI_SIGNING_PRIVATE_KEY`.

```bash
# Verify the updater manifest (available in release assets or generated by the action)
# The tauri-action outputs the manifest as a release asset.
# Decode and verify:
```

**Pass criteria:** `tauri` updater client can parse and verify the manifest signature.
Verify that the `signature` field in the manifest matches the artifact checksum:

```bash
# Open the release asset 'latest.json' or 'update-manifest.json'
# Check that the signature + pubdate correspond to this release
```

### 6e — Artifact inspection log

```
$ sha256sum ProxyPal_<VERSION>_aarch64.dmg
<PASTE>

$ sha256sum ProxyPal_<VERSION>_x64.dmg
<PASTE>

$ sha256sum ProxyPal_<VERSION>_x64.msi
<PASTE>

$ sha256sum ProxyPal_<VERSION>_amd64.deb
<PASTE>

$ sha256sum ProxyPal_<VERSION>_amd64.AppImage
<PASTE>

=== macOS codesign ===
$ codesign -dvvv /Applications/ProxyPal.app 2>&1
<PASTE>

$ spctl --assess -vv /Applications/ProxyPal.app 2>&1
<PASTE>

=== Windows signtool ===
$ signtool verify /pa /v ProxyPal_<VERSION>_x64.msi
<PASTE>
```

---

## Gate 7 — Release asset completeness

Final visual inspection of the release draft on GitHub.

- [ ] All 5+ platform artifacts present (see Gate 1 matrix)
- [ ] Updater manifest asset present (`latest.json` or `update-manifest.json`)
- [ ] Checksums file present (`SHA256SUMS.txt`)
- [ ] RELEASE_NOTES.md rendered correctly in release body
- [ ] Draft is marked as "Latest release" (not pre-release unless intended)
- [ ] Signed tag matches version

---

## Gate Log (master table)

```
Gate | Status | Date       | Tester
-----|--------|------------|-------
0    | PASS[] | yyyy-mm-dd | <name>
1    | PASS[] | yyyy-mm-dd | <name>
2    | PASS[] | yyyy-mm-dd | <name>
3    | PASS[] | yyyy-mm-dd | <name>
4    | PASS[] | yyyy-mm-dd | <name>
5    | PASS[] | yyyy-mm-dd | <name>
6    | PASS[] | yyyy-mm-dd | <name>
7    | PASS[] | yyyy-mm-dd | <name>
```

**Release verdict:** All 8 gates CLOSED on \<DATE\>.
Release tag: `v<VERSION>` | Publish release: [ ]

---

## Reference

- Build workflow: `.github/workflows/release.yml`
- CI workflow: `.github/workflows/ci.yml`
- Tauri config: `src-tauri/tauri.conf.json`
- Sidecar update: `scripts/update-sidecar.mjs`
- Sidecar smoke: `scripts/sidecar-smoke.mjs`
- Sidecar version: `scripts/sidecar-version`
- Release notes: `RELEASE_NOTES.md`
- Test plan: `TEST_PLAN.md`
- Type check: `pnpm tsc --noEmit` (`pnpm check:ts`)
- Parallel check: `pnpm check:parallel` (type + lint + format)
- Backend: `cd src-tauri && cargo check`
