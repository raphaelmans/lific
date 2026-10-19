# Read before changing or updating this fork

This is Raphael's **local-only, login-free fork** of Lific. Read this file and [SECURITY.md](SECURITY.md) before implementing features, updating dependencies, resolving conflicts, or importing upstream changes. Keep this file in the fork and update it deliberately when the implementation changes.

## The fork contract

An upstream update is acceptable only while these requirements remain true:

1. **One personal operator, always login-free.** `lific init` creates a local workspace without a password or auth-mode choice. Startup forces signup off and browser auto-login on, expires other historical users' sessions and tool keys, and removes publication flags without deleting authorship history. Do not restore login/logout, signup, password settings, user/member management, project ownership transfer, or public sharing. The original schema and internal compatibility tests may retain historical identities; they must not expose multi-user product flows.
2. **Local entry points only.** The server and development servers reject LAN/wildcard bindings. `src/personal_cli.rs` is the only executable command registry. Preserve agent access through direct SQLite CLI, loopback HTTP CLI, stdio MCP, and loopback HTTP MCP. These require no human login; optional per-tool tokens provide attribution and revocation. Remote destinations, multi-instance MCP, OAuth, and live provider imports remain unavailable. Keep `src/personal.rs` enforcing these restrictions on the production router and every startup. Keep Host, Origin, forwarded-header, and cross-site request checks in login-free mode.
3. **No external API traffic or unapproved resources.** Do not restore remote imports, telemetry, analytics, update checks, crash reporting, remote sync, or arbitrary remote assets. The browser's Google Fonts exception is specified below. Backend HTTP clients must still validate destinations, ignore proxies, disable redirects, and pin `localhost` to loopback.
4. **Stored content has no automatic external loads.** HTTP(S) source links may open in a new tab when clicked, with `noopener noreferrer` and no referrer. Do not fetch previews, call provider APIs, or pass embedded credentials. Preserve embed filtering, Markdown/diagram sanitization, and attachment sandboxing. Forms stay local; do not restore OAuth consent or callback flows or broadly relax CSP to fix a broken feature.
5. **Data stays on approved local storage.** Databases, attachments, exports, archives, and backups must not be placed in cloud-synced folders. Never use corporate data or credentials in tests or public reports.
6. **Updates are deliberate.** Repository CI may run on pushes/PRs and dependency checks on a schedule; fork releases may run after a deliberate version tag. These are development operations, never application startup behavior. Publication under the upstream crate/MCP registry identities remains disabled, and upstream binaries/installers must not replace our build. Git fetches and dependency preparation are separate development operations that can use the network; they are not part of application startup.

The application cannot control an MCP client's cloud provider, browser extensions, other local processes, or OS syncing. This contract is an application boundary, not a guarantee that the laptop itself has no external traffic.

### Retained local features — October 19, 2026

The owner approved restoring local agent setup, localhost HTTP CLI/MCP, workspace naming, provider JSON file imports, local audio/video attachments, click-only help/source links, and automatic repository workflows. These supersede the earlier stdio-only and manual-workflow restrictions. Keep human accounts and remote connections unavailable. Protect `src/api/local_workspace.rs`, `src/import/file.rs`, and `e2e/local-agents.ts` alongside the existing policy modules. File imports must never follow provider or attachment URLs, and previews must not create issues or tool identities. Raw Markdown media must use same-origin attachment URLs only, with no autoplay.

### Accepted browser exceptions — October 19, 2026

The owner accepted Google Fonts and source links opened by clicking. These exceptions supersede the initial blanket ban on external fonts and navigation, and must survive upstream merges:

- The fixed stylesheet request at `https://fonts.googleapis.com/css2` loads Space Grotesk and DM Sans. Font files may load from `https://fonts.gstatic.com/s/`. These are stylesheet/font permissions only: do not add these hosts to `connect-src`, `script-src`, `img-src`, or the backend network allowlist. Never add project text, source URLs, or dynamic `text=` values to font requests. System fonts remain the fallback if Google is unavailable.
- Google receives network metadata such as the client's IP address and browser request headers. This browser exception means the app is not strictly offline. The page uses `no-referrer`; the application's font request contains only the fixed font selection.
- Ordinary HTTP(S) source links, including GitHub, Linear, Jira, and self-hosted sources, can be stored in Markdown and opened by the user. Opening a link contacts the destination using the browser's normal session. Rendering or hovering a source link must not contact it. Live provider imports and API integrations stay disabled.

## Repository roles

| Name | Repository / purpose |
| --- | --- |
| `origin` | `git@github.com:raphaelmans/lific` — our fork; reviewed changes go here |
| `upstream` | `https://github.com/VoidNullable/lific.git` — original project; fetch only |
| `main` | Our accepted fork history |
| `master` | Previous fork branch; retained at the original hardening baseline |
| `sync/upstream-YYYYMMDD` | Temporary review branch for one pinned upstream revision |

