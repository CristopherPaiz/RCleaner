<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { bulkPick } from "../lib/results";
  import { api, type Finding } from "../lib/api";
  import { formatBytes, formatCount, modeColors, splitBytes, splitPath } from "../lib/format";
  import ResultList from "../components/ResultList.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import Glyph from "../components/Glyph.svelte";
  import OrbButton from "../components/OrbButton.svelte";
  import ActionButton from "../components/ActionButton.svelte";
  import Dock from "../components/Dock.svelte";
  import Icon from "../components/Icon.svelte";
  import { fade } from "svelte/transition";
  import { morph } from "../lib/motion";
  import { i18n, t, tn } from "../lib/i18n.svelte";
  import { activity } from "../lib/activity.svelte";
  import ExpandList from "../components/ExpandList.svelte";

  let confirming = $state(false);
  let query = $state("");
  let sortBy = $state<"size" | "age">("size");

  const report = $derived(app.report);
  const findings = $derived(report?.findings ?? []);
  const chosen = $derived<Finding[]>(findings.filter((f) => app.selected.has(f.id)));
  const chosenBytes = $derived(chosen.reduce((s, f) => s + f.bytes, 0));
  const totalBytes = $derived(findings.reduce((s, f) => s + f.bytes, 0));
  const percent = $derived(app.progress.total ? Math.round((app.progress.done / app.progress.total) * 100) : 0);
  const needsAdmin = $derived(!app.info?.is_admin && findings.some((f) => f.requires_admin));
  const moduleName = $derived(app.modules.find((m) => m.id === app.progress.current)?.name ?? "");

  const freed = $derived(app.cleanReport?.outcomes.reduce((s, o) => s + o.freed_bytes, 0) ?? 0);
  const skipped = $derived(app.cleanReport?.outcomes.reduce((s, o) => s + o.skipped_items, 0) ?? 0);
  const problems = $derived(
    (app.cleanReport?.outcomes ?? [])
      .filter((o) => o.errors.length > 0)
      .map((o) => ({ label: findings.find((f) => f.id === o.finding_id)?.label ?? o.finding_id, errors: o.errors })),
  );
  const cleaned = $derived(
    (app.cleanReport?.outcomes ?? [])
      .filter((o) => o.freed_bytes > 0 || o.removed_items > 0)
      .map((o) => ({ o, f: findings.find((f) => f.id === o.finding_id) }))
      .sort((a, b) => b.o.freed_bytes - a.o.freed_bytes),
  );
  const big = $derived(splitBytes(chosenBytes));
  /** Unidad de una ruta ("C:"), en mayúsculas. */
  const driveOf = (path?: string) => (path && /^[a-z]:/i.test(path) ? path.slice(0, 2).toUpperCase() : null);
  /** Dónde busca la limpieza: el disco del sistema y los de tus proyectos. */
  const scanDrives = $derived.by(() => {
    const set = new Set<string>();
    const sys = driveOf(app.info?.home);
    if (sys) set.add(sys);
    for (const r of app.info?.dev_roots ?? []) {
      const d = driveOf(r);
      if (d) set.add(d);
    }
    return [...set];
  });
  /** Lo marcado, sumado por unidad. */
  const chosenDrives = $derived.by(() => {
    const map = new Map<string, number>();
    for (const f of chosen) {
      const d = driveOf(f.path) ?? scanDrives[0] ?? "C:";
      map.set(d, (map.get(d) ?? 0) + f.bytes);
    }
    return [...map].sort((a, b) => b[1] - a[1]);
  });
  const freedParts = $derived(splitBytes(freed));

  function select(which: "safe" | "all" | "none") {
    app.selected.clear();
    if (which === "none") return;
    for (const f of findings) {
      if (!app.selectable(f)) continue;
      // Datos del usuario (cookies, historial, modelos…) sólo a mano.
      if (bulkPick(f, which)) app.selected.add(f.id);
    }
  }

  // Píldora que se desliza bajo el modo elegido (en vez de saltar de botón en botón).
  let seg = $state<HTMLDivElement>();
  let pill = $state<{ x: number; y: number; w: number; h: number } | null>(null);
  let pillReady = $state(false);
  function placePill() {
    const on = seg?.querySelector<HTMLElement>("button.on");
    if (!on) return (pill = null);
    pill = { x: on.offsetLeft, y: on.offsetTop, w: on.offsetWidth, h: on.offsetHeight };
  }
  $effect(() => {
    void app.mode?.id;
    if (!seg) return;
    placePill();
    const ro = new ResizeObserver(placePill);
    ro.observe(seg);
    requestAnimationFrame(() => (pillReady = true));
    return () => ro.disconnect();
  });

  async function doClean() {
    confirming = false;
    await app.clean();
  }
