<script lang="ts">
  import { onMount } from "svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { api, type DiskFile, type Drive } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { space, type Pick, type SpaceTab } from "../lib/space.svelte";
  import { menu, type MenuEntry } from "../lib/menu.svelte";
  import { copyText } from "../lib/activity.svelte";
  import DeleteFlow from "../components/DeleteFlow.svelte";
  import SystemPick from "../components/SystemPick.svelte";
  import { layout } from "../lib/sunburst";
  import { fileIcon, formatAge, formatBytes, formatCount, splitBytes, splitPath } from "../lib/format";
  import Glyph from "../components/Glyph.svelte";
  import OrbButton from "../components/OrbButton.svelte";
  import ActionButton from "../components/ActionButton.svelte";
  import Dock from "../components/Dock.svelte";
  import Icon from "../components/Icon.svelte";
  import FileIcon from "../components/FileIcon.svelte";
  import { fileKey, folderSvg } from "../lib/icons.svelte";
  import Sunburst from "../components/Sunburst.svelte";
  import Treemap from "../components/Treemap.svelte";
  import { extColor, rgb } from "../lib/treemap";

  const RINGS = 4;
  let drives = $state<Drive[]>([]);
  let deleting = $state(false);
  let systemAsk = $state<{ pick: Pick; reason: string } | null>(null);
  let hoverId = $state<number | null>(null);
  /** Elemento señalado en el gráfico: se resalta en la lista. */
  let chartHover = $state<number | null>(null);
  let rowsEl = $state<HTMLElement>();
  $effect(() => {
    if (chartHover === null || !rowsEl) return;
    rowsEl.querySelector(`[data-id="${chartHover}"]`)?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  });

  /** Lo necesario de una carpeta o archivo para marcarlo o abrir su menú. */
  interface NodeInfo {
    id: number;
    name: string;
    size: number;
    kind: "dir" | "file" | "link";
    protected: string | null;
    excluded: boolean;
  }

  const canPick = (n: NodeInfo) => n.kind !== "link" && !n.excluded && (!n.protected || app.settings.allow_system);

  /** Marca o desmarca. Los archivos de Windows piden aceptar el riesgo cada vez. */
  function pick(n: NodeInfo) {
    if (space.picks.has(n.id)) {
      space.picks.delete(n.id);
      return;
    }
    if (!canPick(n)) return;
    const p: Pick = { id: n.id, name: n.name, size: n.size };
    if (n.protected) systemAsk = { pick: { ...p, system: true }, reason: n.protected };
    else space.togglePick(p);
  }

  async function nodeMenu(e: MouseEvent, n: NodeInfo) {
    e.preventDefault();
    const path = await api.diskPath(n.id);
    const excl = path ? app.exclusionFor(path) : undefined;
    const picked = space.picks.has(n.id);
    const why = n.excluded ? t("ui.menu.excluded") : n.protected && !app.settings.allow_system ? t("ui.menu.protected") : undefined;
    const items: (MenuEntry | false | null)[] = [
      n.kind === "dir" && { label: t("ui.menu.open"), icon: "folder", action: () => space.open(n.id) },
      { label: t("ui.common.show_in_explorer"), icon: "external", action: () => api.diskReveal(n.id) },
      !!path && { label: t("ui.menu.copy_path"), icon: "copy", action: () => copyText(path!) },
      "-",
      picked
        ? { label: t("ui.space.unpick"), icon: "minus", action: () => void space.picks.delete(n.id) }
        : { label: t("ui.space.pick"), icon: "plus", disabled: !canPick(n), hint: why, action: () => pick(n) },
      "-",
      excl
        ? {
            label: t(excl.toLowerCase() === path?.toLowerCase() ? "ui.menu.unexclude" : "ui.menu.unexclude_parent"),
            icon: "ban",
            action: async () => {
              await app.removeExclusion(excl);
              await space.refresh();
            },
          }
        : !!path && {
            label: t("ui.menu.exclude"),
            icon: "shield",
            action: async () => {
              await app.addExclusion(path!);
              space.picks.delete(n.id);
              await space.refresh();
            },
          },
    ];
    menu.show(e, items.filter((x): x is MenuEntry => !!x));
  }

  async function chartMenu(e: MouseEvent, id: number) {
    e.preventDefault();
    const n = await api.diskEntry(id);
    if (n) await nodeMenu(e, n);
  }

  function driveMenu(e: MouseEvent, d: Drive) {
    menu.show(e, [
      { label: t("ui.menu.analyze"), icon: "pie", action: () => space.scan(d.root) },
      { label: t("ui.menu.copy_path"), icon: "copy", action: () => copyText(d.root) },
    ]);
  }

  onMount(() => {
    void api.drives().then((d) => (drives = d));
    void space.restore();
  });

  const colors = $derived(space.view ? layout(space.view, RINGS).colors : new Map<number, string>());
  const current = $derived(space.crumbs.at(-1));
  const maxEntry = $derived(space.entries[0]?.size || 1);
  const maxExt = $derived(space.summary?.extensions[0]?.bytes || 1);
  /** Puesto de cada extensión por espacio total: decide su color en el mapa. */
  const ranks = $derived(new Map((space.summary?.extensions ?? []).map((x, i) => [x.ext, i])));
  const scanned = $derived(splitBytes(space.progress.bytes));
  const now = Date.now() / 1000;

  async function pickFolder() {
    const dir = await api.pickFolder();
    if (dir) void space.scan(dir);
  }

  /** Desplegable para cambiar de unidad o carpeta sin salir del análisis. */
  function switchMenu(e: MouseEvent) {
    const cur = space.summary?.path.toLowerCase();
    menu.showUnder(e.currentTarget as HTMLElement, [
      ...drives.map((d) => ({
        label: `${d.label || t("ui.space.drive")} (${d.root.replace(/\\$/, "")})`,
        icon: "drive",
        hint: t("ui.space.free_short", { size: formatBytes(d.free_bytes) }),
        disabled: d.root.toLowerCase() === cur,
        action: () => space.scan(d.root),
      })),
      "-",
      { label: t("ui.space.pick_folder"), icon: "folder", action: pickFolder },
      { label: t("ui.space.all_drives"), icon: "grid", action: () => space.reset() },
    ]);
  }

  function pickedDrive(path: string) {
    return drives.find((d) => d.root.toLowerCase() === path.toLowerCase());
  }

  const TABS: { id: SpaceTab; label: string }[] = [
    { id: "folder", label: "ui.space.tab_folder" },
    { id: "largest", label: "ui.space.tab_largest" },
    { id: "types", label: "ui.space.tab_types" },
  ];


  $effect(() => {
    if (space.error) {
      app.error = space.error;
      space.error = null;
    }
  });
