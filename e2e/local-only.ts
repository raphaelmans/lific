/** Security regression: use a disposable instance, never real tracker data. */
import assert from "node:assert/strict";
import { spawn, execFileSync, type ChildProcess } from "node:child_process";
import { mkdtempSync, readFileSync, realpathSync, rmSync } from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, type Browser } from "playwright";
import { checkLocalAgents } from "./local-agents";

const binary = process.env.LIFIC_BIN ?? resolve(import.meta.dir, "../target/debug/lific");
const scratch = realpathSync(mkdtempSync(join(tmpdir(), "lific-local-only-")));
const config = join(scratch, "lific.toml");
const database = join(scratch, "lific.db");
const fontStylesheet = "https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;500;600;700&family=DM+Sans:ital,opsz,wght@0,9..40,300..700;1,9..40,300..700&display=swap";
const fontFile = "https://fonts.gstatic.com/s/lific-test.woff2";
const sourceLinks = [
  ["GitHub source", "https://github.com/example/source/issues/123"],
  ["Linear source", "https://linear.app/example/issue/SRC-1"],
  ["Jira source", "https://example.atlassian.net/browse/SRC-1"],
] as const;
let server: ChildProcess | undefined;
let browser: Browser | undefined;
let serverLog = "";
const cli = (args: string[]) => execFileSync(binary, ["--config", config, "--db", database, ...args], { encoding: "utf8" });

async function freePort(): Promise<number> {
  const listener = createServer();
  return await new Promise((resolve, reject) => {
    listener.on("error", reject);
    listener.listen(0, "127.0.0.1", () => {
      const port = (listener.address() as { port: number }).port;
      listener.close(() => resolve(port));
    });
  });
}

