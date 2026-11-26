<script lang="ts" module>
  export interface DeleteItem {
    key: string | number;
    name: string;
    size: number;
    /** Archivo de Windows protegido elegido bajo responsabilidad del usuario. */
    system?: boolean;
  }
</script>

<script lang="ts">
  // Borrado en tres pasos para Espacio y Duplicados:
  // 1. Revisar la lista y elegir papelera o borrado permanente.
  // 2. Progreso con la ruta que se está eliminando (se puede detener).
  // 3. Resultado con todo lo eliminado y los fallos.
  import { activity } from "../lib/activity.svelte";
  import { api, type DeleteResult } from "../lib/api";
  import { formatBytes, splitPath } from "../lib/format";
  import { t, tn } from "../lib/i18n.svelte";
  import ExpandList from "./ExpandList.svelte";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";

  interface Props {
    items: DeleteItem[];
    run: (permanent: boolean) => Promise<DeleteResult | null>;
    onclose: () => void;
  }
  let { items, run, onclose }: Props = $props();

  let step = $state<"confirm" | "running" | "done">("confirm");
  let permanent = $state(false);
  let result = $state<DeleteResult | null>(null);

  const bytes = $derived(items.reduce((s, i) => s + i.size, 0));
  const systemCount = $derived(items.filter((i) => i.system).length);
  const pct = $derived(activity.del.total ? Math.round((activity.del.done / activity.del.total) * 100) : 0);

  async function start() {
    activity.reset();
    step = "running";
    result = await run(permanent);
    activity.reset();
    if (result) step = "done";
    else onclose();
  }

  function close() {
    if (step !== "running") onclose();
  }

  const title = $derived(
    step === "confirm"
      ? t("ui.delete.title", { size: formatBytes(bytes) })
      : step === "running"
        ? t(permanent ? "ui.delete.deleting" : "ui.delete.recycling")
        : t(permanent ? "ui.delete.done_permanent" : "ui.delete.done_bin", { size: formatBytes(result?.freed_bytes ?? 0) }),
  );
</script>

<Modal {title} onclose={close}>
  {#if step === "confirm"}
    <p class="dim">{tn("ui.delete.text", items.length)}</p>
    <ExpandList {items}>
      {#snippet row(it)}
        <span class="ellipsis" class:sys={it.system}>{#if it.system}<Icon name="alert" size={12} /> {/if}{it.name}</span>
        <span class="num faint">{formatBytes(it.size)}</span>
      {/snippet}
    </ExpandList>
    {#if systemCount > 0}
      <p class="sys-note"><Icon name="alert" size={14} /> {tn("ui.delete.system_note", systemCount)}</p>
    {/if}

    <div class="choices" role="radiogroup" aria-label={t("ui.delete.how")}>
      <button class="choice" class:on={!permanent} role="radio" aria-checked={!permanent} onclick={() => (permanent = false)}>
        <span class="c-ic"><Icon name="trash" size={18} /></span>
        <span class="c-text"><strong>{t("ui.delete.bin")}</strong><span class="faint">{t("ui.delete.bin_hint")}</span></span>
      </button>
      <button class="choice danger" class:on={permanent} role="radio" aria-checked={permanent} onclick={() => (permanent = true)}>
        <span class="c-ic"><Icon name="alert" size={18} /></span>
        <span class="c-text"><strong>{t("ui.delete.permanent")}</strong><span class="faint">{t("ui.delete.permanent_hint")}</span></span>
      </button>
    </div>
  {:else if step === "running"}
    <div class="meter"><div style:width="{pct}%"></div></div>
    <p class="faint num small">{t("ui.delete.progress", { done: activity.del.done, total: activity.del.total || items.length })}</p>
    <p class="now mono" title={activity.now ?? ""}>
      {#if activity.now}
        {@const [dir, name] = splitPath(activity.now)}
        <span class="faint">{dir}\</span>{name}
      {:else}
        …
      {/if}
    </p>
  {:else if result}
    <p class="dim">{tn("ui.delete.removed", result.removed)}</p>
    {#if result.removed_paths.length > 0}
      <ExpandList items={result.removed_paths} limit={5}>
        {#snippet row(p)}
          <span class="mono ellipsis path" title={p}><bdi>{p}</bdi></span>
        {/snippet}
      </ExpandList>
    {/if}
    {#if result.errors.length > 0}
      <details class="errors">
        <summary>{tn("ui.delete.errors", result.errors.length)}</summary>
        {#each result.errors.slice(0, 50) as e, i (i)}<div class="mono faint">{e}</div>{/each}
      </details>
    {/if}
  {/if}

  {#snippet buttons()}
    {#if step === "confirm"}
      <button class="btn" onclick={onclose}>{t("ui.common.cancel")}</button>
      <button class="btn accent" class:danger={permanent} onclick={start}>
        {t(permanent ? "ui.delete.confirm_permanent" : "ui.common.move_to_bin")}
      </button>
    {:else if step === "running"}
      <button class="btn" onclick={() => api.cancel("delete")}><Icon name="stop" size={14} /> {t("ui.common.stop")}</button>
    {:else}
      <button class="btn accent" onclick={onclose}>{t("ui.common.done")}</button>
    {/if}
  {/snippet}
</Modal>

<style>
  .choices { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin-top: 18px; }
  .choice {
    display: flex; gap: 10px; align-items: flex-start; padding: 12px; border-radius: 12px; text-align: left; cursor: pointer;
    border: 1px solid var(--line-2); background: var(--raise); color: var(--text-2);
  }
  .choice:hover { background: var(--raise-2); }
  .choice.on { border-color: color-mix(in srgb, var(--a) 70%, transparent); background: color-mix(in srgb, var(--a) 10%, var(--raise)); color: var(--text); }
  .choice.danger.on { border-color: color-mix(in srgb, var(--bad) 70%, transparent); background: color-mix(in srgb, var(--bad) 10%, var(--raise)); }
  .c-ic { color: var(--a); margin-top: 1px; }
  .choice.danger .c-ic { color: var(--bad); }
  .c-text { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
  .c-text strong { color: var(--text); font-size: 13px; }
  .c-text span { font-size: 12px; }
  .btn.danger { background: var(--bad); }
  .sys { color: var(--warn); display: inline-flex; align-items: center; gap: 4px; }
  .sys-note { margin-top: 10px; font-size: 12px; color: var(--warn); display: flex; align-items: center; gap: 6px; }
  .meter { height: 6px; border-radius: 99px; background: var(--line-2); overflow: hidden; margin-top: 8px; }
  .meter > div { height: 100%; background: linear-gradient(90deg, var(--a), var(--b)); transition: width 0.2s; }
  .small { font-size: 12px; margin-top: 8px; }
  .now { margin-top: 10px; min-height: 2.6em; font-size: 12px; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; word-break: break-all; }
  .errors { margin-top: 12px; font-size: 12px; max-height: 180px; overflow: auto; }
  .errors summary { cursor: pointer; color: var(--warn); }
  @media (max-width: 560px) { .choices { grid-template-columns: 1fr; } }
</style>
