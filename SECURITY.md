# Local-only security boundary

For upstream updates, follow the review and acceptance process in [MUST_README.md](MUST_README.md).

This fork enforces local destinations in application code. It is not an operating-system sandbox and does not certify compliance with a corporate policy.

## Enforced behavior

The server defaults to `127.0.0.1` and refuses any non-loopback bind, even with authentication enabled or a CLI override. External public URLs, remote CORS origins, trusted proxies, and authless path tokens are rejected. Host and Origin checks prevent a remote website or DNS-rebinding name from reaching local APIs. Forwarded requests are rejected.

Every production HTTP client uses the policy in `src/network.rs`: destinations are validated as loopback, localhost resolution is pinned, redirects are disabled, and environment/system proxies are ignored. External GitHub, Linear, and Jira transport implementations have been removed. OAuth callback URLs must also be local.

The browser receives a Content Security Policy restricting scripts, connections, and fonts to its own origin. Forms stay on the same origin, with one validated loopback callback permitted on OAuth consent pages. Images and media may also use local data/blob URLs. External fonts and upstream navigation links have been removed. Markdown blocks active embeds and external links; generated diagrams are sanitized too. External referrers and DNS prefetch are disabled. Uploaded active documents retain their stronger sandbox policy.

There is no configurable switch to re-enable remote connections. Old remote configuration fails closed. Authentication and existing file-permission protections remain in place.

## Boundaries outside the application

- MCP clients receive the data they request. A client using a cloud model can transmit it independently of Lific. Connect only an approved client/model.
- SQLite data, attachments, exports, archives, and backups are local files. OS backups or synced folders can copy them elsewhere; choose an approved local location.
- A local process with access to the account can read files or forward the loopback listener. Browser extensions and endpoint-management software have their own permissions. OS firewall/egress policy is the appropriate independent control when required.
- Building/installing dependencies uses registries unless all dependencies are cached. The running binary has no package-install or update mechanism. Upstream marketing/deployment source is separate from the runtime.
- Repository Git remotes are metadata, not runtime connections. Git operations and manually dispatched hosted workflows have their own network activity. No push or deployment is performed as part of hardening this checkout.

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

The security regressions exercise loopback URL admission, remote command/config refusal, proxy bypass, redirect refusal, hostile Host/Origin headers, OAuth callbacks, and browser content attempting remote requests. The browser test uses a temporary instance and tears it down. It must never use real corporate data.

For a corporate rollout, review the resulting binary and its dependencies through the organization's normal software process. Report findings internally without including real tracker data or credentials in public issues.
