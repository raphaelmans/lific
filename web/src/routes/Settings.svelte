<script lang="ts">
  import { onMount, getContext } from "svelte";
  import { me, updateProfile, getWorkspace, renameWorkspace, refreshSession, saveSession } from "../lib/api";
  import { getUserRevision, publishUser } from "../lib/userState";
  import LocalTools from "../lib/LocalTools.svelte";
  import SettingsTabs from "../lib/SettingsTabs.svelte";
  import {
    themePreference, setPreference, type ThemePreference,
    getAccent, setAccent, type AccentPreset, getDensity, setDensity, type Density,
    getFontScale, setFontScale, type FontScale,
    getMotionPreference, setMotionPreference, type MotionPreference,
  } from "../lib/theme";
  import { ACCENT_PRESETS } from "../lib/appearance/presets";
  import { Sun, Moon, Monitor, Palette, Check, Rows3, Rows2, Type, Zap, ZapOff } from "lucide-svelte";
  let { navigate }: { navigate: (path: string) => void } = $props();
  const topbarCtx = getContext<{ set: (s: import("svelte").Snippet | undefined) => void } | undefined>("lific:topbar");
  $effect(() => { topbarCtx?.set(topbarContent); return () => topbarCtx?.set(undefined); });
  let accentPref = $state<AccentPreset>(getAccent());
  let densityPref = $state<Density>(getDensity());
  let fontScalePref = $state<FontScale>(getFontScale());
  let motionPref = $state<MotionPreference>(getMotionPreference());
  let displayName = $state("");
  let error = $state("");
  let saved = $state(false);
  let saving = $state(false);
  let workspaceName = $state("");
  let workspaceSaved = $state(false);
  let loading = $state(true);
  onMount(async () => {
    const [profile, workspace] = await Promise.all([me(), getWorkspace()]);
    if (profile.ok) displayName = profile.data.display_name;
    else error = profile.error;
    if (workspace.ok) workspaceName = workspace.data.name;
    else error = workspace.error;
    loading = false;
  });
  async function saveName() {
    if (saving || !displayName.trim()) return;
    saving = true; error = ""; saved = false;
    const revision = getUserRevision();
    const result = await updateProfile({ display_name: displayName.trim() });
    saving = false;
    if (result.ok) { publishUser(result.data, revision); saved = true; }
    else error = result.error;
  }
  async function saveWorkspace() {
    saving = true; error = ""; workspaceSaved = false;
    try {
      const session = await refreshSession();
      if (!session.ok) { error = session.error; return; }
      saveSession(session.data.token);
      const result = await renameWorkspace(workspaceName);
      if (result.ok) { workspaceName = result.data.name; workspaceSaved = true; }
      else error = result.error;
    } finally { saving = false; }
  }
  function pickTheme(p: ThemePreference) {
    setPreference(p);
  }

  function pickAccent(p: AccentPreset) {
    accentPref = p;
    setAccent(p);
  }

  function pickDensity(p: Density) {
    densityPref = p;
    setDensity(p);
  }

  function pickFontScale(p: FontScale) {
    fontScalePref = p;
    setFontScale(p);
  }

  function pickMotion(p: MotionPreference) {
    motionPref = p;
    setMotionPreference(p);
  }

</script>

