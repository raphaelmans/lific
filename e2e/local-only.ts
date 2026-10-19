/** Security regression: use a disposable instance, never real tracker data. */
import assert from "node:assert/strict";
import { spawn, execFileSync, type ChildProcess } from "node:child_process";
import { mkdtempSync, realpathSync, rmSync } from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { chromium, type Browser } from "playwright";

const binary = process.env.LIFIC_BIN ?? resolve(import.meta.dir, "../target/debug/lific");
const scratch = realpathSync(mkdtempSync(join(tmpdir(), "lific-local-only-")));
const config = join(scratch, "lific.toml");
const database = join(scratch, "lific.db");
const password = "local-regression-password";
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

try {
  cli(["init", "--no-service", "--name", "Local Operator", "--auth-mode", "passwords", "--password", password, "--json"]);
  cli(["project", "create", "--name", "Local regression", "--identifier", "LOCAL", "--json"]);
  cli(["issue", "create", "--project", "LOCAL", "--title", "Local policy regression", "--description", [
    "[External destination](https://blocked.invalid/leak)",
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
  assert.equal(registration.status, 400, "external OAuth callback must be refused");

  browser = await chromium.launch({ executablePath: process.env.PLAYWRIGHT_EXECUTABLE_PATH });
  const context = await browser.newContext();
  const externalRequests: string[] = [];
  await context.route("**/*", route => {
    const url = route.request().url();
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
  const response = await page.goto(`${base}/login`);
  const policy = response!.headers()["content-security-policy"];
  assert(policy.includes("connect-src 'self'"));
  assert(policy.includes("script-src 'self';"));
  await page.fill("#login-identity", "local-operator");
  await page.fill("#login-password", password);
  await page.click("button[type=submit]");
  await page.waitForURL(url => url.origin === base && url.pathname === "/" && ["", "#/"].includes(url.hash));
  await page.waitForFunction(() => !!localStorage.getItem("lific_token"));
  assert.equal(await page.evaluate(async () => (await fetch("/api/auth/me", { headers: { Authorization: `Bearer ${localStorage.getItem("lific_token")}` } })).status), 200);
  await page.goto(`${base}/LOCAL/issues/LOCAL-1`);
  await page.waitForFunction(() => document.body.innerText.includes("Local policy regression"), undefined, { timeout: 10_000 }).catch(async error => {
    throw new Error(`${error}\n${await page.locator("body").innerText()}`);
  });
  await page.locator('.mermaid-block[data-rendered="true"]').waitFor();
  assert.equal(await page.locator('[href*="blocked.invalid"], [src*="blocked.invalid"], [srcset*="blocked.invalid"], [action*="blocked.invalid"]').count(), 0);
  assert(await page.locator('a[href="#/LOCAL/issues/LOCAL-1"]').count() > 0);
  assert(sockets.length > 0, "local realtime socket should still work");
  assert(sockets.every(url => url.startsWith(base.replace("http:", "ws:"))));
  assert.equal(await page.evaluate(async () => {
    try { await fetch("https://blocked.invalid/from-script"); return false; }
    catch { return true; }
  }), true, "CSP must block script-initiated remote requests");

  // A native client on another loopback port must still receive its OAuth
  // callback. Intercept it locally; no other service needs to be running.
  const callback = `http://127.0.0.1:${await freePort()}/callback`;
  let callbackReceived = false;
  await context.route(`${callback}*`, async route => {
    const url = new URL(route.request().url());
    assert(url.searchParams.get("code"));
    assert.equal(url.searchParams.get("state"), "local-regression");
    callbackReceived = true;
    await route.fulfill({ status: 200, contentType: "text/plain", body: "Local callback received" });
  });
  const clientResponse = await fetch(`${base}/oauth/register`, {
    method: "POST", headers: { "content-type": "application/json" },
    body: JSON.stringify({ client_name: "Local test client", redirect_uris: [callback] }),
  });
  assert(clientResponse.ok);
  const client = await clientResponse.json() as { client_id: string };
  const params = new URLSearchParams({
    client_id: client.client_id, redirect_uri: callback, response_type: "code",
    scope: "mcp", code_challenge: "a".repeat(43), code_challenge_method: "S256", state: "local-regression",
  });
  const consent = await page.goto(`${base}/oauth/authorize?${params}`);
  assert.equal(consent!.headers()["referrer-policy"], "same-origin");
  await page.selectOption("#tool", "__custom__");
  await page.locator("#tool_custom").waitFor({ state: "visible" });
  await page.fill("#tool_custom", "local-security-test");
  const approval = page.waitForResponse(response => new URL(response.url()).pathname === "/oauth/authorize" && response.request().method() === "POST");
  await page.getByRole("button", { name: "Approve", exact: true }).click();
  const approvalResponse = await approval;
  assert.equal(approvalResponse.status(), 303, "local OAuth approval must redirect");
  await page.waitForURL(url => url.origin === new URL(callback).origin && url.pathname === "/callback", { timeout: 5000 }).catch(async error => {
    throw new Error(`${error}\n${await page.locator("body").innerText()}`);
  });
  assert(callbackReceived, "local OAuth form redirect must remain usable under CSP");
  assert.deepEqual(externalRequests, [], "no unexpected external browser requests");
  assert.deepEqual(errors, []);
  console.log("PASS: local login, issue rendering, diagram, realtime, hostile content, CSP, Host/Origin/proxy checks, and OAuth callbacks");
} catch (error) {
  console.error(serverLog);
  throw error;
} finally {
  await browser?.close();
  if (server && server.exitCode === null) {
    const stopped = new Promise<void>(resolve => server!.once("exit", () => resolve()));
    server.kill("SIGTERM");
    await stopped;
  }
  rmSync(scratch, { recursive: true, force: true });
}
