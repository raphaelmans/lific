# Local-only security boundary

For upstream updates, follow the review and acceptance process in [MUST_README.md](MUST_README.md).

This fork enforces local destinations in application code. It is not an operating-system sandbox and does not certify compliance with a corporate policy.

## Enforced behavior

The server defaults to `127.0.0.1` and refuses any non-loopback bind, even with authentication enabled or a CLI override. External public URLs, remote CORS origins, trusted proxies, and authless path tokens are rejected. Host and Origin checks prevent a remote website or DNS-rebinding name from reaching local APIs. Forwarded requests are rejected.

Every production HTTP client uses the policy in `src/network.rs`: destinations are validated as loopback, localhost resolution is pinned, redirects are disabled, and environment/system proxies are ignored. External GitHub, Linear, and Jira transport implementations have been removed. The CLI supports direct SQLite and loopback HTTP; MCP supports local stdio and loopback HTTP. Remote/multi-instance MCP and OAuth remain unavailable. Browser access and local agent access require no human login. Optional tool tokens are limited to the primary operator and their bots; historical human credentials and OAuth tokens are refused. Invalid or revoked supplied tokens never fall back to anonymous operator access.

The browser's Content Security Policy restricts scripts and API connections to its own origin. The owner-approved Google Fonts exception permits stylesheet loads from `https://fonts.googleapis.com/css2` and font files from `https://fonts.gstatic.com/s/`. It does not grant these domains script, fetch, image, or backend access. The stylesheet URL contains a fixed selection of Space Grotesk and DM Sans; never append project text or dynamic query values. System fonts provide an offline fallback. Google receives request metadata, including the browser's IP address, so this exception is not strictly offline.

Forms stay on the same origin. OAuth consent pages are unavailable. Dedicated attachment viewers may use local data/blob URLs. Raw Markdown audio/video/source/track elements may reference only same-origin `/api/attachments/<id>` URLs, without query strings or fragments; autoplay is removed and preload is disabled. Markdown permits HTTP(S) source links to open only when clicked, in a new tab with `noopener noreferrer` and `referrerpolicy="no-referrer"`. Credential-bearing URLs, unsafe schemes, tracking attributes, and active external embeds are stripped. Links do not cause provider imports or background previews; opening one uses the browser's normal connection to that website. Generated diagrams remain sanitized against external resources and links. External referrers and DNS prefetch are disabled. Uploaded active documents retain their stronger sandbox policy.

There is no configurable switch to re-enable remote server/API connections. Old remote configuration fails closed. Personal mode cannot be disabled by old authentication settings. Only the local operator can hold a browser session; login/signup, user/member management, password changes, and publication are unavailable. Historical author records remain in SQLite. Named local tools can be created and disconnected from a browser session; tool tokens cannot mint more tool tokens. A process that already has local access can still act without a token, so disconnecting a tool revokes its identity rather than sandboxing the process. File imports accept bounded local JSON only, never fetch referenced URLs, and preserve embedded content/source IDs without creating external human accounts. Existing file-permission protections remain in place.

## Boundaries outside the application

- MCP clients receive the data they request. A client using a cloud model can transmit it independently of Lific. Connect only an approved client/model.
- SQLite data, attachments, exports, archives, and backups are local files. OS backups or synced folders can copy them elsewhere; choose an approved local location.
- A local process with access to the account can read files or forward the loopback listener. Browser extensions and endpoint-management software have their own permissions. OS firewall/egress policy is the appropriate independent control when required.
- Building/installing dependencies uses registries unless all dependencies are cached. The running binary has no package-install or update mechanism. Upstream marketing/deployment source is separate from the runtime.
- Repository Git remotes are metadata, not runtime connections. Git operations and manually dispatched hosted workflows have their own network activity. CI and dependency checks run on the configured repository events; fork release builds require a deliberate version tag or manual dispatch. Upstream crate/MCP registry publication remains disabled. Pushes and deployments require the owner’s instructions.

## Verification

```bash
cargo test --locked --offline
cargo clippy --locked --offline --all-targets -- -D warnings
cd web
bun run check
bun run build
cd ../e2e
bun run local-only
```

The security regressions exercise loopback URL admission, removed remote/account commands and routes, legacy-settings normalization, local health-check proxy bypass, redirect refusal, hostile Host/Origin headers, and browser content attempting remote requests. The browser test intercepts Google Fonts with local stylesheet/font fixtures and intercepts clicked source navigation, verifies those narrow exceptions and offline fallback, and rejects other external requests. It uses a temporary instance and tears it down. It must never use real corporate data.

For a corporate rollout, review the resulting binary and its dependencies through the organization's normal software process. Report findings internally without including real tracker data or credentials in public issues.
