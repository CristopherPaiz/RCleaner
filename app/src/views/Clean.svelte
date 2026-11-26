<script lang="ts">
  import { app } from "../lib/app.svelte";
  import { api, type Finding } from "../lib/api";
  import { formatBytes } from "../lib/format";
  import ResultList from "../components/ResultList.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import Icon from "../components/Icon.svelte";

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

  const freed = $derived(app.cleanReport?.outcomes.reduce((s, o) => s + o.freed_bytes, 0) ?? 0);
  const skipped = $derived(app.cleanReport?.outcomes.reduce((s, o) => s + o.skipped_items, 0) ?? 0);
  const problems = $derived(
    (app.cleanReport?.outcomes ?? [])
      .filter((o) => o.errors.length > 0)
      .map((o) => ({ label: findings.find((f) => f.id === o.finding_id)?.label ?? o.finding_id, errors: o.errors })),
  );

  function select(which: "safe" | "all" | "none") {
    app.selected.clear();
    if (which === "none") return;
    for (const f of findings) {
      if (!app.selectable(f)) continue;
      if (which === "all" || f.risk === "safe") app.selected.add(f.id);
    }
  }

  async function doClean() {
    confirming = false;
    await app.clean();
  }
</script>

<div class="view">
  <header>
    <div>
      <h1>{app.mode?.name ?? "Limpiar"}</h1>
      <p class="muted">{app.mode?.description ?? "Elige un modo en el menú."}</p>
    </div>
    <div class="actions">
      {#if app.busy}
        <button class="ghost" onclick={() => api.cancel()}>Cancelar</button>
      {:else}
        <button class="primary" onclick={() => app.scan()} disabled={!app.mode}>
          {report ? "Volver a analizar" : "Analizar"}
        </button>
      {/if}
    </div>
  </header>

  {#if app.phase === "scanning" || app.phase === "cleaning"}
    <section class="progress">
      <div class="bar"><div style:width="{percent}%"></div></div>
      <p class="muted">
        {app.phase === "scanning" ? "Analizando" : "Limpiando"} · {app.progress.done}/{app.progress.total} · {percent}%
        <span class="mono">{app.progress.current}</span>
      </p>
    </section>
  {:else if app.phase === "done" && app.cleanReport}
    <section class="summary">
      <h2>{formatBytes(freed)} liberados</h2>
      <p class="muted">
        {app.cleanReport.outcomes.length} elementos en {(app.cleanReport.elapsed_ms / 1000).toFixed(1)} s
        {#if skipped > 0}· {skipped} archivos omitidos (en uso o sin permiso){/if}
      </p>
      {#if problems.length > 0}
        <details>
          <summary>Ver {problems.length} elementos con avisos</summary>
          {#each problems as p (p.label)}
            <div class="problem">
              <strong>{p.label}</strong>
              {#each p.errors.slice(0, 5) as e, i (i)}<div class="mono">{e}</div>{/each}
            </div>
          {/each}
        </details>
      {/if}
      <button class="primary" onclick={() => app.scan()}>Analizar de nuevo</button>
    </section>
  {:else if report}
    {#if needsAdmin}
      <div class="hint">
        <Icon name="shield" size={16} />
        Hay elementos que requieren administrador. Usa "Reiniciar como administrador" en el menú para incluirlos.
      </div>
    {/if}
    <div class="toolbar">
      <label class="search">
        <Icon name="search" size={15} />
        <input placeholder="Filtrar…" bind:value={query} />
      </label>
      <div class="seg">
        <button class:on={sortBy === "size"} onclick={() => (sortBy = "size")}>Tamaño</button>
        <button class:on={sortBy === "age"} onclick={() => (sortBy = "age")}>Antigüedad</button>
      </div>
      <span class="spacer"></span>
      <span class="muted">Seleccionar:</span>
      <button class="ghost sm" onclick={() => select("safe")}>Seguros</button>
      <button class="ghost sm" onclick={() => select("all")}>Todo</button>
      <button class="ghost sm" onclick={() => select("none")}>Nada</button>
    </div>
    <ResultList {report} {query} {sortBy} />
  {:else}
    <section class="empty">
      <p>Pulsa <strong>Analizar</strong> para ver qué se puede limpiar. Analizar nunca borra nada.</p>
    </section>
  {/if}

  {#if app.phase === "results" && report}
    <footer>
      <div>
        <strong>{formatBytes(chosenBytes)}</strong>
        <span class="muted">seleccionados de {formatBytes(totalBytes)} · {chosen.length} elementos</span>
      </div>
      <button class="danger" disabled={chosen.length === 0} onclick={() => (confirming = true)}>Limpiar</button>
    </footer>
  {/if}
</div>

{#if confirming}
  <ConfirmDialog findings={chosen} onconfirm={doClean} oncancel={() => (confirming = false)} />
{/if}

<style>
  .view { display: flex; flex-direction: column; min-height: 0; flex: 1; }
  header { display: flex; justify-content: space-between; align-items: flex-start; gap: 24px; padding: 22px 28px 12px; }
  h1 { margin: 0 0 4px; font-size: 22px; }
  header p { margin: 0; max-width: 75ch; }
  .actions { display: flex; gap: 8px; flex-shrink: 0; }
  .progress, .summary, .empty { padding: 36px 28px; }
  .bar { height: 8px; background: var(--border); border-radius: 99px; overflow: hidden; }
  .bar > div { height: 100%; background: linear-gradient(90deg, var(--accent), var(--accent-2)); transition: width 0.2s; }
  .summary h2 { font-size: 32px; margin: 0 0 6px; color: var(--safe); }
  details { margin: 14px 0 18px; }
  summary { cursor: pointer; color: var(--warn); }
  .problem { margin: 10px 0; }
  .problem .mono { margin: 2px 0 0; color: var(--muted); word-break: break-all; }
  .hint { display: flex; align-items: center; gap: 8px; margin: 0 28px 10px; padding: 8px 12px; border-radius: 8px; background: var(--accent-bg); color: var(--accent); font-size: 13px; }
  .toolbar { display: flex; align-items: center; gap: 8px; padding: 0 28px 12px; flex-wrap: wrap; }
  .search { display: flex; align-items: center; gap: 6px; padding: 6px 10px; border: 1px solid var(--border); border-radius: 8px; background: var(--panel); color: var(--muted); }
  .search input { border: 0; outline: 0; background: none; color: var(--text); font: inherit; width: 200px; }
  .seg { display: flex; border: 1px solid var(--border); border-radius: 8px; overflow: hidden; }
  .seg button { border: 0; background: var(--panel); color: var(--muted); font: inherit; padding: 6px 12px; cursor: pointer; }
  .seg button.on { background: var(--accent-bg); color: var(--accent); font-weight: 600; }
  .spacer { flex: 1; }
  footer { display: flex; justify-content: space-between; align-items: center; padding: 14px 28px; border-top: 1px solid var(--border); background: var(--panel); }
  footer strong { font-size: 18px; margin-right: 8px; }
</style>
