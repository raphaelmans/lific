# Lific — local-only fork

A private, SQLite-backed issue tracker for use on one laptop. The web UI is embedded in the binary. Server/API connections are loopback-only; external imports are disabled. The browser may load Google Fonts and open source links when clicked.

Before changing this fork or bringing in upstream updates, read [MUST_README.md](MUST_README.md) for the fork requirements, merge pipeline, and acceptance checks.

## Build and run this fork

Build from this checkout. `cargo install lific` and upstream release downloads install the original application, which does **not** contain these restrictions.

```bash
cd web
bun install --frozen-lockfile --ignore-scripts
bun run build
cd ..
cargo build --release --locked
./target/release/lific init --no-service
./target/release/lific start
```

Open http://127.0.0.1:3456. Login-free mode opens the tracker directly as your local operator, without a login screen. `init --no-service` avoids installing a background service. Add `--here` to keep the config and database beside the checkout instead of the normal user data directory.

Dependency installation and compilation can download packages and toolchains. Prepare the binary on an approved build machine if the corporate laptop must remain offline. After preparing `web/node_modules` and the Cargo dependency cache, `bun run build` and `cargo build --release --locked --offline` can build without downloading dependencies.

## Local-only behavior

- The HTTP server binds only to loopback. LAN/wildcard bind addresses, external public URLs, trusted proxies, and authless MCP path tokens are rejected.
- One personal workspace opens without login. Login/logout, account creation, member management, public sharing, remote servers, multi-instance MCP, and OAuth are unavailable. Agents can use the direct SQLite CLI, loopback HTTP CLI, stdio MCP, or loopback HTTP MCP without human login.
- GitHub, Linear, and Jira live import transports are removed. The live import commands and UI are removed. Local project archives, provider JSON file imports, and click-only source links remain supported.
- Google Fonts supplies Space Grotesk and DM Sans through a narrowly scoped browser exception; system fonts are the offline fallback. Other assets are local. External API calls, remote embeds, and external form submission remain blocked.
- HTTP(S) source links in Markdown open in a separate tab when clicked, without a referrer or opener access. They do not trigger automatic previews or provider API imports. External image and diagram resources remain blocked.
- External browser origins, nonlocal Host headers, and forwarded requests are rejected. CORS is closed by default. HTTP MCP is served at `/mcp` on loopback; OAuth endpoints return 404.
- No telemetry, automatic update check, or crash-upload service is configured. GitHub CI runs on pushes and pull requests, with scheduled dependency checks and releases on deliberate version tags. These repository jobs do not run on app startup. Publication under the upstream crate and MCP registry identities stays disabled.

Existing upstream configurations using remote settings must be edited before this fork will run. The local login-free settings are:

```toml
[server]
host = "127.0.0.1"
port = 3456
cors_origins = []
trusted_proxies = []

[auth]
required = false
allow_signup = false
```

Keep the existing `[database]` and other sections when migrating a config. Omit `public_url`, `mcp_path_token`, and `mcp_path_user`.

Google Fonts requests expose network metadata such as your IP address and browser request headers to Google. The font URL is fixed and contains no project text. This accepted exception is documented in [MUST_README.md](MUST_README.md); the browser is no longer strictly offline.

An existing local database is normalized on startup: login-free access stays on, signup and publication stay off, and sessions and tool keys belonging to other historical users expire. The earliest active administrator remains the local operator. Existing author records and project content are preserved. There is no setting to turn this fork back into a multi-user service.

## MCP and corporate data

The direct stdio transport needs no HTTP listener:

```bash
lific mcp
# Configure an installed client for local stdio:
lific connect --client codex --yes
# Or use the running localhost HTTP MCP endpoint:
lific connect --client codex --http --url http://127.0.0.1:3456 --yes
# A stdio-only client can proxy to that same local endpoint:
lific mcp --http --url http://127.0.0.1:3456
```

MCP gives the connected client access to tracker data. A cloud AI client can send that data to its own provider even though Lific's API is restricted to local connections. Use an approved local client/model when data must remain on the laptop. Lific cannot control browser extensions, other applications, filesystem sync, or OS-level forwarding. See [SECURITY.md](SECURITY.md) for the boundary and verification steps.

