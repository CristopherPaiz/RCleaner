<script lang="ts">
  // Carpetas que RCleaner nunca limpia ni deja borrar, con lo que ocupan.
  import { onMount } from "svelte";
  import { app } from "../lib/app.svelte";
  import { api, type ExclusionSize } from "../lib/api";
  import { formatBytes, formatCount, splitBytes, splitPath } from "../lib/format";
  import { t, tn } from "../lib/i18n.svelte";
  import { menu } from "../lib/menu.svelte";
  import { copyText } from "../lib/activity.svelte";
  import Icon from "../components/Icon.svelte";

  let sizes = $state<Map<string, ExclusionSize>>(new Map());
  let measuring = $state(false);

  const list = $derived(app.settings.exclusions);
  const total = $derived([...sizes.values()].filter((s) => list.includes(s.path)).reduce((n, s) => n + s.bytes, 0));
  const big = $derived(splitBytes(total));
  const max = $derived(Math.max(1, ...[...sizes.values()].map((s) => s.bytes)));
  const sorted = $derived(
    [...list].sort((a, b) => (sizes.get(b)?.bytes ?? -1) - (sizes.get(a)?.bytes ?? -1)),
  );

  async function measure() {
    measuring = true;
    try {
      const r = await api.exclusionSizes();
      sizes = new Map(r.map((s) => [s.path, s]));
    } catch (e) {
      app.error = String(e);
    } finally {
      measuring = false;
    }
  }

  async function add() {
    const dir = await api.pickFolder();
    if (!dir) return;
    await app.addExclusion(dir);
    await measure();
  }

  function rowMenu(e: MouseEvent, p: string) {
    menu.show(e, [
      { label: t("ui.common.show_in_explorer"), icon: "external", action: () => api.reveal(p) },
      { label: t("ui.menu.copy_path"), icon: "copy", action: () => copyText(p) },
      "-",
      { label: t("ui.menu.unexclude"), icon: "ban", action: () => app.removeExclusion(p) },
    ]);
  }

  onMount(measure);
</script>

<div class="page">
  <header class="head">
    <div>
      <h1>{t("ui.excl.title")}</h1>
      <p class="dim">{t("ui.excl.lead")}</p>
    </div>
    <div class="acts">
      <button class="btn small quiet" disabled={measuring || list.length === 0} onclick={measure}>
        <Icon name="refresh" size={14} /> {t("ui.excl.measure")}
      </button>
      <button class="btn small" onclick={add}><Icon name="plus" size={14} /> {t("ui.excl.add")}</button>
    </div>
  </header>

  {#if list.length > 0}
    <div class="sum">
      <span class="hero-num num">{big[0]}<span class="unit">{big[1]}</span></span>
      <span class="faint">{measuring ? t("ui.excl.measuring") : tn("ui.excl.count", list.length)}</span>
    </div>
  {/if}

  <div class="group">
    {#each sorted as p (p)}
      {@const s = sizes.get(p)}
      {@const [dir, name] = splitPath(p)}
      <div class="row" role="listitem" oncontextmenu={(e) => rowMenu(e, p)}>
        <span class="ic"><Icon name="shield" size={18} /></span>
        <span class="main">
          <span class="ellipsis name">{name || p}</span>
          <span class="faint small ellipsis path"><bdi>{dir}</bdi></span>
        </span>
        <span class="share">{#if s}<span style:width="{(s.bytes / max) * 100}%"></span>{/if}</span>
        <span class="size">
          {#if s && !s.exists}
            <span class="warn small">{t("ui.excl.missing")}</span>
          {:else if s}
            <span class="num">{formatBytes(s.bytes)}</span>
            <span class="faint small num">{tn("ui.common.files", Math.max(0, s.items - 1))}</span>
          {:else}
            <span class="faint small">…</span>
          {/if}
        </span>
        <span class="row-acts">
          <button class="icon-btn" title={t("ui.common.show_in_explorer")} onclick={() => api.reveal(p)}><Icon name="external" size={14} /></button>
          <button class="icon-btn" title={t("ui.menu.unexclude")} onclick={() => app.removeExclusion(p)}><Icon name="x" size={14} /></button>
        </span>
      </div>
    {:else}
      <div class="empty">
        <Icon name="shield" size={28} />
        <p>{t("ui.excl.empty")}</p>
        <p class="faint small">{t("ui.excl.empty_hint")}</p>
      </div>
    {/each}
  </div>
  <p class="faint small note">{t("ui.excl.note", { n: formatCount(list.length) })}</p>
</div>

<style>
  .page { flex: 1; overflow: auto; padding: 4px 32px 32px; display: flex; flex-direction: column; gap: 18px; max-width: 900px; width: 100%; margin: 0 auto; }
  .head { display: flex; justify-content: space-between; align-items: flex-end; gap: 16px; flex-wrap: wrap; }
  .head h1 { font-size: 28px; margin-bottom: 4px; }
  .acts { display: flex; gap: 8px; }
  .sum { display: flex; align-items: baseline; gap: 12px; }
  .hero-num { font-size: 34px; font-weight: 700; font-family: var(--font-display); color: var(--excl); }
  .unit { font-size: 16px; margin-left: 4px; color: var(--text-2); }
  .group { display: flex; flex-direction: column; border: 1px solid var(--line); border-radius: var(--radius); overflow: hidden; background: var(--raise); }
  .row { display: grid; grid-template-columns: 28px minmax(0, 1fr) minmax(60px, 140px) 110px auto; align-items: center; gap: 12px; padding: 10px 14px; border-top: 1px solid var(--line); }
  .row:first-child { border-top: 0; }
  .row:hover { background: var(--hover); }
  .ic { color: var(--excl); display: grid; place-items: center; }
  .main { display: flex; flex-direction: column; min-width: 0; }
  .name { font-weight: 600; color: var(--excl); }
  .path { direction: rtl; text-align: left; }
  .path > bdi { direction: ltr; unicode-bidi: isolate; }
  .share { height: 6px; border-radius: 3px; background: var(--line); overflow: hidden; }
  .share > span { display: block; height: 100%; background: color-mix(in srgb, var(--excl) 70%, transparent); border-radius: 3px; }
  .size { display: flex; flex-direction: column; align-items: flex-end; }
  .warn { color: var(--warn); }
  .row-acts { display: flex; gap: 2px; opacity: 0.6; }
  .row:hover .row-acts { opacity: 1; }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 36px 16px; color: var(--text-2); text-align: center; }
  .note { margin-top: -6px; }
  @media (max-width: 760px) {
    .row { grid-template-columns: 24px minmax(0, 1fr) 96px auto; }
    .share { display: none; }
  }
</style>
