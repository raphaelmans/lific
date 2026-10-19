# Read before changing or updating this fork

This is Raphael's **local-only, login-free fork** of Lific. Read this file and [SECURITY.md](SECURITY.md) before implementing features, updating dependencies, resolving conflicts, or importing upstream changes. Keep this file in the fork and update it deliberately when the implementation changes.

## The fork contract

An upstream update is acceptable only while these requirements remain true:

1. **Login-free local use.** The recommended setup uses `init --auth-mode login-free`. The browser opens as the local operator without asking for credentials. Preserve both `[auth] required = false` and the database's `web_auto_login` setting for that mode. Password-mode tests may remain; they must not change the local setup instructions back to requiring a login.
2. **Loopback access only.** The server and development servers reject LAN/wildcard bindings. CLI HTTP clients, MCP connections, and OAuth callbacks accept only loopback destinations. Keep Host, Origin, forwarded-header, and cross-site request checks even in login-free mode.
3. **No application traffic to external services.** Do not restore remote imports, telemetry, analytics, update checks, crash reporting, remote fonts/assets, or remote sync. HTTP clients must validate destinations, ignore proxies, disable redirects, and pin `localhost` to loopback.
4. **Browser content stays local.** Preserve CSP, external-link and embed filtering, Markdown/diagram sanitization, and attachment sandboxing. Keep the narrowly scoped local OAuth callback exception; do not broadly relax CSP to fix a broken feature.
5. **Data stays on approved local storage.** Databases, attachments, exports, archives, and backups must not be placed in cloud-synced folders. Never use corporate data or credentials in tests or public reports.
6. **Updates are deliberate.** Hosted workflows remain manual-only, Cargo publication remains disabled, and upstream binaries/installers do not replace our build. Git fetches and dependency preparation are separate development operations that can use the network; they are not part of application startup.

The application cannot control an MCP client's cloud provider, browser extensions, other local processes, or OS syncing. This contract is an application boundary, not a guarantee that the laptop itself has no external traffic.

## Repository roles

| Name | Repository / purpose |
| --- | --- |
| `origin` | `git@github.com:raphaelmans/lific` — our fork; reviewed changes go here |
| `upstream` | `https://github.com/VoidNullable/lific.git` — original project; fetch only |
| `master` | Our accepted fork history |
| `sync/upstream-YYYYMMDD` | Temporary review branch for one pinned upstream revision |

At the time this guide was introduced, only `origin` was configured and the hardening changes were still uncommitted. **Commit the reviewed hardening baseline, including its new files and tests, before starting an upstream update.** A worktree created from `master` does not inherit uncommitted changes. Review `git status --short`; stage only intended source and documentation, never local data or secrets.

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
git switch master
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
| Local OAuth and CSP-compatible consent | `src/oauth.rs`, `src/oauth-tool-picker.js` |
| Disabled external imports | `src/import/{github,jira,linear}.rs`, `src/main.rs`, `web/src/routes/ProjectSettings.svelte` |
| Local assets and sanitized content | `web/index.html`, `web/public/appearance.js`, `web/src/app.css`, `web/src/lib/{Layout,PublicLayout,Markdown}.svelte`, `web/vite.config.ts` |
| Login-free initialization and browser entry | `src/config.rs`, init handling/tests in `src/main.rs`, `web/src/App.svelte`, `README.md` |
| No automatic hosted jobs/publication | Every file in `.github/workflows/`, `Cargo.toml` (`publish = false`) |
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

The browser suites create temporary instances and use the exact release binary selected by `LIFIC_BIN`. The security suite deliberately tests password-mode authentication and local OAuth as well as external-request blocking; that does not change our login-free product setup.

Finally, use Computer Use on a **separate temporary login-free instance** with the candidate release binary. Confirm direct dashboard entry without credentials, issue creation/editing/comments/status changes, pages, board navigation, and persistence after reload. Check local diagrams and external-content blocking, and inspect browser network activity. Record the browser used and any blocked/incomplete steps. An unavailable browser or skipped test is not a passing test.

For database changes, test migration and backup restoration on a disposable copy. Keep an untouched backup for rollback; an older binary may not understand the migrated database.

### 4. Review and promote the tested candidate

The acceptance record must include the fork baseline SHA, upstream SHA, candidate merge SHA, dependency/migration review, retained restrictions, test outcomes, browser results, and the release binary's SHA-256. Re-run affected checks after any fixes; test the final candidate commit before accepting it.

Only after review and all required checks pass, return to the clean main checkout:

```bash
git switch master
git merge --ff-only sync/upstream-YYYYMMDD
```

If `master` has moved, integrate those changes into the review branch and validate the resulting candidate again. Do not force the promotion. Push only to `origin` when publishing the reviewed source is intended; do not push automatically as part of testing or use hosted CI as the only gate.

Before installing, stop the old instance and retain its binary plus a consistent local backup of its database, attachments, and configuration. Install the exact tested artifact, record its checksum, then verify login-free startup and the loopback listener. Keep fork release labels distinct from upstream versions. For rollback, restore compatible data together with the previous binary; do not run an older binary blindly against a migrated database.

If the merge cannot preserve these requirements, abort it in the review worktree with `git merge --abort` before the merge is committed, or leave the committed candidate unpromoted. Our accepted fork continues running. Small, reviewed cherry-picks are an alternative when the full update is unsuitable; they require the same checks.

## Enforcement status

This file defines the review and test process. `AGENTS.md` directs coding agents here, and existing tests check important parts of the contract. No scheduled sync, automatic merge, GitHub branch protection, push hook, or mandatory hosted check is configured by this guide. Those would be separate changes; documentation alone cannot prevent a person from bypassing the process.
