<script lang="ts">
  // Resumen: estado del equipo de un vistazo.
  // - Puntuación de salud (espacio libre, basura, papelera, última limpieza).
  // - Qué se puede limpiar ya, por categoría (análisis rápido automático con
  //   el modo por defecto; sólo lee, no borra nada).
  // - Unidades, actividad reciente y lo liberado en las últimas semanas.
  import { onMount } from "svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { api, type Category, type DeletionRecord, type Drive, type HistoryEntry } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { space } from "../lib/space.svelte";
  import { CATEGORY_ICON, CATEGORY_ORDER, categoryLabel, formatAge, formatBytes, formatDate, splitBytes, splitPath } from "../lib/format";
  import ActionButton from "../components/ActionButton.svelte";
  import Dock from "../components/Dock.svelte";
  import Icon from "../components/Icon.svelte";
  import FileIcon from "../components/FileIcon.svelte";
  import DeletionLog from "../components/DeletionLog.svelte";
  import { menu } from "../lib/menu.svelte";

  let drives = $state<Drive[]>([]);
  let history = $state<HistoryEntry[]>([]);
  let recent = $state<DeletionRecord[]>([]);
  let bin = $state<{ bytes: number; items: number } | null>(null);
  let showLog = $state(false);

  onMount(() => {
    void api.drives().then((d) => (drives = d));
    void api.history().then((h) => (history = h));
    void api.deletions(6).then((r) => (recent = r));
    void api.recycleBin().then((b) => (bin = b));
    estimate();
  });

  /** Análisis rápido una vez por sesión (alimenta también la vista Limpieza). */
  function estimate() {
    // Durante el asistente no: cambiar de idioma descarta el análisis.
    if (!app.settings.onboarded || app.busy || app.report || !app.mode || app.estimated) return;
    app.estimated = true;
    void app.scan();
  }
  $effect(() => {
    if (app.mode) estimate();
  });

  const main = $derived(drives.find((d) => d.root.toUpperCase().startsWith("C:")) ?? drives[0]);
  const freePct = $derived(main && main.total_bytes ? main.free_bytes / main.total_bytes : 1);
  const lastClean = $derived(history.at(-1));
  const daysSince = $derived(lastClean ? Math.floor((Date.now() / 1000 - lastClean.timestamp) / 86400) : null);
  const totalFreed = $derived(history.reduce((s, h) => s + h.freed_bytes, 0));

  // Lo que se limpiaría con la selección por defecto, por categoría.
  const scanning = $derived(app.phase === "scanning");
  const chosen = $derived((app.report?.findings ?? []).filter((f) => app.selected.has(f.id)));
  const junk = $derived(chosen.reduce((s, f) => s + f.bytes, 0));
  const byCat = $derived(
    CATEGORY_ORDER.map((c) => ({ c: c as Category, bytes: chosen.filter((f) => f.category === c).reduce((s, f) => s + f.bytes, 0) }))
      .filter((x) => x.bytes > 0)
      .sort((a, b) => b.bytes - a.bytes),
  );
  const maxCat = $derived(byCat[0]?.bytes || 1);
  const scanPct = $derived(app.progress.total ? Math.round((app.progress.done / app.progress.total) * 100) : 0);

  // Puntuación 0–100.
  const score = $derived.by(() => {
    const disk = freePct >= 0.25 ? 40 : freePct >= 0.1 ? 15 + ((freePct - 0.1) / 0.15) * 25 : (freePct / 0.1) * 15;
    const GB = 1024 ** 3;
    const trash = !app.report ? 22 : junk < 0.5 * GB ? 30 : junk < 2 * GB ? 22 : junk < 5 * GB ? 12 : 4;
    const fresh = daysSince === null ? 6 : daysSince <= 7 ? 20 : daysSince <= 30 ? 13 : 6;
    const recycle = !bin || bin.bytes < GB ? 10 : bin.bytes < 5 * GB ? 6 : 2;
    return Math.round(disk + trash + fresh + recycle);
  });
  const level = $derived(score >= 85 ? "great" : score >= 65 ? "good" : score >= 45 ? "fair" : "bad");

  interface Check {
    ok: boolean;
    icon: string;
    text: string;
    action?: { label: string; run: () => void };
  }
  const checks = $derived.by(() => {
    const out: Check[] = [];
    if (main) {
      out.push({
        ok: freePct >= 0.15,
        icon: "drive",
        text: t("ui.home.check_disk", { drive: main.root.replace("\\", ""), pct: Math.round(freePct * 100) }),
        action: freePct < 0.15 ? { label: t("ui.home.see_space"), run: () => openDrive(main) } : undefined,
      });
    }
    if (app.report) {
      out.push({
        ok: junk < 1024 ** 3,
        icon: "sparkles",
        text: junk > 0 ? t("ui.home.check_junk", { size: formatBytes(junk) }) : t("ui.home.check_no_junk"),
        action: junk > 0 ? { label: t("ui.home.review"), run: () => (app.view = "clean") } : undefined,
      });
    }
    if (bin) {
      out.push({
        ok: bin.bytes < 1024 ** 3,
        icon: "trash",
        text: bin.items > 0 ? t("ui.home.check_bin", { size: formatBytes(bin.bytes), n: bin.items }) : t("ui.home.check_bin_empty"),
      });
    }
    out.push({
      ok: daysSince !== null && daysSince <= 30,
      icon: "clock",
      text: daysSince === null ? t("ui.home.never") : t("ui.home.check_last", { when: formatAge(daysSince) }),
    });
    if (app.info && !app.info.is_admin) out.push({ ok: true, icon: "shield", text: t("ui.home.check_admin") });
    return out;
  });

  // Liberado por semana (últimas 8).
  const weeks = $derived.by(() => {
    const now = Date.now() / 1000;
    const w = Array.from({ length: 8 }, () => 0);
    for (const h of history) {
      const i = Math.floor((now - h.timestamp) / (7 * 86400));
      if (i >= 0 && i < 8) w[7 - i] += h.freed_bytes;
    }
    return w;
  });
  const maxWeek = $derived(Math.max(...weeks, 1));

  const hour = new Date().getHours();
  const greeting = hour < 12 ? "ui.home.morning" : hour < 19 ? "ui.home.afternoon" : "ui.home.evening";
  const junkParts = $derived(splitBytes(junk));
  const C = 2 * Math.PI * 44;

  function openDrive(d: Drive) {
    app.view = "space";
    void space.scan(d.root);
  }

  function quickClean() {
    if (app.report && app.selected.size > 0) app.view = "clean";
    else {
      const mode =
        app.modes.find((m) => m.id === app.settings.default_mode) ?? app.modes.find((m) => m.id === "rapido") ?? app.modes[0];
      if (mode) app.openMode(mode, true);
    }
  }