</script>

{#if space.phase === "idle"}
  <div class="stage">
    <Glyph icon="pie" />
    <h1>{t("ui.space.title")}</h1>
    <p class="dim lead">{t("ui.space.lead")}</p>

    <div class="drives">
      {#each drives as d (d.root)}
        {@const used = d.total_bytes - d.free_bytes}
        {@const pct = d.total_bytes ? used / d.total_bytes : 0}
        <button class="drive" onclick={() => space.scan(d.root)} oncontextmenu={(e) => driveMenu(e, d)}>
          <span class="d-ic"><FileIcon key="drive:{d.root}" fallback="drive" size={22} /></span>
          <span class="d-main">
            <span class="d-name">{d.label || t("ui.common.local_disk")} <span class="faint">({d.root.replace("\\", "")})</span></span>
            <span class="d-bar"><span style:width="{pct * 100}%" class:full={pct > 0.9}></span></span>
            <span class="d-sub faint num">{t("ui.space.free_of", { free: formatBytes(d.free_bytes), total: formatBytes(d.total_bytes) })}</span>
          </span>
          <Icon name="chevron" size={16} />
        </button>
      {/each}
      <button class="drive other" onclick={pickFolder}>
        <span class="d-ic"><Icon name="folder" size={20} /></span>
        <span class="d-main"><span class="d-name">{t("ui.space.pick_folder")}</span></span>
        <Icon name="chevron" size={16} />
      </button>
    </div>
  </div>
{:else if space.phase === "scanning"}
  <div class="stage">
    <OrbButton icon="pie" label={t("ui.clean.analyze")} busy onclick={() => space.cancel()} />
    <h1 class="num">{scanned[0]}<span class="unit">{scanned[1]}</span></h1>
    <p class="dim lead num">{tn("ui.common.files", space.progress.files)} · {tn("ui.common.folders", space.progress.dirs)}</p>
    <p class="faint path ellipsis">{space.progress.current || space.target}</p>
  </div>
{:else if space.summary && space.view}
  {@const drive = pickedDrive(space.summary.path)}
  <header class="bar">
    <nav class="crumbs" aria-label={t("ui.space.path")}>
      {#each space.crumbs as c, i (c.id)}
        {#if i > 0}<Icon name="chevron" size={14} />{/if}
        <button class="crumb" class:last={i === space.crumbs.length - 1} title={c.name} onclick={() => space.open(c.id)}>
          {i === 0 ? c.name.replace(/\\$/, "") : c.name}
        </button>
      {/each}
    </nav>
    <div class="bar-acts">
      <div class="segmented small" role="group" aria-label={t("ui.space.chart_aria")}>
        <button class:on={space.chart === "radial"} onclick={() => space.setChart("radial")}><Icon name="pie" size={13} /> {t("ui.space.chart_radial")}</button>
        <button class:on={space.chart === "map"} onclick={() => space.setChart("map")}><Icon name="grid" size={13} /> {t("ui.space.chart_map")}</button>
      </div>
      <button class="btn small" title={t("ui.space.rescan_hint")} onclick={() => space.rescan()}><Icon name="refresh" size={14} /> {t("ui.space.rescan")}</button>
      <button class="btn small switch" onclick={switchMenu}>
        <Icon name="drive" size={14} /> {t("ui.space.other_drive")} <Icon name="chevron" size={12} />
      </button>
    </div>
  </header>

  <div class="split" class:wide={space.chart === "map"}>
    <div class="chart" class:map={space.chart === "map"}>
      {#if space.chart === "map" && space.treemap}
        <Treemap data={space.treemap} {ranks} onopen={(id) => space.open(id)} highlight={hoverId} onmenu={chartMenu} onhover={(id) => (chartHover = id)} />
      {:else}
      <Sunburst
        view={space.view}
        rings={RINGS}
        canUp={space.crumbs.length > 1}
        onopen={(id) => space.open(id)}
        onup={() => space.up()}
        highlight={hoverId}
        onhover={(id) => (chartHover = id)}
        onmenu={chartMenu}
      />
      {/if}
    </div>

    <aside class="panel">
      <div class="tabs segmented">
        {#each TABS as tab (tab.id)}
          <button class:on={space.tab === tab.id} onclick={() => space.setTab(tab.id)}>{t(tab.label)}</button>
        {/each}
      </div>

      <div class="rows" class:focusing={chartHover !== null && space.tab === "folder"} bind:this={rowsEl}>
        {#if space.tab === "folder"}
          {#each space.entries as e (e.id)}
            <div
              class="row"
              class:excluded={e.excluded}
              class:picked={space.picks.has(e.id)}
              class:hot={chartHover === e.id}
              data-id={e.id}
              role="listitem"
              onmouseenter={() => (hoverId = e.id)}
              onmouseleave={() => (hoverId = null)}
              oncontextmenu={(ev) => nodeMenu(ev, e)}
            >
              <span class="sw" style:background={colors.get(e.id) ?? "var(--line-2)"}></span>
              <button class="name" disabled={e.kind !== "dir"} onclick={() => space.open(e.id)}>
                {#if e.kind === "dir"}
                  <FileIcon src={folderSvg(e.name)} key="node:{e.id}" fallback="folder" size={16} />
                {:else if e.kind === "link"}
                  <Icon name="link" size={15} />
                {:else}
                  <FileIcon key={fileKey(e.name, `node:${e.id}`)} fallback={fileIcon(e.name)} size={16} />
                {/if}
                <span class="ellipsis">{e.name}</span>
              </button>
              <span class="share"><span style:width="{(e.size / maxEntry) * 100}%"></span></span>
              <span class="size num">{formatBytes(e.size)}</span>
              <span class="acts">
                <button class="icon-btn" title={t("ui.common.show_in_explorer")} onclick={() => api.diskReveal(e.id)}><Icon name="external" size={14} /></button>
                {@render pickBtn(e)}
              </span>
            </div>
          {:else}
            <p class="faint empty">{t("ui.space.empty_folder")}</p>
          {/each}
        {:else if space.tab === "largest"}
          {#each space.largest as f (f.id)}
            {@render fileRow(f)}
          {:else}
            <p class="faint empty">{t("ui.space.no_files")}</p>
          {/each}
        {:else if space.extFilter !== null}
          {@const x = space.summary.extensions.find((e) => e.ext === space.extFilter)}
          <div class="ext-head">
            <button class="btn quiet small" onclick={() => space.clearExt()}><Icon name="back" size={14} /> {t("ui.space.tab_types")}</button>
            <span class="mono">{space.extFilter ? `.${space.extFilter}` : t("ui.space.no_ext")}</span>
            {#if x}<span class="faint small num">{tn("ui.common.files", x.files)} · {formatBytes(x.bytes)}</span>{/if}
          </div>
          {#if x && x.files > space.extFiles.length}
            <p class="faint note">{t("ui.space.ext_top", { n: space.extFiles.length })}</p>
          {/if}
          {#each space.extFiles as f (f.id)}
            {@render fileRow(f)}
          {:else}
            <p class="faint empty">{t("ui.space.no_files")}</p>
          {/each}
        {:else}
          <p class="faint note">{t("ui.space.types_note")}</p>
          {#each space.summary.extensions as x (x.ext)}
            <button class="row type" title={t("ui.space.ext_show")} onclick={() => space.showExt(x.ext)}>
              <span class="ext mono"><span class="sw" style:background={rgb(extColor(ranks.get(x.ext) ?? -1))}></span><FileIcon key="ext:{x.ext}" fallback={fileIcon(`a.${x.ext}`)} size={16} />{x.ext ? `.${x.ext}` : t("ui.space.no_ext")}</span>
              <span class="share"><span style:width="{(x.bytes / maxExt) * 100}%"></span></span>
              <span class="faint num small">{formatCount(x.files)}</span>
              <span class="size num">{formatBytes(x.bytes)}</span>
            </button>
          {/each}
        {/if}
      </div>
    </aside>
  </div>

  <Dock>
    {#snippet left()}
      <div class="stats faint">
        <span class="num">{t("ui.space.total_in", { size: formatBytes(space.summary!.bytes), files: tn("ui.common.files", space.summary!.files) })}</span>
        {#if drive}<span class="num">{t("ui.space.free_on", { size: formatBytes(drive.free_bytes), drive: drive.root })}</span>{/if}
        {#if space.summary!.unreadable > 0}
          <span class="warn">{tn("ui.space.unreadable", space.summary!.unreadable)}{app.info?.is_admin ? "" : ` ${t("ui.space.use_admin")}`}</span>
        {/if}
      </div>
    {/snippet}
    {#snippet center()}
      <ActionButton
        label={t("ui.space.continue")}
        disabled={space.picks.size === 0}
        onclick={() => (deleting = true)}
      />
    {/snippet}
    {#snippet right()}
      {#if space.picks.size > 0}
        <div class="sel">
          <strong class="num">{formatBytes(space.picksBytes)}</strong>
          <span class="faint">{tn("ui.space.picked", space.picks.size)} · <button class="link" onclick={() => space.picks.clear()}>{t("ui.space.clear_picks")}</button></span>
        </div>
      {:else if space.lastDelete}
        <div class="sel">
          <strong class="num ok">{formatBytes(space.lastDelete.freed_bytes)}</strong>
          <span class="faint">{t(space.lastPermanent ? "ui.common.deleted_forever" : "ui.common.moved_to_bin")}</span>
        </div>
      {:else}
        <span class="faint hint">{t("ui.space.hint_before")} <Icon name="plus" size={12} /> {t("ui.space.hint_after")}</span>
      {/if}
    {/snippet}
  </Dock>
{/if}

{#snippet fileRow(f: DiskFile)}
  {@const [dir] = splitPath(f.path)}
  {@const fn = { ...f, kind: "file" as const }}
  <div class="row file" class:excluded={f.excluded} class:picked={space.picks.has(f.id)} oncontextmenu={(ev) => nodeMenu(ev, fn)} role="listitem">
    <span class="f-ic"><FileIcon key={fileKey(f.name, `node:${f.id}`)} fallback={fileIcon(f.name)} size={20} /></span>
    <span class="f-main">
      <span class="ellipsis">{f.name}</span>
      <span class="faint ellipsis small">{dir} · {f.modified ? formatAge(Math.floor((now - f.modified) / 86400)) : ""}</span>
    </span>
    <span class="size num">{formatBytes(f.size)}</span>
    <span class="acts">
      <button class="icon-btn" title={t("ui.common.show_in_explorer")} onclick={() => api.diskReveal(f.id)}><Icon name="external" size={14} /></button>
      {@render pickBtn(fn)}
    </span>
  </div>
{/snippet}

{#snippet pickBtn(n: NodeInfo)}
  {#if n.excluded}
    <span class="icon-btn state excl" title={t("ui.space.excluded")} aria-label={t("ui.space.excluded")}><Icon name="shield" size={14} /></span>
  {:else if n.protected && !app.settings.allow_system}
    <span class="icon-btn state" title={n.protected} aria-label={n.protected}><Icon name="lock" size={14} /></span>
  {:else if n.kind !== "link"}
    <button
      class="icon-btn pick"
      class:on={space.picks.has(n.id)}
      class:sys={!!n.protected}
      title={n.protected ? `${t("ui.space.pick")} · ${n.protected}` : t(space.picks.has(n.id) ? "ui.space.unpick" : "ui.space.pick")}
      onclick={() => pick(n)}
    >
      <Icon name={space.picks.has(n.id) ? "check" : n.protected ? "alert" : "plus"} size={14} />
    </button>
  {/if}
{/snippet}

{#if deleting}
  <DeleteFlow
    items={[...space.picks.values()].map((p) => ({ key: p.id, name: p.name, size: p.size, system: p.system }))}
    run={(permanent) => space.deletePicks(permanent)}
    onclose={() => (deleting = false)}
  />
{/if}

{#if systemAsk}
  {@const ask = systemAsk}
  <SystemPick
    name={ask.pick.name}
    reason={ask.reason}
    oncancel={() => (systemAsk = null)}
    onaccept={() => {
      space.picks.set(ask.pick.id, ask.pick);
      systemAsk = null;
    }}
  />
{/if}

<style>
  .stage {
    flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: safe center;
    gap: 10px; padding: 0 32px 40px; text-align: center; min-height: 0; overflow: auto;
  }
  .stage > * { flex-shrink: 0; }
  .stage h1 { font-size: 30px; margin-top: 22px; }
  .lead { font-size: 15px; max-width: 56ch; }
  .unit { font-size: 0.5em; font-weight: 500; color: var(--text-2); margin-left: 6px; }
  .path { max-width: 70ch; font-size: 12px; font-family: var(--mono); }

  .drives { margin-top: 26px; width: min(560px, 100%); display: flex; flex-direction: column; }
  .drive {
    display: flex; align-items: center; gap: 14px; padding: 14px 12px; border: 0; background: none;
    border-bottom: 1px solid var(--line); cursor: pointer; text-align: left; color: var(--text-3); border-radius: 0;
  }
  .drive:last-child { border-bottom: 0; }
  .drive:hover { background: var(--hover); border-radius: 10px; }
  .d-ic {
    width: 40px; height: 40px; border-radius: 11px; display: grid; place-items: center; flex-shrink: 0;
    color: var(--a); background: color-mix(in srgb, var(--a) 14%, transparent);
  }
  .d-main { flex: 1; display: flex; flex-direction: column; gap: 5px; min-width: 0; }
  .d-name { color: var(--text); font-weight: 600; }
  .d-bar { height: 5px; border-radius: 99px; background: var(--line-2); overflow: hidden; }
  .d-bar > span { display: block; height: 100%; border-radius: 99px; background: linear-gradient(90deg, var(--a), var(--b)); }
  .d-bar > span.full { background: linear-gradient(90deg, var(--warn), var(--bad)); }
  .d-sub { font-size: 12px; }

  .bar { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 0 32px 4px; }
  .crumbs { display: flex; align-items: center; gap: 2px; min-width: 0; overflow: hidden; color: var(--text-3); }
  .crumbs > :global(svg) { flex-shrink: 0; }
  /* Rutas largas: se encogen las carpetas de arriba, nunca la actual. */
  .crumb { flex: 0 1 auto; min-width: 2.2em; overflow: hidden; text-overflow: ellipsis; border: 0; background: none; cursor: pointer; padding: 4px 6px; border-radius: 6px; color: var(--text-2); white-space: nowrap; }
  .crumb.last { flex-shrink: 0; max-width: 55%; }
  .crumb:hover { background: var(--hover); color: var(--text); }
  .crumb.last { color: var(--text); font-weight: 600; font-family: var(--font-display); font-size: 17px; }

  .rows .row { transition: opacity 0.12s, background 0.12s; }
  .rows.focusing .row:not(.hot) { opacity: 0.38; }
  .row.hot { background: var(--hover); box-shadow: inset 3px 0 0 var(--a); }
  .switch { gap: 6px; }
  .switch :global(svg:last-child) { transform: rotate(90deg); }
  .ext-head { display: flex; align-items: center; gap: 10px; padding: 4px 4px 8px; }
  .ext-head .mono { font-weight: 600; color: var(--text); }
  button.row.type { width: 100%; border: 0; background: none; text-align: left; cursor: pointer; font: inherit; color: inherit; }
  button.row.type:hover { background: var(--hover); }
  .split { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(300px, 1.1fr) minmax(340px, 1fr); gap: 24px; padding: 8px 32px 0; }
  .chart { min-height: 0; display: grid; place-items: center; padding: 8px; container-type: size; }
  .split.wide { grid-template-columns: minmax(360px, 1.7fr) minmax(340px, 1fr); }
  .chart.map { display: block; padding: 0 0 12px; }
  .bar-acts { display: flex; align-items: center; gap: 10px; }
  .segmented.small button { font-size: 12px; padding: 4px 10px; display: inline-flex; align-items: center; gap: 5px; }
  .ext { display: inline-flex; align-items: center; gap: 7px; min-width: 0; white-space: nowrap; overflow: hidden; }
  .ext .sw { flex-shrink: 0; }
  .panel { min-height: 0; display: flex; flex-direction: column; gap: 10px; }
  .tabs { align-self: flex-start; }
  .rows { flex: 1; overflow: auto; margin-right: -10px; padding-right: 10px; }
  .row {
    display: grid; grid-template-columns: 10px minmax(0, 1fr) 70px 74px 64px; align-items: center; gap: 10px;
    padding: 6px 6px; border-radius: 8px; font-size: 13px;
  }
  .row:hover { background: var(--hover); }
  .row.file { grid-template-columns: 20px minmax(0, 1fr) 74px 64px; }
  .row.type { grid-template-columns: 148px 1fr 60px 74px; }
  .sw { width: 10px; height: 10px; border-radius: 3px; }
  .name { display: flex; align-items: center; gap: 8px; min-width: 0; border: 0; background: none; padding: 0; color: var(--text); cursor: pointer; text-align: left; }
  .name:disabled { cursor: default; color: var(--text-2); }
  .name:not(:disabled):hover span { text-decoration: underline; }
  .name :global(svg) { color: var(--text-3); }
  .share { height: 4px; border-radius: 99px; background: var(--line); overflow: hidden; }
  .share > span { display: block; height: 100%; background: color-mix(in srgb, var(--a) 70%, transparent); border-radius: 99px; }
  .size { text-align: right; font-weight: 600; }
  .acts { display: flex; justify-content: flex-end; gap: 2px; opacity: 0; transition: opacity 0.1s; }
  .row:hover .acts, .acts:has(.on), .acts:has(.excl) { opacity: 1; }
  .state { cursor: help; color: var(--text-3); opacity: 0.75; }
  .pick.on { color: #fff; background: linear-gradient(135deg, var(--a), var(--b)); }
  .pick.sys { color: var(--warn); }
  .pick.sys.on { background: var(--warn); color: #1a1205; }
  .row.picked { background: color-mix(in srgb, var(--a) 9%, transparent); }
  /* Exclusiones: verde azulado, en cursiva y sin poder marcarse. */
  .row.excluded { background: color-mix(in srgb, var(--excl) 8%, transparent); }
  .row.excluded .name, .row.excluded .f-main > span:first-child { color: var(--excl); font-style: italic; }
  .row.excluded .share > span { background: color-mix(in srgb, var(--excl) 60%, transparent); }
  .excl { color: var(--excl); opacity: 1; }
  .f-ic { color: var(--text-3); }
  .f-main { display: flex; flex-direction: column; min-width: 0; }
  .small { font-size: 11px; }
  .ext { color: var(--text-2); }
  .note { font-size: 12px; padding: 0 6px 6px; }
  .empty { padding: 20px 6px; }

  .stats { display: flex; flex-direction: column; font-size: 12px; gap: 1px; }
  .stats .warn { color: var(--warn); }
  .sel { display: flex; flex-direction: column; align-items: flex-end; }
  .sel strong { font-size: 16px; }
  .sel span { font-size: 12px; }
  .ok { color: var(--ok); }
  .hint { font-size: 12px; display: inline-flex; align-items: center; gap: 4px; }

  @media (max-width: 1000px) {
    .split, .split.wide { grid-template-columns: 1fr; grid-template-rows: auto auto; overflow: auto; }
    .chart { height: 320px; flex-shrink: 0; }
    .chart.map { height: 300px; }
    .panel { min-height: 360px; }
    .rows { overflow: visible; }
  }
  @media (max-height: 700px) {
    .bar { padding-bottom: 0; }
    .split { padding-top: 4px; }
  }
</style>