The first hardening baseline was committed as `ea97dc0`. **Commit all subsequent reviewed fork changes, including new files and tests, before starting an upstream update.** A worktree created from `main` does not inherit uncommitted changes. Review `git status --short`; stage only intended source and documentation, never local data or secrets.

## Update pipeline

### 1. Establish the baseline and fetch upstream

Run these steps on an approved development machine. Start with a clean, committed fork checkout. Use a unique branch/worktree name for each update.

Configure the upstream remote once, after checking existing remotes:

```bash
git remote -v
git remote add upstream https://github.com/VoidNullable/lific.git
git remote set-url --push upstream disabled://upstream
```

If `upstream` already exists, verify its URL instead of adding it again. The disabled push URL helps prevent accidental pushes to the original project. Do not change `origin` to the original repository.

Then pin the update and create an isolated checkout:

```bash
git switch main
git status --short                    # must be empty
fork_base=$(git rev-parse HEAD)
git fetch --no-tags upstream master
upstream_sha=$(git rev-parse upstream/master)
git log --oneline "$fork_base..$upstream_sha"
git diff --stat "$fork_base...$upstream_sha"
git worktree add -b sync/upstream-YYYYMMDD ../lific-upstream-review "$fork_base"
cd ../lific-upstream-review
git merge --no-ff --no-commit "$upstream_sha"
```

These commands assume the original branch is still `master`; verify that if a fetch fails. Keep `fork_base` and `upstream_sha` for the review record. Inspect incoming build scripts, hooks, dependencies, and workflow changes before executing them. Do not install an upstream release or run its installer as an update shortcut.

### 2. Merge features while retaining the fork contract

Resolve conflicts individually. A conflict-free merge still requires review: a new HTTP client or workflow can arrive in an untouched file.

Retain our mandatory guide and agent entry point from the recorded baseline:

```bash
git restore --source="$fork_base" --staged --worktree -- MUST_README.md AGENTS.md
git diff --name-only --diff-filter=U   # must be empty after all conflicts are resolved
git diff "$fork_base" --stat
git diff "$fork_base" -- .github .gitignore Cargo.toml Cargo.lock web/package.json web/bun.lock e2e/package.json e2e/bun.lock
```

Deliberate updates to this guide can follow that restore. Keep the fork-specific README, SECURITY instructions, regression tests, and root `AGENTS.md` tracking exception. Do not use a blanket `ours`/`theirs` merge strategy for application files: it can discard upstream fixes or the local-only restrictions. A `.gitattributes` merge rule is not a security gate and does not preserve behavior in other files.

Review these areas, including any new equivalents introduced by a refactor:

| Requirement | Current implementation / regression coverage |
| --- | --- |
| Local destinations, proxy/redirect refusal | `src/network.rs`, `src/cli/http.rs`, `src/cli/login.rs`, `src/cli/doctor.rs`, `src/cli/connect/`, `src/cli/mcp_instances.rs`, `src/cli/mcp_proxy.rs`, `tests/local_only.rs` |
| Listener, request admission, browser headers | `src/server.rs`, `src/config.rs`, `src/main.rs` |
| Personal CLI, login-free startup, removed account/remote routes | `src/personal_cli.rs`, `src/personal.rs`, `src/main.rs`, `src/server.rs` |
| File-only imports; disabled live transports | `src/import/{file,github,jira,linear}.rs`, `src/api/local_workspace.rs`, `src/main.rs`, `web/src/lib/OfflineImport.svelte` |
| Approved fonts, click-only source links, sanitized embeds | `web/index.html`, `web/public/appearance.js`, `web/src/app.css`, `web/src/lib/{Layout,PublicLayout,Markdown}.svelte`, `web/vite.config.ts` |
| Login-free initialization and browser entry | `src/config.rs`, init handling/tests in `src/main.rs`, `web/src/App.svelte`, `README.md` |
| Deliberate fork releases; no upstream registry publication | Every file in `.github/workflows/`, `scripts/verify-release-binary.ps1`, `Cargo.toml` (`publish = false`) |
| Browser security regressions | `e2e/local-only.ts` |

Search the complete candidate for new network behavior, not just the table's files:

```bash
rg -n 'https?://|wss?://|reqwest|fetch\(|WebSocket|EventSource|sendBeacon|TcpStream|UdpSocket|openExternal' src web .github scripts Cargo.toml
```

