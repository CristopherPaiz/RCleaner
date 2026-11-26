<script lang="ts">
  import { onMount } from "svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import { api, type Drive, type HistoryEntry } from "../lib/api";
  import { app, type View } from "../lib/app.svelte";
  import { formatBytes, formatDate, splitBytes } from "../lib/format";
  import ActionButton from "../components/ActionButton.svelte";
  import Dock from "../components/Dock.svelte";
  import Icon from "../components/Icon.svelte";

  let drives = $state<Drive[]>([]);
  let history = $state<HistoryEntry[]>([]);

  onMount(() => {
    void api.drives().then((d) => (drives = d));
    void api.history().then((h) => (history = h.slice(0, 4)));
  });

  // La unidad del sistema primero (la que más importa llenar).
  const main = $derived(drives.find((d) => d.root.toUpperCase().startsWith("C:")) ?? drives[0]);
  const others = $derived(drives.filter((d) => d !== main));
  const used = $derived(main ? main.total_bytes - main.free_bytes : 0);
  const pct = $derived(main && main.total_bytes ? used / main.total_bytes : 0);
  const free = $derived(splitBytes(main?.free_bytes ?? 0));
  const totalFreed = $derived(history.reduce((s, h) => s + h.freed_bytes, 0));

  // Anillo de uso: circunferencia del círculo r=44 en un viewBox de 100.
  const C = 2 * Math.PI * 44;

  const SHORTCUTS: { view: View; icon: string; title: string; text: string }[] = [
    { view: "clean", icon: "sparkles", title: "ui.nav.clean", text: "ui.home.clean_text" },
    { view: "space", icon: "pie", title: "ui.nav.space", text: "ui.home.space_text" },
    { view: "dupes", icon: "copy", title: "ui.nav.dupes", text: "ui.home.dupes_text" },
  ];

  function quickClean() {
    const mode =
      app.modes.find((m) => m.id === app.settings.default_mode) ?? app.modes.find((m) => m.id === "rapido") ?? app.modes[0];
    if (mode) app.openMode(mode, true);
  }
</script>

<div class="home">
  <section class="hero">
    {#if main}
      <div class="ring" class:tight={pct > 0.9}>
        <svg viewBox="0 0 100 100" aria-hidden="true">
          <defs>
            <linearGradient id="ring-g" x1="0" y1="0" x2="1" y2="1">
              <stop offset="0" stop-color="var(--a)" />
              <stop offset="1" stop-color="var(--b)" />
            </linearGradient>
          </defs>
          <circle cx="50" cy="50" r="44" class="track" />
          <circle cx="50" cy="50" r="44" class="value" stroke-dasharray="{C * pct} {C}" />
        </svg>
        <div class="ring-text">
          <span class="ring-num num">{free[0]}<small>{free[1]}</small></span>
          <span class="faint">{t("ui.home.free")}</span>
        </div>
      </div>
      <h1>{main.label || t("ui.common.local_disk")} <span class="faint">({main.root.replace("\\", "")})</span></h1>
      <p class="dim num">{t("ui.home.usage", { used: formatBytes(used), total: formatBytes(main.total_bytes), pct: Math.round(pct * 100) })}</p>
      {#if others.length}
        <div class="others">
          {#each others as d (d.root)}
            {@const p = d.total_bytes ? (d.total_bytes - d.free_bytes) / d.total_bytes : 0}
            <div class="mini" title="{d.label} {d.root}">
              <span class="faint">{d.root.replace("\\", "")}</span>
              <span class="mini-bar"><span style:width="{p * 100}%" class:full={p > 0.9}></span></span>
              <span class="faint num">{t("ui.home.free_n", { size: formatBytes(d.free_bytes) })}</span>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </section>

  <section class="shortcuts">
    {#each SHORTCUTS as s (s.view)}
      <button class="sc" data-section={s.view} onclick={() => (app.view = s.view)}>
        <span class="sc-ic"><Icon name={s.icon} size={20} /></span>
        <span class="sc-text">
          <strong>{t(s.title)}</strong>
          <span class="faint">{t(s.text)}</span>
        </span>
        <Icon name="chevron" size={16} />
      </button>
    {/each}
  </section>
</div>

<Dock>
  {#snippet left()}
    {#if history.length}
      <div class="hist">
        <span class="faint small">{t("ui.home.last_clean")}</span>
        <span class="small"><strong class="num">{formatBytes(history[0].freed_bytes)}</strong> · {formatDate(history[0].timestamp)}</span>
        {#if history.length > 1}<span class="faint small num">{t("ui.home.last_n", { size: formatBytes(totalFreed), n: history.length })}</span>{/if}
      </div>
    {:else}
      <span class="faint small">{t("ui.home.never")}</span>
    {/if}
  {/snippet}
  {#snippet center()}
    <ActionButton label={t("ui.home.quick_clean")} disabled={app.busy || app.modes.length === 0} onclick={quickClean} />
  {/snippet}
</Dock>

<style>
  .home { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 34px; padding: 0 32px 24px; min-height: 0; overflow: auto; }
  .hero { display: flex; flex-direction: column; align-items: center; gap: 8px; text-align: center; }
  .ring { position: relative; width: 196px; height: 196px; margin-bottom: 14px; animation: rise 0.35s ease-out; }
  .ring svg { width: 100%; height: 100%; transform: rotate(-90deg); filter: drop-shadow(0 14px 30px color-mix(in srgb, var(--a) 35%, transparent)); }
  .track { fill: none; stroke: var(--line-2); stroke-width: 7; }
  .value { fill: none; stroke: url(#ring-g); stroke-width: 7; stroke-linecap: round; transition: stroke-dasharray 0.8s cubic-bezier(0.2, 0.8, 0.2, 1); }
  .ring.tight .value { stroke: var(--warn); }
  .ring-text { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; }
  .ring-num { font-family: var(--font-display); font-size: 34px; font-weight: 600; letter-spacing: -0.02em; }
  .ring-num small { font-size: 15px; font-weight: 500; color: var(--text-2); margin-left: 4px; }
  h1 { font-size: 24px; }
  .others { display: flex; gap: 22px; margin-top: 14px; flex-wrap: wrap; justify-content: center; }
  .mini { display: flex; align-items: center; gap: 8px; font-size: 12px; }
  .mini-bar { width: 70px; height: 4px; border-radius: 99px; background: var(--line-2); overflow: hidden; }
  .mini-bar > span { display: block; height: 100%; background: var(--text-3); }
  .mini-bar > span.full { background: var(--warn); }

  .shortcuts { display: grid; grid-template-columns: repeat(3, minmax(0, 240px)); gap: 6px; }
  .sc {
    display: flex; align-items: center; gap: 12px; padding: 12px; border: 0; background: none; cursor: pointer;
    text-align: left; border-radius: 12px; color: var(--text-3);
  }
  .sc:hover { background: var(--hover); }
  .sc-ic {
    width: 40px; height: 40px; border-radius: 12px; display: grid; place-items: center; flex-shrink: 0; color: #fff;
    background: radial-gradient(100% 80% at 30% 10%, rgb(255 255 255 / 0.28), transparent 60%), linear-gradient(145deg, var(--a), var(--b));
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.35);
  }
  .sc-text { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .sc-text strong { color: var(--text); font-weight: 600; }
  .sc-text span { font-size: 12px; }

  .hist { display: flex; flex-direction: column; gap: 1px; }
  .small { font-size: 12px; }
</style>
