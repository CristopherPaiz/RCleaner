<script lang="ts">
  // Resumen, al estilo de Configuración › Almacenamiento de Windows 11:
  // 1. El disco del sistema: una barra (usado, lo que se puede liberar sin
  //    riesgo, libre) y dos botones.
  // 2. Sugerencias en filas sencillas, sólo cuando hay algo que hacer.
  // 3. Otras unidades y una línea de historial.
  // El análisis rápido es de sólo lectura y alimenta también Limpieza.
  import { onMount } from "svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { api, type Drive, type HistoryEntry, type HomeStatus } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { space } from "../lib/space.svelte";
  import { update } from "../lib/update.svelte";
  import { formatAge, formatBytes, formatDate } from "../lib/format";
  import Icon from "../components/Icon.svelte";
  import FileIcon from "../components/FileIcon.svelte";

  let drives = $state<Drive[]>([]);
  let history = $state<HistoryEntry[]>([]);
  let bin = $state<{ bytes: number; items: number } | null>(null);
  let status = $state<HomeStatus | null>(null);

  onMount(() => {
    void api.drives().then((d) => (drives = d));
    void api.history().then((h) => (history = h));
    void api.recycleBin().then((b) => (bin = b));
    void api.homeStatus().then((s) => (status = s)).catch(() => {});
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

  const GB = 1024 ** 3;
  const main = $derived(drives.find((d) => d.root.toUpperCase().startsWith("C:")) ?? drives[0]);
  const others = $derived(drives.filter((d) => d !== main));
  const freePct = $derived(main && main.total_bytes ? main.free_bytes / main.total_bytes : 1);
  const lastClean = $derived(history.at(-1));
  const daysSince = $derived(lastClean ? Math.floor((Date.now() / 1000 - lastClean.timestamp) / 86400) : null);
  const totalFreed = $derived(history.reduce((s, h) => s + h.freed_bytes, 0));

  // Lo que se limpiaría con la selección por defecto.
  const scanning = $derived(app.phase === "scanning");
  const findings = $derived(app.report?.findings ?? []);
  const junk = $derived(findings.filter((f) => app.selected.has(f.id)).reduce((s, f) => s + f.bytes, 0));
  const scanPct = $derived(app.progress.total ? Math.round((app.progress.done / app.progress.total) * 100) : 0);
  /** Lo que sólo se puede limpiar como administrador. */
  const adminBytes = $derived(
    app.info?.is_admin ? 0 : findings.filter((f) => f.requires_admin).reduce((s, f) => s + f.bytes, 0),
  );

  // Barra del disco: usado (sin lo liberable), liberable y libre.
  const used = $derived(main ? main.total_bytes - main.free_bytes : 0);
  const pct = (b: number) => (main && main.total_bytes ? (b / main.total_bytes) * 100 : 0);
  const junkShown = $derived(Math.min(junk, used));

  interface Rec {
    id: string;
    warn?: boolean;
    icon: string;
    title: string;
    text: string;
    run: () => void;
  }
  const recs = $derived.by(() => {
    const out: Rec[] = [];
    if (main && freePct < 0.15) {
      out.push({
        id: "disk", warn: true, icon: "drive",
        title: t("ui.home.rec_disk", { drive: main.root.replace("\\", ""), pct: Math.round(freePct * 100) }),
        text: t("ui.home.rec_disk_text"),
        run: () => openDrive(main),
      });
    }
    // Los restos pequeños de una actualización no merecen aviso.
    if (status && status.previous_windows_bytes >= 500 * 1024 ** 2) {
      out.push({
        id: "winold", icon: "layers",
        title: t("ui.home.rec_winold", { size: formatBytes(status.previous_windows_bytes) }),
        text: t(app.info?.is_admin ? "ui.home.rec_winold_text" : "ui.home.rec_winold_admin"),
        run: () => openMode("profundo"),
      });
    }
    if (bin && bin.bytes >= GB) {
      out.push({
        id: "bin", icon: "trash",
        title: t("ui.home.rec_bin", { size: formatBytes(bin.bytes) }),
        text: t("ui.home.rec_bin_text"),
        run: openBin,
      });
    }
    if (status && status.startup_apps >= 6) {
      out.push({
        id: "startup", icon: "rocket",
        title: tn("ui.home.rec_startup", status.startup_apps),
        text: t("ui.home.rec_startup_text"),
        run: () => (app.view = "startup"),
      });
    }
    if (adminBytes >= 100 * 1024 ** 2) {
      out.push({
        id: "admin", icon: "shield",
        title: t("ui.home.rec_admin", { size: formatBytes(adminBytes) }),
        text: t("ui.home.rec_admin_text"),
        run: elevate,
      });
    }
    if (update.info) {
      out.push({
        id: "update", icon: "up",
        title: t("ui.update.title", { v: update.info.version }),
        text: t("ui.home.rec_update_text"),
        run: () => (update.open = true),
      });
    }
    return out;
  });

  const statusText = $derived(
    recs.some((r) => r.warn) ? t("ui.home.status_warn") : junk >= GB ? t("ui.home.status_junk") : t("ui.home.status_ok"),
  );

  /** La papelera está en el análisis actual o, si no, en el modo Rápido. */
  function openBin() {
    if (findings.some((f) => f.module_id === "system.recycle_bin")) app.view = "clean";
    else openMode("rapido");
  }

  function openDrive(d: Drive) {
    app.view = "space";
    void space.scan(d.root);
  }

  function openMode(id: string) {
    const mode = app.modes.find((m) => m.id === id);
    if (mode) app.openMode(mode, true);
  }

  async function elevate() {
    if (!confirm(t("ui.admin.confirm"))) return;
    try {
      await api.restartAsAdmin();
    } catch (e) {
      app.error = String(e);
    }
  }

  function quickClean() {
    if (app.report && app.selected.size > 0) app.view = "clean";
    else {
      const mode =
        app.modes.find((m) => m.id === app.settings.default_mode) ?? app.modes.find((m) => m.id === "rapido") ?? app.modes[0];
      if (mode) app.openMode(mode, true);
    }
  }

  function recheck() {
    app.estimated = false;
    app.reset();
    estimate();
  }
</script>

<div class="home">
  <header class="head">
    <div>
      <h1>{t("ui.nav.home")}</h1>
      <p class="dim">{statusText}</p>
    </div>
    <button class="btn quiet small" disabled={app.busy} onclick={recheck}>
      <Icon name="refresh" size={14} /> {t("ui.home.recheck")}
    </button>
  </header>

  {#if main}
    <section class="card">
      <div class="drive-head">
        <FileIcon key="drive:{main.root}" fallback="drive" size={20} />
        <span class="d-title">{main.label || t("ui.home.local_disk")} ({main.root.replace("\\", "")})</span>
        <span class="faint num">{t("ui.home.used_of", { used: formatBytes(used), total: formatBytes(main.total_bytes) })}</span>
      </div>
      <div class="bar" class:low={freePct < 0.15}>
        <span class="used" style:width="{pct(used - junkShown)}%"></span>
        {#if junkShown > 0}<span class="junk" style:width="{pct(junkShown)}%"></span>{/if}
      </div>
      <div class="legend small">
        {#if scanning}
          <span class="faint">{t("ui.home.estimating")} <span class="num">{scanPct} %</span></span>
        {:else if app.report}
          {#if junk > 0}
            <span><i class="sw junk"></i>{t("ui.home.can_free_safe", { size: formatBytes(junk) })}</span>
          {:else}
            <span class="faint">{t("ui.home.nothing")}</span>
          {/if}
        {/if}
        <span class="faint num free">{t("ui.home.free_n", { size: formatBytes(main.free_bytes) })}</span>
      </div>
      <div class="acts">
        <button class="btn accent" disabled={app.busy || app.modes.length === 0} onclick={quickClean}>
          {t("ui.home.quick_clean")}
        </button>
        <button class="btn" disabled={!app.report || scanning} onclick={() => (app.view = "clean")}>
          {t("ui.home.see_what")}
        </button>
      </div>
    </section>
  {/if}

  <section class="list">
    <h2>{t("ui.home.recs")}</h2>
    {#each recs as r (r.id)}
      <button class="row" class:warn={r.warn} onclick={r.run}>
        <span class="ic"><Icon name={r.icon} size={18} /></span>
        <span class="main">
          <span class="title">{r.title}</span>
          <span class="faint small">{r.text}</span>
        </span>
        <Icon name="chevron" size={16} />
      </button>
    {:else}
      <div class="row ok">
        <span class="ic"><Icon name="check" size={18} /></span>
        <span class="main">
          <span class="title">{t("ui.home.all_good")}</span>
          <span class="faint small">{t("ui.home.all_good_text")}</span>
        </span>
      </div>
    {/each}
  </section>

  {#if others.length}
    <section class="list">
      <h2>{t("ui.home.other_drives")}</h2>
      {#each others as d (d.root)}
        {@const p = d.total_bytes ? (d.total_bytes - d.free_bytes) / d.total_bytes : 0}
        <button class="row drive" onclick={() => openDrive(d)} title={t("ui.home.analyze_drive")}>
          <FileIcon key="drive:{d.root}" fallback="drive" size={18} />
          <span class="title">{d.label || t("ui.home.local_disk")} ({d.root.replace("\\", "")})</span>
          <span class="mini" class:low={p > 0.85}><span style:width="{p * 100}%"></span></span>
          <span class="faint small num">{t("ui.home.free_n", { size: formatBytes(d.free_bytes) })}</span>
          <Icon name="chevron" size={16} />
        </button>
      {/each}
    </section>
  {/if}

  <p class="foot faint small">
    <Icon name="shield" size={13} />
    {t("ui.home.safe_note")}
    {#if history.length && lastClean && daysSince !== null}
      · {t("ui.home.total_freed")}: <span class="num">{formatBytes(totalFreed)}</span> {tn("ui.home.cleanups", history.length)}
      · <span title={formatDate(lastClean.timestamp)}>{t("ui.home.last_clean")}: {formatAge(daysSince)}</span>
    {/if}
  </p>
</div>

<style>
  .home { flex: 1; min-height: 0; overflow: auto; padding: 0 32px 20px; display: flex; flex-direction: column; gap: 20px; max-width: 880px; width: 100%; margin: 0 auto; }
  .home > * { flex-shrink: 0; }
  .head { display: flex; justify-content: space-between; align-items: flex-end; gap: 16px; }
  h2 { font-size: 13px; font-weight: 600; color: var(--text-2); margin: 0 0 4px; }
  .small { font-size: 12px; }

  .card { background: var(--raise); border: 1px solid var(--line); border-radius: 12px; padding: 18px 20px; display: flex; flex-direction: column; gap: 10px; }
  .drive-head { display: flex; align-items: center; gap: 10px; }
  .d-title { flex: 1; font-weight: 600; font-size: 15px; }
  .bar { display: flex; height: 10px; border-radius: 99px; overflow: hidden; background: var(--line-2); }
  .bar .used { background: color-mix(in srgb, var(--text-2) 55%, transparent); }
  .bar.low .used { background: var(--warn); }
  .bar .junk { background: var(--a); min-width: 4px; }
  .legend { display: flex; align-items: center; gap: 12px; min-height: 18px; }
  .legend .free { margin-left: auto; }
  .sw { display: inline-block; width: 9px; height: 9px; border-radius: 3px; margin-right: 6px; vertical-align: 0; }
  .sw.junk { background: var(--a); }
  .acts { display: flex; gap: 8px; margin-top: 6px; }

  .list { display: flex; flex-direction: column; }
  .row {
    display: flex; align-items: center; gap: 14px; padding: 12px 6px; border: 0; border-top: 1px solid var(--line);
    background: none; font: inherit; color: var(--text-2); text-align: left; cursor: pointer;
  }
  .list > h2 + .row { border-top: 0; }
  div.row { cursor: default; }
  button.row:hover { background: var(--hover); }
  .ic { color: var(--text-2); display: grid; place-items: center; width: 22px; }
  .row.warn .ic { color: var(--warn); }
  .row.ok .ic { color: var(--ok); }
  .main { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .title { color: var(--text); font-size: 13.5px; font-weight: 500; }
  .drive .title { flex: 1; }
  .mini { width: 120px; height: 5px; border-radius: 99px; background: var(--line-2); overflow: hidden; }
  .mini > span { display: block; height: 100%; background: color-mix(in srgb, var(--text-2) 55%, transparent); }
  .mini.low > span { background: var(--warn); }

  .foot { margin-top: auto; padding-top: 4px; line-height: 1.6; }
  .foot :global(svg) { display: inline-block; vertical-align: -2px; margin-right: 4px; }
</style>