This is a review aid, not a complete scanner. Distinguish inert documentation/test URLs from executable behavior. Inspect new SDKs, subprocesses, package lifecycle scripts, build tooling, and generated assets too. Keep regression assertions that reject remote behavior; adapt them when interfaces change instead of deleting them to make the merge pass.

Stage only reviewed resolutions and additions, review the complete candidate against `fork_base`, and commit the candidate merge on the review branch. Record the full upstream SHA in the merge message. This gives the build and test results a concrete commit to refer to.

### 3. Run the acceptance checks locally

Prepare the reviewed, locked dependencies on an approved machine first: `bun install --frozen-lockfile --ignore-scripts` in both `web/` and `e2e/`, and `cargo fetch --locked`. These preparation commands can contact package registries. A new worktree does not include the original checkout's ignored dependencies or build output. Use an existing approved Chromium installation or a preinstalled Playwright browser; do not download one during the offline check phase.

Run the following as one Bash session, stopping on the first failure. Make the approved Rust toolchain and Bun available on `PATH` first. On this Mac, `mise exec rust@stable -- bash` provides the installed Rust toolchain.

```bash
set -euo pipefail
# macOS: avoid symlinked temporary paths rejected by SQLite NOFOLLOW.
if [ "$(uname -s)" = Darwin ]; then export TMPDIR=/private/tmp; fi

(cd web && bun run check && bun test && bun run build)
touch web/dist/.gitkeep
cargo fmt --all --check
cargo clippy --locked --offline --all-targets -- -D warnings
cargo test --locked --offline --all-targets
cargo build --release --locked --offline

export LIFIC_BIN="$PWD/target/release/lific"
# If using an installed browser, set its actual executable path, for example:
# export PLAYWRIGHT_EXECUTABLE_PATH='/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
(cd e2e && bun run local-only && bun run smoke)
git diff --exit-code
git status --short                    # investigate any generated/untracked changes
```

`--offline` constrains Cargo's dependency resolution, not arbitrary code in build scripts or tests. Review those inputs and use the organization's network controls when an offline build must be enforced.

`e2e/local-only.ts` starts a disposable personal workspace without credentials. It verifies removed human-account/sharing surfaces, login-free project creation and reload, local realtime/diagrams/media, hostile content, and the approved font/link exceptions. Its `local-agents.ts` checks exercise real SQLite/HTTP CLI and stdio/HTTP MCP calls, named tool setup/revocation, workspace naming, and offline file preview/import/deduplication. Google Fonts and clicked source destinations use local test responses; the test also checks offline fallback.

Finally, use Computer Use on a **separate temporary login-free instance** with the candidate release binary. Confirm direct dashboard entry without credentials, issue creation/editing/comments/status changes, pages, board navigation, and persistence after reload. Confirm Settings and project creation contain no human-account, membership, public-sharing, or remote-destination controls. Confirm local tool setup supports stdio and localhost HTTP, workspace naming persists, and file import preview works. Check local diagrams, Google Fonts and fallback, click-only source links, and external-embed blocking. Inspect browser network activity against the accepted exceptions above. Record the browser used and any blocked/incomplete steps. An unavailable browser or skipped test is not a passing test.

For database changes, test migration and backup restoration on a disposable copy. Keep an untouched backup for rollback; an older binary may not understand the migrated database.

### 4. Review and promote the tested candidate

The acceptance record must include the fork baseline SHA, upstream SHA, candidate merge SHA, dependency/migration review, retained restrictions, test outcomes, browser results, and the release binary's SHA-256. Re-run affected checks after any fixes; test the final candidate commit before accepting it.

Only after review and all required checks pass, return to the clean main checkout:

```bash
git switch main
git merge --ff-only sync/upstream-YYYYMMDD
```

If `main` has moved, integrate those changes into the review branch and validate the resulting candidate again. Do not force the promotion. Push only to `origin` when publishing the reviewed source is intended; do not push automatically as part of testing or use hosted CI as the only gate.

Before installing, stop the old instance and retain its binary plus a consistent local backup of its database, attachments, and configuration. Install the exact tested artifact, record its checksum, then verify login-free startup and the loopback listener. Keep fork release labels distinct from upstream versions. For rollback, restore compatible data together with the previous binary; do not run an older binary blindly against a migrated database.

If the merge cannot preserve these requirements, abort it in the review worktree with `git merge --abort` before the merge is committed, or leave the committed candidate unpromoted. Our accepted fork continues running. Small, reviewed cherry-picks are an alternative when the full update is unsuitable; they require the same checks.

## Enforcement status

This file defines the review and test process. `AGENTS.md` directs coding agents here, and existing tests check important parts of the contract. No scheduled sync, automatic merge, GitHub branch protection, push hook, or mandatory hosted check is configured by this guide. Those would be separate changes; documentation alone cannot prevent a person from bypassing the process.