Local MCP supports issue tracking, pages, plans, comments, search, attachments, and exports. HTTP clients can connect directly to `http://127.0.0.1:3456/mcp`. Neither transport requires a human login. Optional tool tokens label local history; they are not additional human logins. Settings → Preferences → Local agent tools creates named connections, generates local client configuration, and disconnects individual tools. Invalid or revoked tokens are rejected, while a client configured without a token acts as the local operator. Token revocation controls that identity; it cannot exclude a process that already has local access.

## MCP tools

| Family | Tools |
| --- | --- |
| Issues | `list_issues` · `get_issue` · `create_issue` · `update_issue` · `bulk_update` · `edit_issue` · `get_board` |
| Relations and waits | `link_issues` · `unlink_issues` |
| Pages | `get_page` · `create_page` · `update_page` · `edit_page` |
| Plans | `create_plan` · `get_plan` · `edit_plan_step` · `update_plan_step` |
| Comments | `add_comment` · `list_comments` · `edit_comment` · `delete_comment` |
| Attachments | `upload_attachment` · `get_attachment` · `list_attachments` |
| Search and history | `get_briefing` · `search` · `get_activity` |
| Structure | `list_resources` · `manage_resource` · `delete` |
| Export | `export` |

## CLI

```bash
lific issue list --project APP
lific issue create --project APP --title "Fix search bug" --priority high
lific issue update APP-42 --status done
lific search "search" --project APP
lific --backend http --url http://127.0.0.1:3456 issue list --project APP
```

The CLI reads the local database directly by default, or uses the loopback API with `--backend http`. Supply `--api-key` or `LIFIC_TOKEN` for named-tool attribution. Remote URLs are rejected before any request. Output is JSON when piped. `lific agents-md` adds a short Lific section to a repo's `AGENTS.md`. Use `lific --help` and subcommand help for this fork's supported commands; upstream CLI documentation describes features this fork does not expose.

## Local customization and file imports

Settings → Preferences lets you name the workspace shown on Home. The help/source links open only when clicked. Markdown can play local attachments with `<audio controls src="/api/attachments/123"></audio>` or the equivalent `<video>` element. Only attachment URLs on this app are accepted; autoplay is removed and preload is disabled. Existing attachment viewers remain available.

Project Overview → Import issues from a local file previews a saved provider JSON file before writing. The CLI supports the same pipeline:

```bash
lific import-file issues.json --source github --namespace owner/repo --project APP --dry-run
lific import-file issues.json --source github --namespace owner/repo --project APP
lific import-file linear.json --source linear --project APP
lific import-file jira.json --source jira --namespace company --project APP
```

Files are limited to 4 MiB and 10,000 issues. Supported shapes are provider issue arrays, `{ "issues": [...] }`, and Linear `{ "data": { "issues": { "nodes": [...] } } }`. GitHub uses its REST issue fields (`number`, `title`, `state`, `body`, `labels`); a minimal file is:

```json
[{"number":12,"title":"Saved issue","state":"open","body":"Local content"}]
```

Linear uses issue `identifier`, `title`, `state.type`, and nested label/comment `nodes`. Jira uses `key` and `fields` (including `summary`, ADF `description`, and `comment.comments`). GitHub comments can be embedded in each issue's `comments` array. Only embedded comments are imported: URL fields never trigger downloads. CSV files need conversion to one of these JSON shapes first. External assignees are not converted into human accounts. Re-imports skip existing source IDs. Imports commit each issue independently; a failed batch can be retried without duplicating completed issues.

## Backups

Backups stay on disk. Keep the database, attachments, archives, and backups outside cloud-synced folders.

```bash
lific dump
lific restore path/to/backup.tar.gz  # stop the server first
```

The `site/`, `promo/`, and `deploy/` directories and historical documentation are upstream development material, not part of the embedded application. Their deployment instructions do not apply to this fork. Standard container port publishing is also unsupported because the process binds to loopback inside the container.

## License

Forked from Lific by VoidNullable. Original copyright and [Apache-2.0 license](LICENSE) are retained.