async function main(): Promise<number> {
try {
  cli(["init", "--no-service", "--name", "Local Operator", "--json"]);
  cli(["project", "create", "--name", "Local regression", "--identifier", "LOCAL", "--json"]);
  cli(["issue", "create", "--project", "LOCAL", "--title", "Local policy regression", "--description", [
    ...sourceLinks.map(([name, url]) => `[${name}](${url})`),
    '<a href="https://github.com/example/source/issues/123" ping="https://blocked.invalid/ping" attributionsrc="https://blocked.invalid/attribution">Source without tracking</a>',
    '[Unsafe script](javascript:alert(1))',
    '[Credentials in URL](https://user:secret@blocked.invalid/)',
    '![Font domain image](https://fonts.gstatic.com/s/pixel.png)',
    "![Remote image](https://blocked.invalid/pixel)",
    '<img src="//blocked.invalid/pixel2" srcset="https://blocked.invalid/pixel3 2x">',
    '<style>@import "https://blocked.invalid/style.css";</style>',
    '<iframe src="https://blocked.invalid/embed"></iframe>',
    '<form action="https://blocked.invalid/submit"><input name="secret"></form>',
    '[Local issue](#/LOCAL/issues/LOCAL-1)',
    '```mermaid\ngraph LR\n  A[Local] --> B[Only]\n```',
  ].join("\n\n"), "--json"]);
  const base = `http://127.0.0.1:${await freePort()}`;
  server = spawn(binary, ["--config", config, "start", "--port", new URL(base).port], { stdio: ["ignore", "pipe", "pipe"] });
  server.stdout?.on("data", chunk => { serverLog += chunk; });
  server.stderr?.on("data", chunk => { serverLog += chunk; });
  const deadline = Date.now() + 20_000;
  while (true) {
    try { if ((await fetch(`${base}/api/health`)).ok) break; } catch {}
    assert(Date.now() < deadline, serverLog);
    await new Promise(resolve => setTimeout(resolve, 100));
  }

  for (const headers of [
    { Host: "rebinding.invalid" },
    { Origin: "https://blocked.invalid" },
    { Origin: "null" },
    { "X-Forwarded-For": "127.0.0.1" },
    { Forwarded: "for=127.0.0.1" },
    { "Sec-Fetch-Site": "cross-site" },
  ]) {
    assert.equal((await fetch(`${base}/api/health`, { headers })).status, 403, JSON.stringify(headers));
  }
  const registration = await fetch(`${base}/oauth/register`, {
    method: "POST", headers: { "content-type": "application/json" },
    body: JSON.stringify({ client_name: "local regression", redirect_uris: ["https://blocked.invalid/callback"] }),
  });
  assert.equal(registration.status, 404, "OAuth registration is unavailable");

  browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  const context = await browser.newContext();
  context.setDefaultTimeout(10_000);
  const externalRequests: string[] = [];
  const fontRequests: string[] = [];
  let fontsAvailable = true;
  let sourceClicked = false;
  let sourceNavigations = 0;
  // Fulfill the Google endpoints locally so this regression never needs the
  // internet. A font already shipped by KaTeX supplies valid WOFF2 bytes.
  const fontBytes = readFileSync(resolve(import.meta.dir, "../web/node_modules/katex/dist/fonts/KaTeX_Main-Regular.woff2"));
  await context.route("**/*", route => {
    const request = route.request();
    const url = request.url();
    if ((url === fontStylesheet && request.resourceType() === "stylesheet")
      || (url === fontFile && request.resourceType() === "font")) {
      fontRequests.push(url);
      assert.equal(request.headers().referer, undefined, "font requests must not include a tracker referrer");
      assert.equal(request.headers().authorization, undefined);
      if (!fontsAvailable) return route.abort();
      if (url === fontStylesheet) {
        return route.fulfill({
          contentType: "text/css",
          body: ["DM Sans", "Space Grotesk"].map(family =>
            `@font-face { font-family: "${family}"; src: url("${fontFile}") format("woff2"); font-weight: 400; font-display: swap; }`,
          ).join("\n"),
          headers: { "referrer-policy": "no-referrer" },
        });
      }
      return route.fulfill({ contentType: "font/woff2", body: fontBytes, headers: { "access-control-allow-origin": "*" } });
    }
    if (sourceClicked && url === sourceLinks[0][1] && request.isNavigationRequest()) {
      sourceNavigations++;
      assert.equal(request.headers().referer, undefined, "source links must not include a tracker referrer");
      assert.equal(request.headers().authorization, undefined);
      return route.fulfill({ contentType: "text/plain", body: "External source opened by explicit click" });
    }
    if (new URL(url).origin !== base) {
      externalRequests.push(url);
      return route.abort();
    }
    return route.continue();
  });
  const page = await context.newPage();
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  const sockets: string[] = [];
  page.on("websocket", socket => sockets.push(socket.url()));
  const response = await page.goto(`${base}/`);
  const policy = response!.headers()["content-security-policy"];
  assert(policy.includes("connect-src 'self'"));
  assert(policy.includes("script-src 'self';"));
  assert(policy.includes("style-src 'self' 'unsafe-inline' https://fonts.googleapis.com/css2;"));
  assert(policy.includes("font-src 'self' https://fonts.gstatic.com/s/;"));
  assert.equal(await page.evaluate(async () => {
    await document.fonts.load('400 14px "DM Sans"');
    await document.fonts.load('400 14px "Space Grotesk"');
    return document.fonts.check('400 14px "DM Sans"') && document.fonts.check('400 14px "Space Grotesk"');
  }), true, "the narrow CSP exception must allow the requested fonts to load");
  assert(fontRequests.includes(fontStylesheet) && fontRequests.includes(fontFile));
  assert.equal(await page.locator('input[type="password"]').count(), 0);
  await page.waitForFunction(() => !!localStorage.getItem("lific_token"));
  assert.equal(await page.evaluate(async () => (await fetch("/api/auth/me", { headers: { Authorization: `Bearer ${localStorage.getItem("lific_token")}` } })).status), 200);
  await page.goto(`${base}/LOCAL/issues/LOCAL-1`);
  await page.waitForFunction(() => document.body.innerText.includes("Local policy regression"), undefined, { timeout: 10_000 }).catch(async error => {
    throw new Error(`${error}\n${await page.locator("body").innerText()}`);
  });
  await page.locator('.mermaid-block[data-rendered="true"]').waitFor();
  assert.equal(await page.locator('[href*="blocked.invalid"], [src*="blocked.invalid"], [srcset*="blocked.invalid"], [action*="blocked.invalid"], img[src*="fonts.gstatic.com"], [ping], [attributionsrc]').count(), 0);
  assert.equal(await page.getByText("Unsafe script", { exact: true }).getAttribute("href"), null);
  for (const [name, url] of sourceLinks) {
    const link = page.getByRole("link", { name, exact: true });
    assert.equal(await link.getAttribute("href"), url);
    assert.equal(await link.getAttribute("target"), "_blank");
    assert.equal(await link.getAttribute("rel"), "noopener noreferrer");
    assert.equal(await link.getAttribute("referrerpolicy"), "no-referrer");
  }
  assert.equal(sourceNavigations, 0, "rendering source links must not fetch their destinations");
  assert.deepEqual(externalRequests, [], "no source previews, tracking pixels, or unexpected external resources");
  assert(await page.locator('a[href="#/LOCAL/issues/LOCAL-1"]').count() > 0);
  assert(sockets.length > 0, "local realtime socket should still work");
  assert(sockets.every(url => url.startsWith(base.replace("http:", "ws:"))));
  for (const url of ["https://blocked.invalid/from-script", fontStylesheet, fontFile]) {
    assert.equal(await page.evaluate(async url => {
      try { await fetch(url); return false; }
      catch { return true; }
    }, url), true, "CSP must block script-initiated requests even to font endpoints");
  }

  sourceClicked = true;
  const sourceOpened = context.waitForEvent("page");
  await page.getByRole("link", { name: sourceLinks[0][0], exact: true }).click();
  const sourcePage = await sourceOpened;
  await sourcePage.waitForLoadState();
  assert.equal(sourcePage.url(), sourceLinks[0][1]);
  assert.equal(await sourcePage.evaluate(() => window.opener === null), true);
  assert.equal(sourceNavigations, 1);
  assert(page.url().includes("LOCAL-1"), "source navigation must leave the tracker tab open");
  await sourcePage.close();

  // Block the approved fonts to verify the app remains usable offline.
  fontsAvailable = false;
  const fallbackPage = await context.newPage();
  await fallbackPage.goto(`${base}/LOCAL/issues/LOCAL-1`);
  await fallbackPage.getByRole("button", { name: "Local policy regression", exact: true }).waitFor();
  await fallbackPage.close();
  fontsAvailable = true;

  // Removed surfaces must stay unavailable, even on loopback.
  for (const [path, method] of [
    ["/oauth/register", "POST"], ["/oauth/authorize", "GET"], ["/authorize", "GET"],
    ["/.well-known/oauth-authorization-server", "GET"],
    ["/api/auth/login", "POST"], ["/api/auth/signup", "POST"], ["/api/auth/logout", "POST"],
    ["/api/auth/me/password", "POST"], ["/api/auth/keys", "POST"],
    ["/api/users", "POST"], ["/api/users/1/promote", "POST"],
    ["/api/projects/1/members", "GET"], ["/api/projects/1/members", "POST"],
    ["/api/instance/settings", "PATCH"], ["/api/public/LOCAL/issues", "GET"],
    ["/api/projects/1/import/github", "POST"],
  ]) {
    assert.equal((await fetch(`${base}${path}`, { method })).status, 404, `${method} ${path}`);
  }
  for (const patch of [{ is_public: true }, { lead_user_id: 123 }]) {
    assert.equal((await fetch(`${base}/api/projects/1`, {
      method: "PUT", headers: { "content-type": "application/json" }, body: JSON.stringify(patch),
    })).status, 400, "sharing and ownership changes are disabled");
  }
  await page.goto(`${base}/#/settings`);
  await page.getByRole("heading", { name: "Personal workspace", exact: true }).waitFor();
  assert.equal(await page.getByText(/Sign out|Change password|Create account|Connect agent/).count(), 0);
  await page.goto(`${base}/#/settings/instance`);
  await page.getByRole("heading", { name: "Local storage", exact: true }).waitFor();
  assert.equal(await page.getByText(/Allow signups|Require login|Create user|Members/).count(), 0);
  await page.goto(`${base}/#/LOCAL/overview`);
  await page.getByRole("button", { name: "Local regression", exact: true }).waitFor();
  assert.equal(await page.getByText(/Publish issues|Public view|Project lead|Members/).count(), 0);
  await page.goto(`${base}/#/projects/new`);
  await page.locator("#project-name").fill("Personal UI test");
  await page.locator("#project-id").fill("PERS");
  assert.equal(await page.getByText("Lead", { exact: true }).count(), 0);
  await page.getByRole("button", { name: "Create project", exact: true }).click();
  await page.waitForURL(/#\/PERS\/overview/);
  await page.reload();
  await page.getByRole("button", { name: "Personal UI test", exact: true }).waitFor();
  assert.equal(await page.locator('input[type="password"]').count(), 0);
  await page.goto(`${base}/#/login`);
  await page.waitForURL(url => url.hash === "#/" || url.hash === "");
  assert.equal(await page.locator('input[type="password"]').count(), 0);
  assert.deepEqual(externalRequests, [], "no unexpected external browser requests");
  await checkLocalAgents(binary, config, database, base, scratch, page);
  assert.deepEqual(externalRequests, [], "restored features must not contact external services");
  assert.deepEqual(errors, []);
  console.log("PASS: approved fonts, offline fallback, click-only source links, login-free personal workflow, removed account/sharing/remote endpoints, diagram, realtime, hostile content, CSP, and Host/Origin/proxy checks");
  return 0;
} catch (error) {
  console.error(error);
  console.error(serverLog);
  return 1;
} finally {
  await Promise.race([browser?.close(), new Promise(resolve => setTimeout(resolve, 5000))]);
  if (server && server.exitCode === null && server.signalCode === null) {
    const stopped = new Promise<void>(resolve => server!.once("exit", () => resolve()));
    server.kill("SIGTERM");
    const forceStop = setTimeout(() => server?.kill("SIGKILL"), 5000);
    await stopped;
    clearTimeout(forceStop);
  }
  rmSync(scratch, { recursive: true, force: true });
}

}
process.exit(await main());
