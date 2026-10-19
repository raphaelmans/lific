import assert from "node:assert/strict";
import { spawn, execFileSync, spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { Page } from "playwright";

export async function checkLocalAgents(binary: string, config: string, database: string, base: string, scratch: string, page: Page) {
  const env = { ...process.env };
  delete env.LIFIC_TOKEN; delete env.LIFIC_URL; delete env.LIFIC_API_KEY;
  const cli = (args: string[]) => JSON.parse(execFileSync(binary, ["--config", config, "--db", database, "--json", ...args], { encoding: "utf8", env }));
  const httpCli = (args: string[]) => cli(["--backend", "http", "--url", base, ...args]);
  const project = httpCli(["project", "create", "--name", "Local agent workflow", "--identifier", "AGNT"]);
  const cliIssue = httpCli(["issue", "create", "--project", "AGNT", "--title", "Created by localhost CLI"]);
  assert(cliIssue.identifier.startsWith("AGNT-"));
  assert.equal(cli(["issue", "get", cliIssue.identifier]).title, "Created by localhost CLI");

  const rpc = async (method: string, params: unknown = {}, token?: string) => {
    const response = await fetch(`${base}/mcp`, {
      method: "POST", headers: { "content-type": "application/json", accept: "application/json, text/event-stream", ...(token ? { authorization: `Bearer ${token}` } : {}) },
      body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
    });
    assert.equal(response.status, 200, await response.clone().text());
    const result = await response.json();
    assert(!result.error, JSON.stringify(result));
    return result.result;
  };
  const tools = await rpc("tools/list");
  assert(tools.tools.some((tool: { name: string }) => tool.name === "create_issue"));
  const created = await rpc("tools/call", { name: "create_issue", arguments: { project: "AGNT", title: "Created by localhost MCP" } });
  assert(JSON.stringify(created).includes("Created by localhost MCP"));
  const localIssues = cli(["issue", "list", "--project", "AGNT"]);
  const mcpIssue = localIssues.find((issue: { title: string }) => issue.title === "Created by localhost MCP");
  assert(mcpIssue);
  await rpc("tools/call", { name: "update_issue", arguments: { identifier: mcpIssue.identifier, status: "active" } });
  assert.equal(httpCli(["issue", "get", mcpIssue.identifier]).status, "active");

  // Both stdio paths are real subprocesses using the same temporary workspace.
  for (const extra of [[], ["--http", "--url", base]]) {
    const child = spawn(binary, ["--config", config, "--db", database, "mcp", ...extra], { env, stdio: ["pipe", "pipe", "pipe"] });
    const pending = new Map<number, { resolve: (value: any) => void; reject: (error: Error) => void }>();
    let output = "";
    child.stdout.on("data", chunk => {
      output += chunk;
      let end: number;
      while ((end = output.indexOf("\n")) >= 0) {
        const line = output.slice(0, end); output = output.slice(end + 1);
        if (!line.trim()) continue;
        const message = JSON.parse(line);
        pending.get(message.id)?.resolve(message); pending.delete(message.id);
      }
    });
    child.stderr.resume();
    child.on("error", error => { for (const promise of pending.values()) promise.reject(error); });
    child.on("exit", () => { for (const promise of pending.values()) promise.reject(new Error("MCP subprocess exited")); });
    const send = async (id: number, method: string, params: unknown) => {
      let timer: ReturnType<typeof setTimeout>;
      const reply = new Promise<any>((resolve, reject) => {
        pending.set(id, { resolve, reject });
        timer = setTimeout(() => { pending.delete(id); reject(new Error(`MCP timed out: ${method}`)); }, 10_000);
      });
      child.stdin.write(JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n");
      try { const result = await reply; assert(!result.error, JSON.stringify(result)); return result.result; }
      finally { clearTimeout(timer!); }
    };
    try {
      await send(1, "initialize", { protocolVersion: "2025-03-26", capabilities: {}, clientInfo: { name: "local-regression", version: "1" } });
      child.stdin.write('{"jsonrpc":"2.0","method":"notifications/initialized"}\n');
      const result = await send(2, "tools/call", { name: "get_issue", arguments: { identifier: mcpIssue.identifier } });
      assert(JSON.stringify(result).includes("Created by localhost MCP"));
    } finally { child.stdin.end(); child.kill("SIGTERM"); }
  }

  for (const args of [
    ["--backend", "http", "--url", "https://external.invalid", "project", "list"],
    ["mcp", "--http", "--url", "http://192.168.1.20"],
    ["connect", "--http", "--url", "https://external.invalid", "--dry-run", "--yes"],
  ]) {
    const result = spawnSync(binary, ["--config", config, ...args], { encoding: "utf8", env });
    assert.notEqual(result.status, 0); assert(result.stderr.includes("Local-only policy"));
  }

  await page.goto(`${base}/#/settings`);
  await page.getByRole("heading", { name: "Personal workspace", exact: true }).waitFor();
  await page.getByLabel("Workspace name", { exact: true }).fill("Local agents workspace");
  await page.getByRole("button", { name: "Save workspace name", exact: true }).click();
  await page.getByRole("status").filter({ hasText: "Workspace name saved." }).waitFor();
  assert.equal((await (await fetch(`${base}/api/workspace`)).json()).name, "Local agents workspace");
  await page.reload();
  await page.waitForFunction(() => (document.querySelector("#workspace-name") as HTMLInputElement)?.value === "Local agents workspace");
  await page.getByLabel("Connection ID", { exact: true }).fill("browser-agent");
  await page.getByLabel("Tool display name", { exact: true }).fill("Browser test agent");
  await page.getByRole("button", { name: "Create local connection", exact: true }).click();
  await page.getByRole("button", { name: "Show token", exact: true }).waitFor();
  await page.getByRole("button", { name: "Show token", exact: true }).click();
  let connection = JSON.parse(await page.locator("pre").innerText()).mcpServers["browser-agent"];
  assert.equal(connection.command, binary);
  assert.deepEqual(connection.args, ["--db", database, "mcp"]);
  const toolKey = connection.env.LIFIC_TOKEN;
  await page.getByRole("combobox", { name: /^Transport/ }).selectOption("http");
  connection = JSON.parse(await page.locator("pre").innerText()).mcpServers["browser-agent"];
  assert.equal(connection.url, `${base}/mcp`);
  assert.equal(connection.headers.Authorization, `Bearer ${toolKey}`);
  await rpc("tools/call", { name: "create_issue", arguments: { project: "AGNT", title: "Named local agent" } }, toolKey);
  const forbidden = await fetch(`${base}/api/auth/bots`, { method: "POST", headers: { authorization: `Bearer ${toolKey}`, "content-type": "application/json" }, body: '{"tool":"unwanted"}' });
  assert.equal(forbidden.status, 401, "tool tokens cannot mint more tools");
  await page.getByRole("button", { name: "Disconnect Browser test agent", exact: true }).click();
  await page.getByRole("button", { name: "Reconnect Browser test agent", exact: true }).waitFor();
  assert.equal((await fetch(`${base}/mcp`, { method: "POST", headers: { authorization: `Bearer ${toolKey}` } })).status, 401);

  const exportFile = join(scratch, "github-issues.json");
  writeFileSync(exportFile, JSON.stringify([{ number: 91, title: "Imported local file", state: "open", body: "A saved source link: https://external.invalid/issue/91", comments: [{ body: "Saved offline comment" }], labels: [{ name: "imported" }] }]));
  const args = ["import-file", exportFile, "--source", "github", "--namespace", "fixture/repo", "--project", "AGNT"];
  assert.equal(cli([...args, "--dry-run"]).issues_created, 1);
  assert(!cli(["issue", "list", "--project", "AGNT"]).some((issue: { title: string }) => issue.title === "Imported local file"));
  await page.goto(`${base}/#/AGNT/overview`);
  await page.getByText("Import issues from a local file", { exact: true }).click();
  await page.getByLabel("Source repository (owner/repo)", { exact: true }).fill("fixture/repo");
  await page.getByLabel("Issue export file", { exact: true }).setInputFiles(exportFile);
  await page.getByRole("button", { name: "Preview import", exact: true }).click();
  await page.getByRole("status").filter({ hasText: "Preview: 1 issues" }).waitFor();
  await page.getByRole("button", { name: "Import into AGNT", exact: true }).click();
  await page.getByRole("status").filter({ hasText: "Imported: 1 issues" }).waitFor();
  assert.equal(cli(args).issues_skipped_existing, 1);
  const imported = cli(["issue", "list", "--project", "AGNT"]).find((issue: { title: string }) => issue.title === "Imported local file");
  assert(imported);
  // A generated 0.1-second silent MP3 proves inline media survives sanitizing without opening
  // remote src/source/poster/autoplay paths.
  const mp3 = readFileSync(join(import.meta.dir, "fixtures/silence.mp3"));
  const form = new FormData();
  form.append("file", new Blob([mp3], { type: "audio/mpeg" }), "local.mp3");
  const upload = await fetch(`${base}/api/attachments`, { method: "POST", body: form });
  assert.equal(upload.status, 200, await upload.clone().text());
  const attachment = await upload.json();
  const mediaPath = `/api/attachments/${attachment.id}`;
  httpCli(["issue", "update", imported.identifier, "--description", `<audio autoplay src="${mediaPath}"></audio>\n\n<video src="https://external.invalid/movie.mp4" poster="https://external.invalid/poster.png"><source src="https://external.invalid/source.mp4"></video>\n\n<audio src="/api/auth/auto-login"></audio>`]);
  await page.goto(`${base}/AGNT/issues/${imported.identifier}`);
  const audio = page.locator(`audio[src="${mediaPath}"]`);
  await audio.waitFor();
  assert.equal(await audio.getAttribute("preload"), "none");
  assert.equal(await audio.getAttribute("autoplay"), null);
  assert.equal(await audio.getAttribute("controls"), "");
  assert.equal(await page.locator('audio[src="/api/auth/auto-login"], [src*="external.invalid"], [poster]').count(), 0);
  await audio.evaluate((element: HTMLAudioElement) => element.load());
  await page.waitForFunction(path => {
    const player = document.querySelector(`audio[src="${path}"]`) as HTMLAudioElement;
    return player?.readyState >= 1;
  }, mediaPath);
  await page.goto(`${base}/#/`);
  await page.getByText("Local agents workspace", { exact: true }).first().waitFor();
  console.log("PASS: SQLite CLI, localhost HTTP CLI/MCP, stdio MCP and local proxy, named-tool config/revocation, workspace naming, offline import preview/write/deduplication, local audio and blocked external media");
}
