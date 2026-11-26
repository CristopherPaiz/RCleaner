<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Drive, type HistoryEntry } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { formatBytes, formatDate } from "../lib/format";
  import Icon from "../components/Icon.svelte";

  let drives = $state<Drive[]>([]);
  let history = $state<HistoryEntry[]>([]);

  onMount(async () => {
    try {
      [drives, history] = await Promise.all([api.drives(), api.history()]);
    } catch (e) {
      app.error = String(e);
    }
  });

  const totalFreed = $derived(history.reduce((s, h) => s + h.freed_bytes, 0));
  const recent = $derived([...history].reverse().slice(0, 6));
</script>

<div class="page">
  <header>
    <h1>Inicio</h1>
    <p class="muted">Estado de tus discos y accesos rápidos a cada modo.</p>
  </header>

  <section class="drives">
    {#each drives as d (d.root)}
      {@const used = d.total_bytes - d.free_bytes}
      {@const pct = d.total_bytes ? Math.round((used / d.total_bytes) * 100) : 0}
      <div class="card drive">
        <div class="row">
          <Icon name="drive" />
          <strong>{d.label || "Disco local"} ({d.root.replace("\\", "")})</strong>
          <span class="muted right">{formatBytes(d.free_bytes)} libres de {formatBytes(d.total_bytes)}</span>
        </div>
        <div class="bar" class:warn={pct >= 85} class:crit={pct >= 95}>
          <div style:width="{pct}%"></div>
        </div>
        <span class="muted small">{pct}% usado</span>
      </div>
    {/each}
  </section>

  <h2>Analizar ahora</h2>
  <section class="modes">
    {#each app.modes as m (m.id)}
      <button class="card mode" disabled={app.busy} onclick={() => app.openMode(m, true)}>
        <span class="icon"><Icon name={m.icon || "circle"} size={20} /></span>
        <strong>{m.name}</strong>
        <span class="muted small">{m.description}</span>
      </button>
    {/each}
  </section>

  <h2>Historial</h2>
  {#if history.length === 0}
    <p class="muted">Aún no has limpiado nada con Clean-CRP.</p>
  {:else}
    <section class="card history">
      <div class="total">
        <span class="muted">Liberado en total</span>
        <strong>{formatBytes(totalFreed)}</strong>
        <span class="muted small">en {history.length} limpiezas</span>
      </div>
      <ul>
        {#each recent as h (h.timestamp)}
          <li>
            <span>{formatDate(h.timestamp)}</span>
            <span class="muted">{h.mode ?? "—"} · {h.findings} elementos</span>
            <strong>{formatBytes(h.freed_bytes)}</strong>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
</div>

<style>
  .page { padding: 22px 28px 32px; overflow: auto; }
  header h1 { margin: 0 0 4px; font-size: 22px; }
  header p { margin: 0 0 18px; }
  h2 { font-size: 15px; margin: 26px 0 10px; }
  .card { background: var(--panel); border: 1px solid var(--border); border-radius: 12px; padding: 14px 16px; }
  .drives { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 12px; }
  .row { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; }
  .right { margin-left: auto; font-size: 12px; }
  .small { font-size: 12px; }
  .bar { height: 8px; border-radius: 99px; background: var(--border); overflow: hidden; margin-bottom: 6px; }
  .bar > div { height: 100%; background: var(--accent); }
  .bar.warn > div { background: var(--warn); }
  .bar.crit > div { background: var(--danger); }
  .modes { display: grid; grid-template-columns: repeat(auto-fill, minmax(210px, 1fr)); gap: 12px; }
  button.mode { display: flex; flex-direction: column; gap: 6px; text-align: left; font: inherit; color: var(--text); cursor: pointer; }
  button.mode:hover:not(:disabled) { border-color: var(--accent); }
  button.mode:disabled { opacity: 0.5; cursor: default; }
  .icon { width: 36px; height: 36px; border-radius: 10px; display: grid; place-items: center; background: var(--accent-bg); color: var(--accent); }
  .history { display: grid; grid-template-columns: 200px 1fr; gap: 20px; }
  .total { display: flex; flex-direction: column; gap: 2px; }
  .total strong { font-size: 26px; color: var(--safe); }
  ul { list-style: none; margin: 0; padding: 0; }
  li { display: grid; grid-template-columns: 150px 1fr auto; gap: 12px; padding: 6px 0; border-bottom: 1px solid var(--border-soft); }
  li:last-child { border-bottom: 0; }
</style>