</script>

<div class="home">
  <header class="head">
    <div>
      <h1>{t(greeting)}</h1>
      <p class="dim">{t(`ui.home.level_${level}_text`)}</p>
    </div>
    <button class="btn quiet small" disabled={app.busy} onclick={() => { app.estimated = false; app.reset(); estimate(); }}>
      <Icon name="refresh" size={14} /> {t("ui.home.recheck")}
    </button>
  </header>

  <div class="grid">
    <section class="card health {level}">
      <div class="ring">
        <svg viewBox="0 0 100 100" aria-hidden="true">
          <defs>
            <linearGradient id="hg" x1="0" y1="0" x2="1" y2="1">
              <stop offset="0" stop-color="var(--a)" />
              <stop offset="1" stop-color="var(--b)" />
            </linearGradient>
          </defs>
          <circle cx="50" cy="50" r="44" class="track" />
          <circle cx="50" cy="50" r="44" class="value" stroke-dasharray="{(C * score) / 100} {C}" />
        </svg>
        <div class="ring-text">
          <span class="score num">{score}</span>
          <span class="lvl">{t(`ui.home.level_${level}`)}</span>
        </div>
      </div>
      <ul class="checks">
        {#each checks as c (c.icon)}
          <li class:ok={c.ok}>
            <span class="dot"><Icon name={c.ok ? "check" : "alert"} size={13} /></span>
            <span class="c-text">{c.text}</span>
            {#if c.action}<button class="link small" onclick={c.action.run}>{c.action.label}</button>{/if}
          </li>
        {/each}
      </ul>
    </section>

    <section class="card junk" data-section="clean">
      <div class="card-head">
        <h2>{t("ui.home.to_clean")}</h2>
        {#if app.mode}<span class="faint small">{t("ui.home.with_mode", { mode: app.mode.name })}</span>{/if}
      </div>
      {#if scanning}
        <div class="scanning">
          <span class="spinner"></span>
          <span class="dim">{t("ui.home.estimating")}</span>
          <div class="meter"><div style:width="{scanPct}%"></div></div>
        </div>
      {:else if app.report}
        <div class="big num">{junkParts[0]}<span class="unit">{junkParts[1]}</span></div>
        <div class="cats">
          {#each byCat as x (x.c)}
            <button class="cat" onclick={() => (app.view = "clean")}>
              <span class="cat-ic"><Icon name={CATEGORY_ICON[x.c]} size={14} /></span>
              <span class="cat-name">{categoryLabel(x.c)}</span>
              <span class="cat-bar"><span style:width="{(x.bytes / maxCat) * 100}%"></span></span>
              <span class="num cat-size">{formatBytes(x.bytes)}</span>
            </button>
          {:else}
            <p class="faint">{t("ui.home.check_no_junk")}</p>
          {/each}
        </div>
      {:else}
        <p class="faint">{t("ui.home.no_estimate")}</p>
      {/if}
    </section>
  </div>

  <section class="block">
    <h2>{t("ui.home.drives")}</h2>
    <div class="drives">
      {#each drives as d (d.root)}
        {@const p = d.total_bytes ? (d.total_bytes - d.free_bytes) / d.total_bytes : 0}
        <button
          class="drive"
          onclick={() => openDrive(d)}
          oncontextmenu={(e) => menu.show(e, [{ label: t("ui.menu.analyze"), icon: "pie", action: () => openDrive(d) }])}
          title={t("ui.home.analyze_drive")}
        >
          <FileIcon key="drive:{d.root}" fallback="drive" size={26} />
          <span class="d-main">
            <span class="d-name ellipsis">{d.label || t("ui.common.local_disk")} <span class="faint">({d.root.replace("\\", "")})</span></span>
            <span class="d-bar"><span style:width="{p * 100}%" class:full={p > 0.9}></span></span>
            <span class="faint small num">{t("ui.space.free_of", { free: formatBytes(d.free_bytes), total: formatBytes(d.total_bytes) })}</span>
          </span>
        </button>
      {/each}
    </div>
  </section>

  <div class="grid lower">
    <section class="block">
      <div class="card-head">
        <h2>{t("ui.home.activity")}</h2>
        <button class="link small" onclick={() => (showLog = true)}>{t("ui.log.open")}</button>
      </div>
      {#if recent.length}
        <ul class="recent">
          {#each recent as r, i (i)}
            {@const [dir, name] = splitPath(r.path)}
            <li title={r.path}>
              <Icon name={r.permanent ? "alert" : "trash"} size={13} />
              <span class="r-main">
                <span class="ellipsis">{name || r.path}</span>
                <span class="faint small ellipsis">{dir} · {formatDate(r.timestamp)}</span>
              </span>
              <span class="num small">{formatBytes(r.bytes)}</span>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="faint small">{t("ui.log.empty")}</p>
      {/if}
    </section>

    <section class="block">
      <div class="card-head">
        <h2>{t("ui.home.freed")}</h2>
        <span class="faint small num">{t(history.length === 1 ? "ui.home.freed_total_one" : "ui.home.freed_total", { size: formatBytes(totalFreed), n: history.length })}</span>
      </div>
      <div class="weeks" role="img" aria-label={t("ui.home.freed")}>
        {#each weeks as w, i (i)}
          <span class="wk" title={formatBytes(w)}><span style:height="{Math.max((w / maxWeek) * 100, w > 0 ? 6 : 2)}%"></span></span>
        {/each}
      </div>
      <div class="wk-labels faint small"><span>{tn("ui.home.weeks_ago", 8)}</span><span>{t("ui.home.this_week")}</span></div>
    </section>
  </div>
</div>

<Dock>
  {#snippet left()}
    {#if lastClean}
      <div class="hist">
        <span class="faint small">{t("ui.home.last_clean")}</span>
        <span class="small"><strong class="num">{formatBytes(lastClean.freed_bytes)}</strong> · {formatDate(lastClean.timestamp)}</span>
      </div>
    {:else}
      <span class="faint small">{t("ui.home.never")}</span>
    {/if}
  {/snippet}
  {#snippet center()}
    <ActionButton label={t("ui.home.quick_clean")} disabled={app.busy || app.modes.length === 0} onclick={quickClean} />
  {/snippet}
</Dock>

{#if showLog}
  <DeletionLog onclose={() => (showLog = false)} />
{/if}

<style>
  .home { flex: 1; min-height: 0; overflow: auto; padding: 0 32px 16px; display: flex; flex-direction: column; gap: 18px; }
  .head { display: flex; justify-content: space-between; align-items: flex-end; gap: 16px; }
  .head h1 { font-size: 26px; }
  h2 { font-size: 14px; }
  .small { font-size: 12px; }
  .grid { display: grid; grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr); gap: 14px; }
  .card, .block { background: var(--raise); border: 1px solid var(--line); border-radius: 16px; padding: 18px; min-width: 0; }
  .card-head { display: flex; justify-content: space-between; align-items: baseline; gap: 12px; margin-bottom: 12px; }

  .health { display: flex; gap: 20px; align-items: center; }
  .health.great, .health.good { --st: var(--ok); }
  .health.fair { --st: var(--warn); }
  .health.bad { --st: var(--bad); }
  .ring { position: relative; width: 136px; height: 136px; flex-shrink: 0; }
  .ring svg { width: 100%; height: 100%; transform: rotate(-90deg); }
  .track { fill: none; stroke: var(--line-2); stroke-width: 8; }
  .value { fill: none; stroke: url(#hg); stroke-width: 8; stroke-linecap: round; transition: stroke-dasharray 0.8s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .health.fair .value, .health.bad .value { stroke: var(--st); }
  .ring-text { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; }
  .score { font-family: var(--font-display); font-size: 38px; font-weight: 600; line-height: 1; }
  .lvl { font-size: 12px; color: var(--st); font-weight: 600; margin-top: 4px; }
  .checks { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; flex: 1; min-width: 0; font-size: 13px; }
  .checks li { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .dot { width: 22px; height: 22px; border-radius: 50%; display: grid; place-items: center; flex-shrink: 0; color: var(--warn); background: color-mix(in srgb, var(--warn) 14%, transparent); }
  .checks li.ok .dot { color: var(--ok); background: color-mix(in srgb, var(--ok) 14%, transparent); }
  .c-text { flex: 1; min-width: 0; }

  .junk { display: flex; flex-direction: column; }
  .big { font-family: var(--font-display); font-size: 40px; font-weight: 600; letter-spacing: -0.02em; line-height: 1.05; margin-bottom: 10px; }
  .unit { font-size: 0.45em; font-weight: 500; color: var(--text-2); margin-left: 6px; }
  .cats { display: flex; flex-direction: column; gap: 2px; }
  .cat { display: grid; grid-template-columns: 24px 96px 1fr 70px; gap: 10px; align-items: center; padding: 5px 6px; border: 0; background: none; border-radius: 8px; cursor: pointer; text-align: left; font-size: 13px; }
  .cat:hover { background: var(--hover); }
  .cat-ic { width: 24px; height: 24px; border-radius: 7px; display: grid; place-items: center; color: var(--a); background: color-mix(in srgb, var(--a) 14%, transparent); }
  .cat-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cat-bar { height: 5px; border-radius: 99px; background: var(--line); overflow: hidden; }
  .cat-bar > span { display: block; height: 100%; border-radius: 99px; background: linear-gradient(90deg, var(--a), var(--b)); }
  .cat-size { text-align: right; font-weight: 600; }
  .scanning { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; padding: 16px 0; }
  .spinner { width: 26px; height: 26px; border-radius: 50%; border: 3px solid var(--line-2); border-top-color: var(--a); animation: spin 0.8s linear infinite; }
  .meter { width: min(220px, 80%); height: 4px; border-radius: 99px; background: var(--line-2); overflow: hidden; }
  .meter > div { height: 100%; background: linear-gradient(90deg, var(--a), var(--b)); transition: width 0.25s; }

  .block h2 { margin-bottom: 0; }
  .block > h2 { margin-bottom: 12px; }
  .drives { display: grid; grid-template-columns: repeat(auto-fill, minmax(230px, 1fr)); gap: 8px; }
  .drive { display: flex; align-items: center; gap: 12px; padding: 10px; border: 0; background: none; border-radius: 12px; cursor: pointer; text-align: left; min-width: 0; }
  .drive:hover { background: var(--hover); }
  .d-main { flex: 1; display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .d-name { font-weight: 600; font-size: 13px; }
  .d-bar { height: 5px; border-radius: 99px; background: var(--line-2); overflow: hidden; }
  .d-bar > span { display: block; height: 100%; background: linear-gradient(90deg, var(--a), var(--b)); border-radius: 99px; }
  .d-bar > span.full { background: linear-gradient(90deg, var(--warn), var(--bad)); }

  .recent { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .recent li { display: flex; align-items: center; gap: 10px; color: var(--text-3); min-width: 0; }
  .r-main { flex: 1; display: flex; flex-direction: column; min-width: 0; color: var(--text); font-size: 13px; }
  .recent .num { color: var(--text); font-weight: 600; }
  .weeks { display: grid; grid-template-columns: repeat(8, 1fr); gap: 6px; height: 96px; align-items: end; }
  .wk { height: 100%; display: flex; align-items: flex-end; }
  .wk > span { display: block; width: 100%; border-radius: 6px 6px 3px 3px; background: linear-gradient(180deg, var(--b), var(--a)); opacity: 0.85; }
  .wk-labels { display: flex; justify-content: space-between; margin-top: 6px; }
  .hist { display: flex; flex-direction: column; gap: 1px; }

  @media (max-width: 1100px) {
    .grid { grid-template-columns: 1fr; }
  }
  @media (max-width: 720px) {
    .health { flex-direction: column; align-items: stretch; }
    .ring { align-self: center; }
    .cat { grid-template-columns: 24px 80px 1fr 64px; }
  }
</style>