</script>

{#if app.phase === "idle" || (app.phase === "scanning" && !report)}
  <!-- Inicio y análisis: el botón central es el icono del modo -->
  {@const scanning = app.phase === "scanning"}
  <div class="stage">
    <OrbButton
      icon={app.mode?.icon || "sparkles"}
      label={t("ui.clean.analyze")}
      busy={scanning}
      percent={scanning ? percent : null}
      disabled={!app.mode}
      onclick={() => (scanning ? api.cancel() : app.scan())}
    />
    <!-- Alto fijo: el botón nunca se mueve al cambiar de modo ni al analizar -->
    <div class="info">
      {#if scanning}
        <div class="blk" transition:fade={{ duration: 220 }}>
          <h1>{t("ui.clean.analyzing")}</h1>
          <p class="dim lead">{moduleName || t("ui.clean.preparing")}</p>
          <p class="faint num small">{t("ui.clean.progress", { done: app.progress.done, total: app.progress.total })}</p>
        </div>
      {:else}
        <div class="blk" transition:fade={{ duration: 220 }}>
          <h1>{t("ui.clean.title")}</h1>
          <p class="dim lead">{t("ui.clean.lead")}</p>
          {#if scanDrives.length > 0}
            <p class="where faint small">
              <Icon name="drive" size={14} />
              {t("ui.clean.where", { drives: scanDrives.join(", ") })}
            </p>
          {/if}
          <div class="modes" role="tablist" aria-label={t("ui.clean.mode")}>
            <div class="segmented" bind:this={seg}>
              {#if pill}
                <span
                  class="pill"
                  class:ready={pillReady}
                  style:transform="translate({pill.x}px, {pill.y}px)"
                  style:width="{pill.w}px"
                  style:height="{pill.h}px"
                ></span>
              {/if}
              {#each app.modes as m (m.id)}
                {@const [ca] = modeColors(m.icon)}
                <button
                  class:on={app.mode?.id === m.id}
                  role="tab"
                  aria-selected={app.mode?.id === m.id}
                  style:--mc={ca}
                  onclick={() => app.setMode(m)}
                >
                  <span class="mi"><Icon name={m.icon || "star"} size={15} /></span>
                  <span class="nm" data-t={m.name}>{m.name}</span>
                </button>
              {/each}
            </div>
            <div class="desc">
              {#key app.mode?.id}
                <p class="faint" in:morph={{ scale: 0.96, blur: 4, duration: 320 }} out:fade={{ duration: 140 }}>
                  {app.mode?.description ?? ""}
                </p>
              {/key}
            </div>
          </div>
        </div>
      {/if}
    </div>
  </div>
{:else if app.phase === "done" && app.cleanReport}
  <div class="stage">
    <Glyph icon="check" success />
    <h1 class="hero-num num">{freedParts[0]}<span class="unit">{freedParts[1]}</span></h1>
    <p class="dim lead">
      {t("ui.clean.freed", { n: app.cleanReport.outcomes.length, secs: (app.cleanReport.elapsed_ms / 1000).toLocaleString(i18n.lang, { maximumFractionDigits: 1 }) })}
    </p>
    {#if skipped > 0}
      <p class="faint small">{tn("ui.clean.skipped", skipped)}</p>
    {/if}
    {#if cleaned.length > 0}
      <div class="cleaned">
        <ExpandList items={cleaned} limit={5}>
          {#snippet row(c)}
            <span class="ellipsis" title={c.f?.path ?? ""}>{c.f?.label ?? c.o.finding_id}</span>
            <span class="num faint">{formatBytes(c.o.freed_bytes)}</span>
          {/snippet}
        </ExpandList>
      </div>
    {/if}
    {#if problems.length > 0}
      <details class="problems">
        <summary>{tn("ui.clean.problems", problems.length)}</summary>
        {#each problems as p (p.label)}
          <div class="problem">
            <strong>{p.label}</strong>
            {#each p.errors.slice(0, 5) as e, i (i)}<div class="mono faint">{e}</div>{/each}
          </div>
        {/each}
      </details>
    {/if}
  </div>
  <Dock>
    {#snippet center()}
      <ActionButton label={t("ui.common.done")} onclick={() => app.reset()} />
    {/snippet}
  </Dock>
{:else if report}
  <!-- Resultados -->
  <header class="results-head">
    <div class="total">
      <div class="hero-num num">{big[0]}<span class="unit">{big[1]}</span></div>
      <p class="dim">
        {t("ui.clean.selected_of", { total: formatBytes(totalBytes), mode: app.mode?.name ?? "" })}
      </p>
      {#if chosenDrives.length > 0}
        <div class="drives" aria-label={t("ui.clean.on_drives")}>
          {#each chosenDrives as [d, bytes] (d)}
            <span class="drive-chip"><Icon name="drive" size={13} /> {d} <span class="num faint">{formatBytes(bytes)}</span></span>
          {/each}
        </div>
      {/if}
    </div>
    <div class="tools">
      <label class="field search">
        <Icon name="search" size={15} />
        <input placeholder={t("ui.common.filter")} bind:value={query} />
      </label>
      <div class="segmented">
        <button class:on={sortBy === "size"} onclick={() => (sortBy = "size")}>{t("ui.clean.sort_size")}</button>
        <button class:on={sortBy === "age"} onclick={() => (sortBy = "age")}>{t("ui.clean.sort_age")}</button>
      </div>
    </div>
  </header>

  <div class="subbar">
    <span class="faint">{t("ui.clean.mark")}</span>
    <button class="btn quiet small" onclick={() => select("safe")}>{t("ui.clean.mark_safe")}</button>
    <button class="btn quiet small" onclick={() => select("all")}>{t("ui.clean.mark_all")}</button>
    <button class="btn quiet small" onclick={() => select("none")}>{t("ui.clean.mark_none")}</button>
    {#if needsAdmin}
      <span class="hint"><Icon name="shield" size={14} /> {t("ui.clean.needs_admin")}</span>
    {/if}
  </div>

  {#if app.phase === "cleaning"}
    <div class="cleaning">
      <div class="meter wide"><div style:width="{percent}%"></div></div>
      <span class="faint num">{t("ui.clean.cleaning", { done: app.progress.done, total: app.progress.total })}</span>
      {#if activity.now}
        {@const [dir, name] = splitPath(activity.now)}
        <span class="now mono" title={activity.now}><span class="faint">{dir}\</span>{name}</span>
      {/if}
    </div>
  {/if}

  <ResultList {report} {query} {sortBy} />

  <Dock>
    {#snippet left()}
      <button class="btn quiet" disabled={app.busy} onclick={() => app.reset()}>
        <Icon name="back" size={16} /> {t("ui.clean.restart")}
      </button>
    {/snippet}
    {#snippet center()}
      {#if app.phase === "cleaning"}
        <ActionButton label={t("ui.common.stop")} variant="stop" onclick={() => api.cancel()} />
      {:else}
        <ActionButton label={t("ui.clean.clean")} disabled={chosen.length === 0} onclick={() => (confirming = true)} />
      {/if}
    {/snippet}
    {#snippet right()}
      <div class="sel">
        <strong class="num">{formatBytes(chosenBytes)}</strong>
        <span class="faint">{tn("ui.common.items", chosen.length)}</span>
      </div>
    {/snippet}
  </Dock>
{/if}

{#if confirming}
  <ConfirmDialog findings={chosen} onconfirm={doClean} oncancel={() => (confirming = false)} />
{/if}

<style>
  .stage {
    flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: safe center;
    gap: 10px; padding: 0 32px; text-align: center; min-height: 0; overflow: auto;
  }
  .stage > * { flex-shrink: 0; }
  .stage h1 { font-size: 30px; margin-top: 26px; }
  .stage { padding-bottom: 40px; }
  .info { display: grid; width: 100%; min-height: 270px; }
  .blk { grid-area: 1 / 1; display: flex; flex-direction: column; align-items: center; gap: 10px; }

  .modes .segmented { position: relative; }
  .modes .segmented button { position: relative; z-index: 1; transition: color 0.2s; }
  .modes .segmented button.on { background: none; box-shadow: none; }
  .pill {
    position: absolute; left: 0; top: 0; border-radius: 8px; pointer-events: none;
    background: color-mix(in srgb, var(--a) 16%, var(--raise-2));
    box-shadow: 0 1px 0 var(--line-2) inset, 0 1px 2px rgb(0 0 0 / 0.25), 0 0 18px -6px color-mix(in srgb, var(--a) 60%, transparent);
  }
  .pill.ready { transition: transform 0.38s cubic-bezier(0.3, 0.9, 0.3, 1), width 0.38s cubic-bezier(0.3, 0.9, 0.3, 1); }
  .mi { color: var(--mc); display: inline-grid; }
  /* Reserva el ancho en negrita para que los botones no cambien de tamaño. */
  .nm { display: inline-grid; }
  .nm::after { content: attr(data-t); font-weight: 600; height: 0; visibility: hidden; overflow: hidden; }
  .lead { font-size: 15px; max-width: 56ch; }
  .small { font-size: 12px; }
  .modes { margin-top: 26px; display: flex; flex-direction: column; align-items: center; gap: 12px; max-width: 100%; }
  .desc { display: grid; width: 60ch; max-width: 100%; height: 4.5em; line-height: 1.5; font-size: 13px; }
  .desc p { grid-area: 1 / 1; margin: 0; }
  .modes .segmented { flex-wrap: wrap; justify-content: center; }
  .meter { width: 260px; height: 4px; border-radius: 99px; background: var(--line-2); overflow: hidden; margin-top: 14px; }
  .meter > div { height: 100%; background: linear-gradient(90deg, var(--a), var(--b)); transition: width 0.25s; }
  .meter.wide { width: 100%; margin: 0; }

  .hero-num { font-family: var(--font-display); font-size: 46px; font-weight: 600; letter-spacing: -0.02em; line-height: 1.05; }
  .unit { font-size: 0.45em; font-weight: 500; color: var(--text-2); margin-left: 6px; letter-spacing: 0; }
  .stage .hero-num { margin-top: 22px; }

  .problems { margin-top: 18px; text-align: left; max-width: 640px; width: 100%; }
  .problems summary { cursor: pointer; color: var(--warn); text-align: center; }
  .problem { margin: 10px 0; }
  .problem .mono { word-break: break-all; margin-top: 2px; }

  .where { display: inline-flex; align-items: center; gap: 6px; margin: -4px 0 10px; }
  .drives { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 6px; }
  .drive-chip { display: inline-flex; align-items: center; gap: 5px; padding: 2px 9px; border-radius: 999px; background: var(--raise); font-size: 12px; color: var(--text-2); }
  .results-head { display: flex; justify-content: space-between; align-items: flex-end; gap: 24px; padding: 4px 32px 10px; flex-wrap: wrap; }
  .tools { display: flex; gap: 8px; align-items: center; }
  .search { width: 220px; }
  .subbar { display: flex; align-items: center; gap: 4px; padding: 0 32px 6px; flex-wrap: wrap; }
  .subbar > .faint { font-size: 13px; margin-right: 4px; }
  .hint { margin-left: auto; display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--warn); }
  .cleaning { display: flex; align-items: center; gap: 14px; padding: 6px 32px 10px; font-size: 12px; }
  .cleaning span { white-space: nowrap; }
  .cleaning .meter.wide { width: 200px; flex-shrink: 0; }
  .cleaning .now { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .cleaned { width: min(520px, 100%); margin-top: 10px; text-align: left; display: flex; flex-direction: column; }
  .sel { display: flex; flex-direction: column; align-items: flex-end; }
  .sel strong { font-size: 16px; }
  .sel span { font-size: 12px; }
</style>