{#snippet topbarContent()}
  <div class="flex items-center gap-3 px-6 py-2 w-full"><span class="text-body-sm font-medium">Settings</span></div>
{/snippet}
<div class="flex-1 overflow-y-auto">
  <div class="w-full max-w-[1000px] mx-auto px-6 py-10 md:py-12">
    <SettingsTabs active="account" isAdmin={true} {navigate} />
    <h1 class="font-display text-title text-[var(--text)] mb-2">Personal workspace</h1>
    <p class="text-body text-[var(--text-muted)] mb-8">Your projects stay on this computer. The app opens directly, without an account or password.</p>
    <section class="rounded-xl bg-[var(--surface)] p-5 mb-6">
      <label for="workspace-name" class="block text-body-sm font-semibold mb-2">Workspace name</label>
      <p class="text-body-sm text-[var(--text-muted)] mb-3">Shown on your home dashboard.</p>
      <form class="flex flex-wrap gap-3 mb-6" onsubmit={(event) => { event.preventDefault(); saveWorkspace(); }}>
        <input id="workspace-name" bind:value={workspaceName} disabled={loading} maxlength="60" placeholder="My workspace" class="min-w-0 rounded-md border border-[var(--border)] bg-[var(--bg)] px-3 py-2" />
        <button class="rounded-md bg-[var(--accent)] text-[var(--accent-text)] px-4 py-2" disabled={loading || saving}>Save workspace name</button>
      </form>
      {#if workspaceSaved}<p role="status" class="text-caption mb-4">Workspace name saved.</p>{/if}
      <label for="local-name" class="block text-body-sm font-semibold mb-2">Display name</label>
      <p class="text-body-sm text-[var(--text-muted)] mb-3">Used for authorship in your local history.</p>
      <form class="flex flex-wrap gap-3" onsubmit={(event) => { event.preventDefault(); saveName(); }}>
        <input id="local-name" bind:value={displayName} disabled={loading} class="rounded-md border border-[var(--border)] bg-[var(--bg)] px-3 py-2" required />
        <button class="rounded-md bg-[var(--accent)] text-[var(--accent-text)] px-4 py-2" disabled={loading || saving}>{saving ? "Saving…" : "Save name"}</button>
      </form>
      {#if saved}<p class="text-caption mt-2" role="status">Saved.</p>{/if}
      {#if error}<p class="text-caption text-[var(--error)] mt-2" role="alert">{error}</p>{/if}
    </section>
      <section class="rounded-xl bg-[var(--surface)] shadow-[0_1px_2px_rgba(0,0,0,0.06)] p-5 animate-reveal delay-250">
        <div class="flex items-center gap-2 mb-1">
          <Palette size={15} class="text-[var(--text-muted)]" />
          <h2 class="text-body-lg font-semibold text-[var(--text)]">Appearance</h2>
        </div>
        <p class="text-body-sm text-[var(--text-muted)] mb-3.5">System follows your OS.</p>
        <div class="inline-flex p-0.5 rounded-lg bg-[var(--bg)] shadow-[inset_0_1px_2px_rgba(0,0,0,0.10)]">
          {#each [["light", "Light", Sun], ["dark", "Dark", Moon], ["system", "System", Monitor]] as [val, label, Icon]}
            {@const IconComp = Icon as typeof Sun}
            <button
              class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-body-sm font-medium transition-all
                     {$themePreference === val
                ? 'bg-[var(--surface)] text-[var(--text)] shadow-[0_1px_2px_rgba(0,0,0,0.12)]'
                : 'text-[var(--text-muted)] hover:text-[var(--text)]'}"
              onclick={() => pickTheme(val as ThemePreference)}
            >
              <IconComp size={14} />
              {label}
            </button>
          {/each}
        </div>

        <!-- Accent color -->
        <div class="mt-5 pt-5 border-t border-[var(--border)]">
          <span class="block text-micro font-semibold uppercase tracking-widest text-[var(--text-faint)] mb-2.5">Accent color</span>
          <div class="flex items-center gap-2.5 flex-wrap">
            {#each ACCENT_PRESETS as preset (preset.id)}
              <button
                type="button"
                class="size-8 shrink-0 rounded-full transition-transform motion-safe:active:scale-90
                       {accentPref === preset.id ? 'ring-2 ring-offset-2 ring-offset-[var(--surface)] ring-[var(--text)]' : 'hover:scale-110'}"
                style:background-color={preset.swatch}
                onclick={() => pickAccent(preset.id)}
                title={preset.label}
                aria-label="Accent: {preset.label}"
                aria-pressed={accentPref === preset.id}
              >
                {#if accentPref === preset.id}
                  <Check size={14} class="mx-auto text-white drop-shadow-[0_1px_1px_rgba(0,0,0,0.4)]" />
                {/if}
              </button>
            {/each}
          </div>
        </div>

        <!-- Density -->
        <div class="mt-5 pt-5 border-t border-[var(--border)]">
          <span class="block text-micro font-semibold uppercase tracking-widest text-[var(--text-faint)] mb-2.5">Density</span>
          <div class="inline-flex p-0.5 rounded-lg bg-[var(--bg)] shadow-[inset_0_1px_2px_rgba(0,0,0,0.10)]">
            {#each [["comfortable", "Comfortable", Rows3], ["compact", "Compact", Rows2]] as [val, label, Icon]}
              {@const IconComp = Icon as typeof Rows3}
              <button
                class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-body-sm font-medium transition-all
                       {densityPref === val
                  ? 'bg-[var(--surface)] text-[var(--text)] shadow-[0_1px_2px_rgba(0,0,0,0.12)]'
                  : 'text-[var(--text-muted)] hover:text-[var(--text)]'}"
                onclick={() => pickDensity(val as Density)}
              >
                <IconComp size={14} />
                {label}
              </button>
            {/each}
          </div>
        </div>

        <!-- Text size -->
        <div class="mt-5 pt-5 border-t border-[var(--border)]">
          <span class="flex items-center gap-1.5 text-micro font-semibold uppercase tracking-widest text-[var(--text-faint)] mb-2.5">
            <Type size={12} /> Text size
          </span>
          <div class="inline-flex p-0.5 rounded-lg bg-[var(--bg)] shadow-[inset_0_1px_2px_rgba(0,0,0,0.10)]">
            {#each [["sm", "S"], ["md", "M"], ["lg", "L"]] as [val, label]}
              <button
                class="px-3.5 py-1.5 rounded-md text-body-sm font-medium transition-all
                       {fontScalePref === val
                  ? 'bg-[var(--surface)] text-[var(--text)] shadow-[0_1px_2px_rgba(0,0,0,0.12)]'
                  : 'text-[var(--text-muted)] hover:text-[var(--text)]'}"
                onclick={() => pickFontScale(val as FontScale)}
              >
                {label}
              </button>
            {/each}
          </div>
        </div>

        <!-- Motion -->
        <div class="mt-5 pt-5 border-t border-[var(--border)]">
          <span class="block text-micro font-semibold uppercase tracking-widest text-[var(--text-faint)] mb-2.5">Motion</span>
          <div class="inline-flex p-0.5 rounded-lg bg-[var(--bg)] shadow-[inset_0_1px_2px_rgba(0,0,0,0.10)]">
            {#each [["system", "System", Monitor], ["reduced", "Reduced", ZapOff], ["full", "Full", Zap]] as [val, label, Icon]}
              {@const IconComp = Icon as typeof Monitor}
              <button
                class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-body-sm font-medium transition-all
                       {motionPref === val
                  ? 'bg-[var(--surface)] text-[var(--text)] shadow-[0_1px_2px_rgba(0,0,0,0.12)]'
                  : 'text-[var(--text-muted)] hover:text-[var(--text)]'}"
                onclick={() => pickMotion(val as MotionPreference)}
              >
                <IconComp size={14} />
                {label}
              </button>
            {/each}
          </div>
          <p class="text-caption text-[var(--text-muted)] mt-2">
            System honors your OS's reduce-motion setting.
          </p>
        </div>
      </section>
    <LocalTools />
    <section class="mt-6 text-body-sm text-[var(--text-muted)]">
      <h2 class="font-semibold mb-2">Help and source</h2>
      <p class="mb-2">These links open only when clicked. Upstream documentation also describes features unavailable in this personal fork.</p>
      <div class="flex flex-wrap gap-4">
        <a class="text-[var(--accent)] underline" href="https://github.com/raphaelmans/lific" target="_blank" rel="noopener noreferrer" referrerpolicy="no-referrer">Fork source and guide</a>
        <a class="text-[var(--accent)] underline" href="https://lific.dev/docs" target="_blank" rel="noopener noreferrer" referrerpolicy="no-referrer">Upstream documentation</a>
      </div>
    </section>
  </div>
</div>
