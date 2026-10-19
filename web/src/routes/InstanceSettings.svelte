<script lang="ts">
  import { onMount, getContext } from "svelte";
  import { listArchivedProjects, unarchiveProject, formatBytes, type ArchivedProject } from "../lib/api";
  import SettingsTabs from "../lib/SettingsTabs.svelte";
  import { toast } from "../lib/toast/toast.svelte";
  let { navigate }: { navigate: (path: string) => void } = $props();
  const topbarCtx = getContext<{ set: (s: import("svelte").Snippet | undefined) => void } | undefined>("lific:topbar");
  $effect(() => { topbarCtx?.set(topbarContent); return () => topbarCtx?.set(undefined); });
  let archived = $state<ArchivedProject[]>([]);
  let error = $state("");
  let loading = $state(true);
  let busy = $state<number | null>(null);
  onMount(async () => {
    const result = await listArchivedProjects();
    if (result.ok) archived = result.data;
    else error = result.error;
    loading = false;
  });
  async function restore(entry: ArchivedProject) {
    if (busy !== null) return;
    busy = entry.id; error = "";
    const result = await unarchiveProject(entry.id);
    busy = null;
    if (!result.ok) { error = result.error; return; }
    archived = archived.filter((item) => item.id !== entry.id);
    toast(`Restored ${entry.identifier}.`, { kind: "success" });
  }
</script>
{#snippet topbarContent()}<div class="px-6 py-2 text-body-sm font-medium">Settings</div>{/snippet}
<div class="flex-1 overflow-y-auto">
  <div class="w-full max-w-[1000px] mx-auto px-6 py-10 md:py-12">
    <SettingsTabs active="instance" isAdmin={true} {navigate} />
    <h1 class="font-display text-title mb-2">Local storage</h1>
    <p class="text-body text-[var(--text-muted)] mb-8">Your database, attachments, and backups are files on this computer.</p>
    <section class="rounded-xl bg-[var(--surface)] p-5 mb-6">
      <h2 class="text-body-lg font-semibold mb-2">Backups</h2>
      <p class="text-body-sm text-[var(--text-muted)]">Run <code>lific dump</code> to save a complete backup. To restore it, stop the app and run <code>lific restore path/to/backup.tar.gz</code>. Keep these files outside cloud-synced folders.</p>
    </section>
    <section class="rounded-xl bg-[var(--surface)] p-5">
      <h2 class="text-body-lg font-semibold mb-2">Archived projects</h2>
      <p class="text-body-sm text-[var(--text-muted)] mb-4">Archived projects stay in the local data folder and are included in backups. Restore one to work on it again.</p>
      {#if error}<p role="alert" class="text-[var(--error)]">{error}</p>{/if}
      {#if loading}<p>Loading…</p>
      {:else if archived.length === 0}<p class="text-body-sm text-[var(--text-faint)]">No archived projects.</p>
      {:else}{#each archived as entry (entry.id)}
        <div class="flex items-center gap-3 py-3 border-t border-[var(--border)]">
          <span class="font-mono text-caption">{entry.identifier}</span>
          <span class="flex-1">{entry.name}</span>
          <span class="text-caption text-[var(--text-muted)]">{formatBytes(entry.size_bytes)}</span>
          <button class="text-[var(--accent)] text-body-sm" disabled={busy !== null} onclick={() => restore(entry)}>{busy === entry.id ? "Restoring…" : "Restore"}</button>
        </div>
      {/each}{/if}
    </section>
  </div>
</div>
