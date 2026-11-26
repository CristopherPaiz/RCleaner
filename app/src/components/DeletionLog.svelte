<script lang="ts">
  // Registro de todo lo que RCleaner eliminó (limpieza, Espacio y Duplicados).
  import { onMount } from "svelte";
  import { api, type DeletionRecord } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { formatBytes, formatDate } from "../lib/format";
  import { t } from "../lib/i18n.svelte";
  import { menu } from "../lib/menu.svelte";
  import { copyText } from "../lib/activity.svelte";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";

  interface Props {
    onclose: () => void;
  }
  let { onclose }: Props = $props();

  let records = $state<DeletionRecord[] | null>(null);
  let query = $state("");
  let source = $state<"" | DeletionRecord["source"]>("");

  onMount(() => {
    void api.deletions(2000).then((r) => (records = r));
  });

  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return (records ?? []).filter((r) => (!source || r.source === source) && (!q || r.path.toLowerCase().includes(q)));
  });
  const total = $derived(shown.reduce((s, r) => s + r.bytes, 0));

  const SOURCES = [
    { id: "", label: "ui.log.all" },
    { id: "clean", label: "ui.nav.clean" },
    { id: "space", label: "ui.nav.space" },
    { id: "dupes", label: "ui.nav.dupes" },
  ] as const;
</script>

<Modal title={t("ui.log.title")} {onclose} wide>
  <p class="dim">{t("ui.log.text")}</p>
  <div class="tools">
    <label class="field search">
      <Icon name="search" size={14} />
      <input placeholder={t("ui.common.filter")} bind:value={query} />
    </label>
    <div class="segmented small">
      {#each SOURCES as s (s.id)}
        <button class:on={source === s.id} onclick={() => (source = s.id)}>{t(s.label)}</button>
      {/each}
    </div>
  </div>
  {#if records === null}
    <p class="faint pad">…</p>
  {:else if shown.length === 0}
    <p class="faint pad">{t("ui.log.empty")}</p>
  {:else}
    <p class="faint small num">{t("ui.log.total", { n: shown.length, size: formatBytes(total) })}</p>
    <div class="rows">
      {#each shown.slice(0, 500) as r, i (i)}
        <div
          class="row"
          role="listitem"
          oncontextmenu={(e) => menu.show(e, [{ label: t("ui.menu.copy_path"), icon: "copy", action: () => copyText(r.path) }])}
        >
          <span class="src" title={t(r.permanent ? "ui.log.permanent" : "ui.log.bin")}>
            <Icon name={r.permanent ? "alert" : "trash"} size={13} />
          </span>
          <span class="main">
            <span class="mono ellipsis path" title={r.path}><bdi>{r.path}</bdi></span>
            <span class="faint small">{formatDate(r.timestamp)} · {t(`ui.nav.${r.source}`)}</span>
          </span>
          <span class="num size">{formatBytes(r.bytes)}</span>
        </div>
      {/each}
    </div>
  {/if}
  {#snippet buttons()}
    {#if app.info}
      <button class="btn quiet" onclick={() => api.reveal(app.info!.config_dir)}><Icon name="external" size={14} /> {t("ui.settings.config_folder")}</button>
    {/if}
    <button class="btn accent" onclick={onclose}>{t("ui.common.close")}</button>
  {/snippet}
</Modal>

<style>
  .tools { display: flex; gap: 8px; margin: 14px 0 8px; flex-wrap: wrap; }
  .search { flex: 1; min-width: 160px; height: 30px; }
  .segmented.small button { height: 24px; padding: 0 8px; font-size: 12px; }
  .rows { display: flex; flex-direction: column; gap: 2px; margin-top: 6px; }
  .row { display: grid; grid-template-columns: 18px minmax(0, 1fr) auto; gap: 10px; align-items: center; padding: 6px; border-radius: 8px; }
  .row:hover { background: var(--hover); }
  .src { color: var(--text-3); display: grid; place-items: center; }
  .main { display: flex; flex-direction: column; min-width: 0; }
  .size { font-weight: 600; font-size: 13px; }
  .small { font-size: 11px; }
  .pad { padding: 20px 0; }
</style>
