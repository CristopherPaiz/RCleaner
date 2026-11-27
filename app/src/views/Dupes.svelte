<script lang="ts">
  import { onMount } from "svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { api, type DupeGroup } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { dupes, MIN_SIZES, type KeepRule } from "../lib/dupes.svelte";
  import { fileIcon, formatBytes, formatCount, formatDate, splitBytes, splitPath } from "../lib/format";
  import Glyph from "../components/Glyph.svelte";
  import OrbButton from "../components/OrbButton.svelte";
  import ActionButton from "../components/ActionButton.svelte";
  import Dock from "../components/Dock.svelte";
  import FileIcon from "../components/FileIcon.svelte";
  import { fileKey } from "../lib/icons.svelte";
  import Icon from "../components/Icon.svelte";
  import DeleteFlow from "../components/DeleteFlow.svelte";
  import { menu } from "../lib/menu.svelte";
  import { copyText } from "../lib/activity.svelte";

  const PAGE = 120;
  let shown = $state(PAGE);
  let query = $state("");
  let confirming = $state(false);

  onMount(() => {
    dupes.init(app.info?.home ?? "");
    void dupes.restore();
  });

  $effect(() => {
    if (app.info) dupes.init(app.info.home);
  });

  $effect(() => {
    if (dupes.error) {
      app.error = dupes.error;
      dupes.error = null;
    }
  });

  const groups = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const all = dupes.report?.groups ?? [];
    return q ? all.filter((g) => g.files.some((f) => f.path.toLowerCase().includes(q))) : all;
  });
  const wasted = $derived(splitBytes(dupes.report?.wasted_bytes ?? 0));
  const hashing = $derived(dupes.progress.phase === "hashing" ? dupes.progress : null);
  const percent = $derived(hashing && hashing.total ? Math.round((hashing.done / hashing.total) * 100) : 0);

  async function addRoot() {
    const dir = await api.pickFolder();
    if (dir && !dupes.roots.some((r) => r.toLowerCase() === dir.toLowerCase())) dupes.roots = [...dupes.roots, dir];
  }

  function title(g: DupeGroup) {
    return splitPath(g.files[0].path)[1];
  }

  /** Archivos marcados, con su tamaño, para el diálogo de borrado. */
  const marked = $derived.by(() => {
    const out: { key: number; name: string; size: number }[] = [];
    for (const g of dupes.report?.groups ?? []) {
      for (const f of g.files) if (dupes.selected.has(f.id)) out.push({ key: f.id, name: f.path, size: g.size });
    }
    return out;
  });

  function fileMenu(e: MouseEvent, g: DupeGroup, f: { id: number; path: string }) {
    const isMarked = dupes.selected.has(f.id);
    const [dir] = splitPath(f.path);
    const excl = app.exclusionFor(dir);
    menu.show(e, [
      { label: t("ui.common.show_in_explorer"), icon: "external", action: () => api.dupesReveal(f.id) },
      { label: t("ui.menu.copy_path"), icon: "copy", action: () => copyText(f.path) },
      "-",
      isMarked
        ? { label: t("ui.dupes.keep_this"), icon: "check", action: () => dupes.toggle(g, f.id) }
        : { label: t("ui.dupes.mark_this"), icon: "trash", action: () => dupes.toggle(g, f.id) },
      { label: t("ui.dupes.keep_only_this"), icon: "star", action: () => keepOnly(g, f.id) },
      "-",
      excl
        ? { label: t("ui.menu.unexclude_parent"), icon: "ban", action: () => app.removeExclusion(excl) }
        : { label: t("ui.menu.exclude_folder"), icon: "shield", action: () => app.addExclusion(dir) },
    ]);
  }

  /** Conserva sólo `id` y marca las demás copias del grupo. */
  function keepOnly(g: DupeGroup, id: number) {
    for (const f of g.files) {
      if (f.id === id) dupes.selected.delete(f.id);
      else dupes.selected.add(f.id);
    }
  }

  const RULES: { id: KeepRule; label: string }[] = [
    { id: "oldest", label: "ui.dupes.keep_oldest" },
    { id: "newest", label: "ui.dupes.keep_newest" },
    { id: "shortest", label: "ui.dupes.keep_shortest" },
  ];
</script>

