# Lific — local-only fork

A private, SQLite-backed issue tracker for use on one laptop. The web UI is embedded in the binary. This fork enforces loopback-only networking; remote server connections and external imports are disabled.

Before changing this fork or bringing in upstream updates, read [MUST_README.md](MUST_README.md) for the fork requirements, merge pipeline, and acceptance checks.

## Build and run this fork

Build from this checkout. `cargo install lific` and upstream release downloads install the original application, which does **not** contain these restrictions.

```bash
cd web
bun install --frozen-lockfile --ignore-scripts
bun run build
cd ..
cargo build --release --locked
./target/release/lific init --no-service --auth-mode login-free
./target/release/lific start
```

Open http://127.0.0.1:3456. Login-free mode opens the tracker directly as your local operator, without a login screen. `init --no-service` avoids installing a background service. Add `--here` to keep the config and database beside the checkout instead of the normal user data directory.

Dependency installation and compilation can download packages and toolchains. Prepare the binary on an approved build machine if the corporate laptop must remain offline. After preparing `web/node_modules` and the Cargo dependency cache, `bun run build` and `cargo build --release --locked --offline` can build without downloading dependencies.

## Local-only behavior

- The HTTP server binds only to loopback. LAN/wildcard bind addresses, external public URLs, trusted proxies, and authless MCP path tokens are rejected.
- HTTP CLI commands, login, diagnostics, MCP proxies, and multi-instance MCP can target only loopback addresses or `localhost`. HTTP clients ignore proxy settings and never follow redirects. `localhost` is pinned to loopback without DNS.
- GitHub, Linear, and Jira live import transports are removed. The import UI is removed; old commands fail with an explanation.
- Fonts and assets load locally. Browser policy blocks external requests, remote embeds, and external form submission. Markdown and diagram links cannot navigate to external destinations.
- External browser origins, nonlocal Host headers, and forwarded requests are rejected. CORS is closed by default. OAuth callbacks must use loopback URLs.
- No telemetry, automatic update check, or crash-upload service is configured. GitHub workflows require manual dispatch, and Cargo publication is disabled.

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

For an existing instance, also run `lific instance set --auto-login true --signups false` and restart it. Fresh `init --auth-mode login-free` instances already have browser auto-login enabled.

## MCP and corporate data

The direct stdio transport needs no HTTP listener:

```bash
lific connect --client <approved-client> --stdio
```

MCP gives the connected client access to tracker data. A cloud AI client can send that data to its own provider even though Lific itself is local-only. Use an approved local client/model when data must remain on the laptop. Lific cannot control browser extensions, other applications, filesystem sync, or OS-level forwarding. See [SECURITY.md](SECURITY.md) for the boundary and verification steps.

## What your agent can do

| Call | What it does |
| --- | --- |
| `get_briefing(project="APP")` | Resume a project: plans and next steps, blocked and workable issues, and with `since`, what changed |
| `list_issues(project="APP", workable=true)` | Open issues whose blockers are all resolved |
| `list_issues(members=["me"])` | Your active work across the projects you belong to |
| `create_plan` / `get_plan` | Nested step plans that survive the session; a step and its issue close together |
| `link_issues(target="APP-9", relation_type="blocks", user="ada")` | Block an issue on a person or a date window, not only another issue |
| `update_issue(assignees=["human"])` | Mark work a person must do; agents take unassigned issues |
| `update_issue(status="done", evidence="cargo test: 412 passed")` | Close with the proof saved as a verification comment |
| `edit_issue` / `edit_page` | Change one string without resending the document |
| `get_activity(identifier="APP", since="2026-10-01")` | Who changed what while you were away |

Each AI tool connects as its own bot under your account. The history says "OpenCode closed APP-42", and you can revoke one tool without touching the others.

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

The 5,781 tokens are the `tools/list` definitions as compact JSON, counted with tiktoken's o200k_base. Parameters and output: [MCP tools](https://lific.dev/docs/mcp/tools).

## CLI

```bash
lific issue list --project APP
lific issue create --project APP --title "Fix login bug" --priority high
lific issue update APP-42 --status done
lific search "authentication" --project APP
lific --backend http --url http://127.0.0.1:3456 issue list --project APP
```

The CLI reads the database directly, or the local server with `--backend http`. Remote servers are refused. Output is JSON when piped. `lific agents-md` adds a short Lific section to a repo's `AGENTS.md`. Full reference: [CLI](https://lific.dev/docs/cli).

## Backups

Backups stay on disk. Keep the database, attachments, archives, and backups outside cloud-synced folders.

```bash
lific dump
lific restore path/to/backup.tar.gz  # stop the server first
```

The `site/`, `promo/`, and `deploy/` directories and historical documentation are upstream development material, not part of the embedded application. Their deployment instructions do not apply to this fork. Standard container port publishing is also unsupported because the process binds to loopback inside the container.

## License

Forked from Lific by VoidNullable. Original copyright and [Apache-2.0 license](LICENSE) are retained.
