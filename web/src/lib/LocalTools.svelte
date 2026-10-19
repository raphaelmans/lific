<script lang="ts">
  import { onMount } from "svelte";
  import { createBot, listBots, disconnectBot, deleteBot, getWorkspace, refreshSession, saveSession, type Bot, type LocalWorkspace } from "./api";
  import { copyToClipboard } from "./clipboard";
  let bots = $state<Bot[]>([]);
  let workspace = $state<LocalWorkspace | null>(null);
  let id = $state("");
  let name = $state("");
  let transport = $state("stdio");
  let error = $state("");
  let busy = $state(false);
  let key = $state("");
  let createdId = $state("");
  let revealed = $state(false);
  let copied = $state(false);
  let format = $state("standard");
  async function load() {
    const [tools, local] = await Promise.all([listBots(), getWorkspace()]);
    if (tools.ok) bots = tools.data; else error = tools.error;
    if (local.ok) workspace = local.data; else error = local.error;
  }
  onMount(load);
  async function connect(tool = id, display = name) {
    busy = true; error = ""; key = ""; revealed = false; copied = false;
    try {
      const session = await refreshSession();
      if (!session.ok) { error = session.error; return; }
      saveSession(session.data.token);
      const result = await createBot(tool, display.trim() || undefined);
      if (!result.ok) { error = result.error; return; }
      key = result.data.key; createdId = result.data.tool;
      await load();
    } finally { busy = false; }
  }
  async function disconnect(bot: Bot, remove = false) {
    busy = true; error = "";
    try {
      const result = remove ? await deleteBot(bot.id) : await disconnectBot(bot.id);
      if (!result.ok) error = result.error;
      else { key = ""; await load(); }
    } finally { busy = false; }
  }
  let config = $derived.by(() => {
    if (!workspace || !key) return "";
    const entry = transport === "stdio"
      ? { command: workspace.executable, args: ["--db", workspace.database, "mcp"], env: { LIFIC_TOKEN: key } }
      : { url: `${window.location.origin}/mcp`, headers: { Authorization: `Bearer ${key}` } };
    if (format === "codex") {
      const title = `[mcp_servers.${JSON.stringify(createdId)}]`;
      return transport === "stdio"
        ? `${title}\ncommand = ${JSON.stringify(workspace.executable)}\nargs = ${JSON.stringify(["--db", workspace.database, "mcp"])}\nenv = { LIFIC_TOKEN = ${JSON.stringify(key)} }`
        : `${title}\nurl = ${JSON.stringify(`${window.location.origin}/mcp`)}\nhttp_headers = { Authorization = ${JSON.stringify(`Bearer ${key}`)} }`;
    }
    return JSON.stringify(format === "vscode"
      ? { servers: { [createdId]: { type: transport === "stdio" ? "stdio" : "http", ...entry } } }
      : { mcpServers: { [createdId]: entry } }, null, 2);
  });
  async function copy() { copied = await copyToClipboard(config, { silentSuccess: true }); }
</script>

<section class="rounded-xl bg-[var(--surface)] p-5 mt-6">
  <h2 class="text-body-lg font-semibold mb-2">Local agent tools</h2>
  <p class="text-body-sm text-[var(--text-muted)] mb-4">Agents can use the local CLI, stdio MCP, or the MCP endpoint on this computer. Named tools label your history and can be disconnected individually. A connected client's own data policy still applies.</p>
  <p class="text-body-sm text-[var(--text-muted)] mb-4">Run <code>lific mcp</code> for stdio. For automatic setup of an installed client, use <code>lific connect --client codex</code> or add <code>--http</code> for localhost HTTP.</p>
  <form class="grid gap-3 sm:grid-cols-2" onsubmit={(event) => { event.preventDefault(); connect(); }}>
    <label class="text-body-sm">Connection ID
      <input class="mt-1 block w-full min-w-0 rounded-md border border-[var(--border)] bg-[var(--bg)] p-2" bind:value={id} placeholder="my-local-agent" required pattern={"[a-zA-Z0-9][a-zA-Z0-9_\\-]{0,47}"} maxlength="48" />
    </label>
    <label class="text-body-sm">Tool display name
      <input class="mt-1 block w-full min-w-0 rounded-md border border-[var(--border)] bg-[var(--bg)] p-2" bind:value={name} placeholder="My local agent" maxlength="80" />
    </label>
    <label class="text-body-sm">Transport
      <select class="mt-1 block w-full rounded-md border border-[var(--border)] bg-[var(--bg)] p-2" bind:value={transport}>
        <option value="stdio">Local process (stdio)</option><option value="http">Localhost HTTP</option>
      </select>
    </label>
    <label class="text-body-sm">Config format
      <select class="mt-1 block w-full rounded-md border border-[var(--border)] bg-[var(--bg)] p-2" bind:value={format}>
        <option value="standard">MCP JSON (mcpServers)</option><option value="vscode">VS Code JSON</option><option value="codex">Codex TOML</option>
      </select>
    </label>
    <button class="justify-self-start rounded-md bg-[var(--accent)] text-[var(--accent-text)] px-4 py-2 text-body-sm" disabled={busy || !workspace}>Create local connection</button>
  </form>
  {#if error}<p role="alert" class="text-body-sm text-[var(--error)] mt-3">{error}</p>{/if}
  {#if key}
    <div class="mt-4 p-3 rounded-md border border-[var(--border)] min-w-0">
      <h3 class="text-body-sm font-semibold">Configuration for {createdId}</h3>
      <p class="text-caption text-[var(--text-muted)] my-2">Copy this into your client's MCP configuration. The token is shown only here; disconnect and reconnect if you lose it.</p>
      <pre class="max-w-full overflow-x-auto text-caption p-3 bg-[var(--bg)] rounded">{revealed ? config : config.split(key).join("••••••••")}</pre>
      <div class="flex flex-wrap gap-4 mt-3 text-body-sm">
        <button onclick={copy}>{copied ? "Copied" : "Copy configuration"}</button>
        <button onclick={() => revealed = !revealed}>{revealed ? "Hide token" : "Show token"}</button>
        <button onclick={() => key = ""}>Done</button>
      </div>
    </div>
  {/if}
  {#each bots as bot (bot.id)}
    <div class="flex flex-wrap justify-between items-center gap-2 py-3 mt-2 border-t border-[var(--border)]" data-connection-id={bot.tool_id}>
      <span class="text-body-sm">{bot.display_name} <span class="text-caption text-[var(--text-muted)]">{bot.connected ? "Connected" : "Disconnected"}</span></span>
      <div class="flex gap-3 text-body-sm">
        {#if bot.connected}<button disabled={busy} onclick={() => disconnect(bot)}>Disconnect {bot.display_name}</button>
        {:else}
          <button disabled={busy || !bot.tool_id} onclick={() => connect(bot.tool_id!, bot.display_name)}>Reconnect {bot.display_name}</button>
          <button disabled={busy} onclick={() => disconnect(bot, true)}>Remove {bot.display_name}</button>
        {/if}
      </div>
    </div>
  {/each}
</section>
