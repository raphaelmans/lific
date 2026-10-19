<script lang="ts">
  import { listProjectGroups, type ProjectGroup } from "./api";
  import IconPicker from "./IconPicker.svelte";
  import Select from "./Select.svelte";

  let {
    name = $bindable(""),
    identifier = $bindable(""),
    description = $bindable(""),
    emoji = $bindable(""),
    groupId = $bindable<number | null>(null),
    mode = "create",
    identifierLocked = false,
  }: {
    name?: string;
    identifier?: string;
    description?: string;
    emoji?: string;
    /** Sidebar group to file the project into. The form only collects it —
     *  persisting it is the caller's job, since it rides a separate endpoint. */
    groupId?: number | null;
    mode?: "create" | "edit";
    identifierLocked?: boolean;
  } = $props();

  let groups = $state<ProjectGroup[]>([]);
  let identifierTouched = $state(false);

  $effect(() => {
    listProjectGroups().then((res) => {
      if (res.ok) groups = res.data;
    });
  });

  // Auto-generate identifier from name (create mode only)
  $effect(() => {
    if (mode === "create" && !identifierTouched && name) {
      identifier = name
        .toUpperCase()
        .replace(/[^A-Z0-9]+/g, "")
        .slice(0, 5);
    }
  });

  const PLACEHOLDERS = [
    "Half-Life 3",
    "Star Citizen 2",
    "Portal 3",
    "Aperture Science",
    "Rewriting Rust in Rust",
    "Is It DNS?",
    "TODO: Name This Later",
    "Untitled Goose Project",
    "Regex for Dummys",
    "Shovelware Simulator",
    "Moon Base Alpha",
    "Sentient Spreadsheet",
    "Banana for Scale",
  ];
  const placeholder = PLACEHOLDERS[Math.floor(Math.random() * PLACEHOLDERS.length)];

  let previewId = $derived((identifier.trim().toUpperCase() || "PRO") + "-1");

  let groupOptions = $derived([
    { value: null, label: "No group" },
    ...groups.map((g) => ({ value: g.id, label: g.name })),
  ]);

</script>

<div class="max-w-[560px] mx-auto px-6 pt-12 md:pt-16">
  <!-- Name -->
  <div class="mb-8">
    <label
      for="project-name"
      class="block text-body-sm font-medium text-[var(--text)] mb-2 w-fit"
    >
      Name
    </label>
    <input
      id="project-name"
      type="text"
      bind:value={name}
      class="w-full rounded-md px-3 py-2.5 text-body-lg
             border border-[var(--border)] bg-[var(--bg-subtle)]
             text-[var(--text)] placeholder:text-[var(--text-faint)]
             outline-none transition-colors
             focus:border-[var(--accent)] focus:shadow-[0_0_0_3px_var(--accent-subtle)]"
      placeholder={mode === "create" ? placeholder : ""}
      autofocus={mode === "create"}
    />
  </div>

  <!-- Identifier + Icon row -->
  <div class="flex gap-5 items-start mb-8">
    <div class="shrink-0">
      <label
        for="project-id"
        class="block text-body-sm font-medium text-[var(--text)] mb-2 w-fit"
      >
        Identifier
      </label>
      <input
        id="project-id"
        type="text"
        bind:value={identifier}
        oninput={() => { identifierTouched = true; }}
        disabled={identifierLocked}
        class="w-[120px] rounded-md px-3 py-2.5 text-body-lg font-mono
               uppercase tracking-wide
               border border-[var(--border)] bg-[var(--bg-subtle)]
               text-[var(--text)] placeholder:text-[var(--text-faint)]
               outline-none transition-colors
               focus:border-[var(--accent)] focus:shadow-[0_0_0_3px_var(--accent-subtle)]
               disabled:opacity-50 disabled:cursor-not-allowed"
        placeholder="PRO"
        maxlength="5"
        spellcheck="false"
        autocapitalize="characters"
      />
      <p class="mt-1.5 text-body-sm text-[var(--text-faint)] w-fit">
        {mode === "edit" ? "Issue prefix" : "Issues become"}
      </p>
      <span
        class="inline-block font-mono text-caption font-medium
               text-[var(--accent)] bg-[var(--accent-subtle)]
               px-1.5 py-0.5 rounded mt-0.5"
      >
        {previewId}
      </span>
    </div>

    <div>
      <label
        class="block text-body-sm font-medium text-[var(--text)] mb-2 w-fit"
      >
        Icon
      </label>
      <IconPicker value={emoji} onchange={(v) => { emoji = v; }} />
    </div>
  </div>

  {#if groups.length > 0}
    <!-- Sidebar grouping. Personal project organization. -->
    <div class="mb-8 max-w-xs">
      <label class="block text-body-sm font-medium text-[var(--text)] mb-2 w-fit">
        Group
      </label>
      <Select options={groupOptions} bind:value={groupId} placeholder="No group" />
    </div>
  {/if}

  <!-- Description -->
  <div class="mb-8">
    <div class="flex items-baseline gap-2 mb-2">
      <label
        for="project-desc"
        class="text-body-sm font-medium text-[var(--text)]"
      >
        Description
      </label>
      <span class="text-caption text-[var(--text-faint)]">optional</span>
    </div>
    <textarea
      id="project-desc"
      bind:value={description}
      class="w-full rounded-md px-3 py-2.5 text-body-lg min-h-[100px]
             border border-[var(--border)] bg-[var(--bg-subtle)]
             text-[var(--text)] placeholder:text-[var(--text-faint)]
             outline-none resize-y transition-colors
             focus:border-[var(--accent)] focus:shadow-[0_0_0_3px_var(--accent-subtle)]"
      placeholder="What is this project about?"
      rows="3"
    ></textarea>
  </div>
</div>
