<script lang="ts">
  import { importIssueFile, type FileImportSummary } from "./api";
  let { project, onImported }: { project: string; onImported?: () => void } = $props();
  let source = $state("github");
  let namespace = $state("");
  let file = $state<File | null>(null);
  let error = $state("");
  let busy = $state(false);
  let summary = $state<FileImportSummary | null>(null);
  let previewData = $state<unknown>(null);
  function reset() { summary = null; previewData = null; error = ""; }
  async function preview() {
    if (!file || busy) return;
    busy = true; reset();
    try {
      if (file.size > 4 * 1024 * 1024) throw new Error("Choose a JSON file no larger than 4 MiB.");
      const data = JSON.parse(await file.text());
      const result = await importIssueFile({ source, project, namespace: namespace || undefined, data, dry_run: true });
      if (!result.ok) { error = result.error; return; }
      previewData = data; summary = result.data;
    } catch (e) { error = e instanceof Error ? e.message : "Could not read that JSON file."; }
    finally { busy = false; }
  }
  async function apply() {
    if (!previewData || busy) return;
    busy = true; error = "";
    try {
      const result = await importIssueFile({ source, project, namespace: namespace || undefined, data: previewData, dry_run: false });
      if (!result.ok) { error = result.error; return; }
      summary = result.data; previewData = null; onImported?.();
    } finally { busy = false; }
  }
</script>

<details class="rounded-xl border border-[var(--border)] p-4 mt-5">
  <summary class="text-body-sm font-semibold cursor-pointer">Import issues from a local file</summary>
  <p class="text-body-sm text-[var(--text-muted)] mt-3">Choose a saved provider JSON export. Preview it before importing into {project}. No provider API or linked attachment is fetched.</p>
  <fieldset disabled={busy} class="grid gap-3 mt-4 sm:grid-cols-2">
    <label class="text-body-sm">Source format
      <select bind:value={source} onchange={reset} class="block w-full mt-1 p-2 rounded border border-[var(--border)] bg-[var(--bg)]">
        <option value="github">GitHub issue JSON</option><option value="linear">Linear issue JSON</option><option value="jira">Jira issue JSON</option>
      </select>
    </label>
    {#if source !== "linear"}
      <label class="text-body-sm">{source === "github" ? "Source repository (owner/repo)" : "Source site label"}
        <input bind:value={namespace} oninput={reset} placeholder={source === "github" ? "team/repository" : "company"} class="block w-full mt-1 p-2 min-w-0 rounded border border-[var(--border)] bg-[var(--bg)]" />
      </label>
    {/if}
    <label class="text-body-sm sm:col-span-2">Issue export file
      <input type="file" accept=".json,application/json" class="block w-full mt-1 text-caption" onchange={(event) => { file = event.currentTarget.files?.[0] ?? null; reset(); }} />
    </label>
    <button disabled={!file} onclick={preview} class="justify-self-start rounded-md bg-[var(--accent)] text-[var(--accent-text)] px-3 py-2 text-body-sm">{busy ? "Working…" : "Preview import"}</button>
  </fieldset>
  <p class="text-caption text-[var(--text-muted)] mt-3">Supports issue arrays, an issues array in a JSON object, and Linear issues.nodes exports. Comments must be embedded in the file. Existing source IDs are skipped; external assignees are not turned into accounts.</p>
  {#if error}<p role="alert" class="text-body-sm text-[var(--error)] mt-3">{error}</p>{/if}
  {#if summary}
    <p role="status" class="text-body-sm mt-4">{summary.dry_run ? "Preview:" : "Imported:"} {summary.issues_created} issues, {summary.dry_run ? summary.comments_planned : summary.comments_created} comments, {summary.dry_run ? summary.labels_planned : summary.labels_created} labels. {summary.issues_skipped_existing} existing issues skipped.</p>
    {#if summary.dry_run}<button disabled={busy || summary.issues_created === 0} onclick={apply} class="mt-3 px-3 py-2 rounded-md bg-[var(--accent)] text-[var(--accent-text)] text-body-sm">Import into {project}</button>{/if}
  {/if}
</details>