{#if dupes.phase === "idle" || dupes.phase === "scanning"}
  {@const scanning = dupes.phase === "scanning"}
  <div class="stage">
    <OrbButton
      icon="copy"
      label={t("ui.dupes.search")}
      busy={scanning}
      percent={scanning && hashing ? percent : null}
      disabled={dupes.roots.length === 0}
      onclick={() => (scanning ? dupes.cancel() : dupes.scan())}
    />
    {#if scanning}
      {#if hashing}
        <h1>{t("ui.dupes.comparing")}</h1>
        <p class="dim lead num">{t("ui.dupes.candidates", { done: hashing.done, total: hashing.total })}</p>
      {:else}
        <h1>{t("ui.dupes.listing")}</h1>
        <p class="dim lead num">{t("ui.dupes.checked", { n: dupes.progress.phase === "listing" ? dupes.progress.files : 0 })}</p>
      {/if}
      <p class="faint small">{t("ui.common.tap_to_stop")}</p>
    {:else}
      <h1>{t("ui.dupes.title")}</h1>
      <p class="dim lead">{t("ui.dupes.lead")}</p>

      <div class="setup">
        <div class="roots">
          {#each dupes.roots as r (r)}
            <div class="root">
              <Icon name="folder" size={16} />
              <span class="ellipsis">{r}</span>
              <button class="icon-btn" title={t("ui.common.remove")} onclick={() => (dupes.roots = dupes.roots.filter((x) => x !== r))}>
                <Icon name="x" size={14} />
              </button>
            </div>
          {/each}
          <button class="root add" onclick={addRoot}><Icon name="plus" size={16} /> {t("ui.dupes.add_folder")}</button>
        </div>

        <div class="opts">
          <span class="faint">{t("ui.dupes.min_size")}</span>
          <div class="segmented">
            {#each MIN_SIZES as m (m.bytes)}
              <button class:on={dupes.minSize === m.bytes} onclick={() => (dupes.minSize = m.bytes)}>{m.label}</button>
            {/each}
          </div>
        </div>
        <label class="check">
          <input type="checkbox" bind:checked={dupes.includeSystem} />
          <span>{t("ui.dupes.include_system")} <span class="faint">(AppData, node_modules, .git…)</span></span>
        </label>
      </div>
    {/if}
  </div>
{:else if dupes.report}
  <header class="results-head">
    <div>
      <div class="hero-num num">{wasted[0]}<span class="unit">{wasted[1]}</span></div>
      <p class="dim">
        {t("ui.dupes.summary", { groups: tn("ui.dupes.groups", dupes.report.groups.length), files: tn("ui.dupes.files_checked", dupes.report.files_scanned) })}
      </p>
    </div>
    <label class="field search">
      <Icon name="search" size={15} />
      <input placeholder={t("ui.dupes.filter_path")} bind:value={query} />
    </label>
  </header>

  <div class="subbar">
    <span class="faint">{t("ui.dupes.mark_keep")}</span>
    <select class="field rule" value={dupes.keep} onchange={(e) => dupes.autoSelect((e.currentTarget as HTMLSelectElement).value as KeepRule)}>
      {#each RULES as r (r.id)}<option value={r.id}>{t(r.label)}</option>{/each}
    </select>
    <button class="btn quiet small" onclick={() => dupes.selected.clear()}>{t("ui.dupes.unmark_all")}</button>
    {#if dupes.report.cloud_skipped > 0}
      <span class="faint note">{tn("ui.dupes.cloud_skipped", dupes.report.cloud_skipped)}</span>
    {/if}
  </div>

  <div class="list">
    {#each groups.slice(0, shown) as g, gi (g.files[0].id)}
      <section style:animation-delay="{Math.min(gi, 12) * 25}ms">
        <div class="g-head">
          <span class="g-ic"><FileIcon key={fileKey(title(g), `dupe:${g.files[0].id}`)} fallback={fileIcon(title(g))} size={20} /></span>
          <span class="g-title ellipsis">{title(g)}</span>
          <span class="faint num">{g.files.length} × {formatBytes(g.size)}</span>
        </div>
        {#each g.files as f (f.id)}
          {@const [dir, name] = splitPath(f.path)}
          {@const marked = dupes.selected.has(f.id)}
          <div class="row" class:marked oncontextmenu={(e) => fileMenu(e, g, f)} role="listitem">
            <input type="checkbox" checked={marked} onchange={() => dupes.toggle(g, f.id)} aria-label={t("ui.dupes.mark_file", { path: f.path })} />
            <span class="p" title={f.path}><span class="faint dir">{dir}\</span><span class="fname">{name}</span></span>
            <span class="keep">{marked ? "" : t("ui.dupes.kept")}</span>
            <span class="faint num date">{f.modified ? formatDate(f.modified) : ""}</span>
            <button class="icon-btn" title={t("ui.common.show_in_explorer")} onclick={() => api.dupesReveal(f.id)}><Icon name="external" size={14} /></button>
          </div>
        {/each}
      </section>
    {:else}
      <div class="none">
        <Glyph icon="check" size={72} success />
        <p class="dim">{t(query ? "ui.common.no_match" : "ui.dupes.none")}</p>
      </div>
    {/each}
    {#if groups.length > shown}
      <button class="btn more" onclick={() => (shown += PAGE)}>{t("ui.dupes.show_more", { n: Math.min(PAGE, groups.length - shown) })}</button>
    {/if}
  </div>

  <Dock>
    {#snippet left()}
      <button class="btn quiet" onclick={() => dupes.reset()}><Icon name="back" size={16} /> {t("ui.dupes.new_search")}</button>
    {/snippet}
    {#snippet center()}
      <ActionButton label={t("ui.space.continue")} disabled={dupes.selected.size === 0} onclick={() => (confirming = true)} />
    {/snippet}
    {#snippet right()}
      {#if dupes.selected.size > 0}
        <div class="sel">
          <strong class="num">{formatBytes(dupes.selectedBytes)}</strong>
          <span class="faint">{tn("ui.dupes.marked", dupes.selected.size)}</span>
        </div>
      {:else if dupes.lastDelete}
        <div class="sel">
          <strong class="num ok">{formatBytes(dupes.lastDelete.freed_bytes)}</strong>
          <span class="faint">{t(dupes.lastPermanent ? "ui.common.deleted_forever" : "ui.common.moved_to_bin")}</span>
        </div>
      {/if}
    {/snippet}
  </Dock>
{/if}

{#if confirming}
  <DeleteFlow items={marked} run={(permanent) => dupes.deleteSelected(permanent)} onclose={() => (confirming = false)} />
{/if}

<style>
  .stage {
    flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: safe center;
    gap: 10px; padding: 0 32px 40px; text-align: center; min-height: 0; overflow: auto;
  }
  .stage > * { flex-shrink: 0; }
  .stage h1 { font-size: 30px; margin-top: 26px; }
  .small { font-size: 12px; }
  .lead { font-size: 15px; max-width: 56ch; }

  .setup { margin-top: 26px; width: min(600px, 100%); display: flex; flex-direction: column; gap: 18px; text-align: left; }
  .roots { display: flex; flex-direction: column; }
  .root {
    display: flex; align-items: center; gap: 12px; padding: 10px 8px 10px 12px; border-bottom: 1px solid var(--line);
    color: var(--text-3);
  }
  .root span { flex: 1; color: var(--text); }
  .root.add { border: 0; background: none; cursor: pointer; color: var(--a); font-weight: 600; border-radius: 10px; }
  .root.add:hover { background: var(--hover); }
  .opts { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; font-size: 13px; }
  .check { display: flex; align-items: center; gap: 10px; font-size: 13px; cursor: pointer; }

  .hero-num { font-family: var(--font-display); font-size: 46px; font-weight: 600; letter-spacing: -0.02em; line-height: 1.05; }
  .unit { font-size: 0.45em; font-weight: 500; color: var(--text-2); margin-left: 6px; letter-spacing: 0; }
  .results-head { display: flex; justify-content: space-between; align-items: flex-end; gap: 24px; padding: 4px 32px 10px; flex-wrap: wrap; }
  .search { width: 240px; }
  .subbar { display: flex; align-items: center; gap: 8px; padding: 0 32px 8px; flex-wrap: wrap; font-size: 13px; }
  .rule { height: 30px; padding: 0 8px; }
  .note { margin-left: auto; font-size: 12px; }

  .list { flex: 1; overflow: auto; padding: 0 32px 12px; }
  section { padding: 12px 0 8px; border-bottom: 1px solid var(--line); animation: rise 0.25s ease-out backwards; }
  .g-head { display: flex; align-items: center; gap: 12px; padding: 0 8px 6px; }
  .g-ic {
    width: 32px; height: 32px; border-radius: 9px; display: grid; place-items: center; flex-shrink: 0;
    color: var(--a); background: color-mix(in srgb, var(--a) 14%, transparent);
  }
  .g-title { flex: 1; font-weight: 600; }
  .row {
    display: grid; grid-template-columns: 18px minmax(0, 1fr) 90px 150px 30px; gap: 12px; align-items: center;
    padding: 5px 8px 5px 52px; border-radius: 8px; font-size: 13px;
  }
  .row:hover { background: var(--hover); }
  .p { display: flex; min-width: 0; white-space: nowrap; }
  .p .dir { overflow: hidden; text-overflow: ellipsis; min-width: 40px; }
  .p .fname { flex-shrink: 0; max-width: 70%; overflow: hidden; text-overflow: ellipsis; }
  .row.marked .p { text-decoration: line-through; text-decoration-color: color-mix(in srgb, var(--bad) 60%, transparent); }
  .keep { font-size: 12px; color: var(--ok); }
  .date { font-size: 12px; text-align: right; }
  .none { display: flex; flex-direction: column; align-items: center; gap: 16px; padding: 60px 0; }
  .more { margin: 16px auto; display: flex; }

  .sel { display: flex; flex-direction: column; align-items: flex-end; }
  .sel strong { font-size: 16px; }
  .sel span { font-size: 12px; }
  .ok { color: var(--ok); }
</style>
