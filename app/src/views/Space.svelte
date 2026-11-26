<script lang="ts">
  import { onMount } from "svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { api, type Drive } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { space, type SpaceTab } from "../lib/space.svelte";
  import { layout } from "../lib/sunburst";
  import { fileIcon, formatAge, formatBytes, formatCount, splitBytes, splitPath } from "../lib/format";
  import Glyph from "../components/Glyph.svelte";
  import OrbButton from "../components/OrbButton.svelte";
  import ActionButton from "../components/ActionButton.svelte";
  import Dock from "../components/Dock.svelte";
  import Icon from "../components/Icon.svelte";
  import Sunburst from "../components/Sunburst.svelte";
  import Modal from "../components/Modal.svelte";

  const RINGS = 4;
  let drives = $state<Drive[]>([]);
  let confirming = $state(false);
  let hoverId = $state<number | null>(null);

  onMount(() => {
    void api.drives().then((d) => (drives = d));
    void space.restore();
  });

  const colors = $derived(space.view ? layout(space.view, RINGS).colors : new Map<number, string>());
  const current = $derived(space.crumbs.at(-1));
  const maxEntry = $derived(space.entries[0]?.size || 1);
  const maxExt = $derived(space.summary?.extensions[0]?.bytes || 1);
  const scanned = $derived(splitBytes(space.progress.bytes));
  const now = Date.now() / 1000;

  async function pickFolder() {
    const dir = await api.pickFolder();
    if (dir) void space.scan(dir);
  }

  function pickedDrive(path: string) {
    return drives.find((d) => d.root.toLowerCase() === path.toLowerCase());
  }

  const TABS: { id: SpaceTab; label: string }[] = [
    { id: "folder", label: "ui.space.tab_folder" },
    { id: "largest", label: "ui.space.tab_largest" },
    { id: "types", label: "ui.space.tab_types" },
  ];

  async function confirmDelete() {
    confirming = false;
    await space.deletePicks();
  }

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
        <button class="drive" onclick={() => space.scan(d.root)}>
          <span class="d-ic"><Icon name="drive" size={20} /></span>
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
        <button class="crumb" class:last={i === space.crumbs.length - 1} onclick={() => space.open(c.id)}>
          {i === 0 ? c.name.replace(/\\$/, "") : c.name}
        </button>
      {/each}
    </nav>
    <button class="btn quiet small" onclick={() => space.reset()}><Icon name="refresh" size={14} /> {t("ui.space.other_drive")}</button>
  </header>

  <div class="split">
    <div class="chart">
      <Sunburst
        view={space.view}
        rings={RINGS}
        canUp={space.crumbs.length > 1}
        onopen={(id) => space.open(id)}
        onup={() => space.up()}
        highlight={hoverId}
      />
    </div>

    <aside class="panel">
      <div class="tabs segmented">
        {#each TABS as tab (tab.id)}
          <button class:on={space.tab === tab.id} onclick={() => space.setTab(tab.id)}>{t(tab.label)}</button>
        {/each}
      </div>

      <div class="rows">
        {#if space.tab === "folder"}
          {#each space.entries as e (e.id)}
            <div class="row" role="listitem" onmouseenter={() => (hoverId = e.id)} onmouseleave={() => (hoverId = null)}>
              <span class="sw" style:background={colors.get(e.id) ?? "var(--line-2)"}></span>
              <button class="name" disabled={e.kind !== "dir"} onclick={() => space.open(e.id)}>
                <Icon name={e.kind === "dir" ? "folder" : e.kind === "link" ? "link" : fileIcon(e.name)} size={15} />
                <span class="ellipsis">{e.name}</span>
              </button>
              <span class="share"><span style:width="{(e.size / maxEntry) * 100}%"></span></span>
              <span class="size num">{formatBytes(e.size)}</span>
              <span class="acts">
                <button class="icon-btn" title={t("ui.common.show_in_explorer")} onclick={() => api.diskReveal(e.id)}><Icon name="external" size={14} /></button>
                {#if e.protected}
                  <span class="icon-btn locked" title={e.protected} aria-label={e.protected}><Icon name="lock" size={14} /></span>
                {:else if e.kind !== "link"}
                  <button
                    class="icon-btn pick"
                    class:on={space.picks.has(e.id)}
                    title={t(space.picks.has(e.id) ? "ui.space.unpick" : "ui.space.pick")}
                    onclick={() => space.togglePick({ id: e.id, name: e.name, size: e.size })}
                  >
                    <Icon name={space.picks.has(e.id) ? "check" : "plus"} size={14} />
                  </button>
                {/if}
              </span>
            </div>
          {:else}
            <p class="faint empty">{t("ui.space.empty_folder")}</p>
          {/each}
        {:else if space.tab === "largest"}
          {#each space.largest as f (f.id)}
            {@const [dir] = splitPath(f.path)}
            <div class="row file">
              <span class="f-ic"><Icon name={fileIcon(f.name)} size={16} /></span>
              <span class="f-main">
                <span class="ellipsis">{f.name}</span>
                <span class="faint ellipsis small">{dir} · {f.modified ? formatAge(Math.floor((now - f.modified) / 86400)) : ""}</span>
              </span>
              <span class="size num">{formatBytes(f.size)}</span>
              <span class="acts">
                <button class="icon-btn" title={t("ui.common.show_in_explorer")} onclick={() => api.diskReveal(f.id)}><Icon name="external" size={14} /></button>
                {#if f.protected}
                  <span class="icon-btn locked" title={f.protected} aria-label={f.protected}><Icon name="lock" size={14} /></span>
                {:else}
                  <button
                    class="icon-btn pick"
                    class:on={space.picks.has(f.id)}
                    title={t(space.picks.has(f.id) ? "ui.space.unpick" : "ui.space.pick")}
                    onclick={() => space.togglePick({ id: f.id, name: f.name, size: f.size })}
                  >
                    <Icon name={space.picks.has(f.id) ? "check" : "plus"} size={14} />
                  </button>
                {/if}
              </span>
            </div>
          {:else}
            <p class="faint empty">{t("ui.space.no_files")}</p>
          {/each}
        {:else}
          <p class="faint note">{t("ui.space.types_note")}</p>
          {#each space.summary.extensions as x (x.ext)}
            <div class="row type">
              <span class="ext mono">{x.ext ? `.${x.ext}` : t("ui.space.no_ext")}</span>
              <span class="share"><span style:width="{(x.bytes / maxExt) * 100}%"></span></span>
              <span class="faint num small">{formatCount(x.files)}</span>
              <span class="size num">{formatBytes(x.bytes)}</span>
            </div>
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
        label={t("ui.common.to_bin")}
        disabled={space.picks.size === 0}
        onclick={() => (confirming = true)}
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
          <span class="faint">{t("ui.common.moved_to_bin")}</span>
        </div>
      {:else}
        <span class="faint hint">{t("ui.space.hint_before")} <Icon name="plus" size={12} /> {t("ui.space.hint_after")}</span>
      {/if}
    {/snippet}
  </Dock>
{/if}

{#if confirming}
  <Modal title={t("ui.space.confirm_title", { size: formatBytes(space.picksBytes) })} onclose={() => (confirming = false)}>
    <p class="dim">{t("ui.space.confirm_text")}</p>
    <ul class="picklist">
      {#each [...space.picks.values()].slice(0, 8) as p (p.id)}
        <li><span class="ellipsis">{p.name}</span><span class="num faint">{formatBytes(p.size)}</span></li>
      {/each}
      {#if space.picks.size > 8}<li class="faint">{t("ui.common.and_more", { n: space.picks.size - 8 })}</li>{/if}
    </ul>
    {#snippet buttons()}
      <button class="btn" onclick={() => (confirming = false)}>{t("ui.common.cancel")}</button>
      <button class="btn accent" onclick={confirmDelete}>{t("ui.common.move_to_bin")}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .stage {
    flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: 10px; padding: 0 32px 40px; text-align: center; min-height: 0; overflow: auto;
  }
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
  .crumb { border: 0; background: none; cursor: pointer; padding: 4px 6px; border-radius: 6px; color: var(--text-2); white-space: nowrap; }
  .crumb:hover { background: var(--hover); color: var(--text); }
  .crumb.last { color: var(--text); font-weight: 600; font-family: var(--font-display); font-size: 17px; }

  .split { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(300px, 1.1fr) minmax(340px, 1fr); gap: 24px; padding: 8px 32px 0; }
  .chart { min-height: 0; display: grid; place-items: center; padding: 8px; }
  .panel { min-height: 0; display: flex; flex-direction: column; gap: 10px; }
  .tabs { align-self: flex-start; }
  .rows { flex: 1; overflow: auto; margin-right: -10px; padding-right: 10px; }
  .row {
    display: grid; grid-template-columns: 10px minmax(0, 1fr) 70px 74px 64px; align-items: center; gap: 10px;
    padding: 6px 6px; border-radius: 8px; font-size: 13px;
  }
  .row:hover { background: var(--hover); }
  .row.file { grid-template-columns: 20px minmax(0, 1fr) 74px 64px; }
  .row.type { grid-template-columns: 90px 1fr 60px 74px; }
  .sw { width: 10px; height: 10px; border-radius: 3px; }
  .name { display: flex; align-items: center; gap: 8px; min-width: 0; border: 0; background: none; padding: 0; color: var(--text); cursor: pointer; text-align: left; }
  .name:disabled { cursor: default; color: var(--text-2); }
  .name:not(:disabled):hover span { text-decoration: underline; }
  .name :global(svg) { color: var(--text-3); }
  .share { height: 4px; border-radius: 99px; background: var(--line); overflow: hidden; }
  .share > span { display: block; height: 100%; background: color-mix(in srgb, var(--a) 70%, transparent); border-radius: 99px; }
  .size { text-align: right; font-weight: 600; }
  .acts { display: flex; justify-content: flex-end; gap: 2px; opacity: 0; transition: opacity 0.1s; }
  .row:hover .acts, .acts:has(.on) { opacity: 1; }
  .locked { cursor: help; color: var(--text-3, var(--text-2)); opacity: .75; }
  .pick.on { color: #fff; background: linear-gradient(135deg, var(--a), var(--b)); }
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

  .picklist { list-style: none; margin: 14px 0 0; padding: 0; display: flex; flex-direction: column; gap: 4px; font-size: 13px; }
  .picklist li { display: flex; justify-content: space-between; gap: 12px; }

  @media (max-width: 1000px) {
    .split { grid-template-columns: 1fr; overflow: auto; }
    .chart { max-height: 380px; }
  }
</style>
