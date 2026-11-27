<script lang="ts">
  // Versión nueva: notas, progreso de la descarga e instalar (o descargar,
  // en la versión portable).
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import { update } from "../lib/update.svelte";
  import { app } from "../lib/app.svelte";
  import { formatBytes } from "../lib/format";
  import { t } from "../lib/i18n.svelte";

  const installing = $derived(update.status === "installing");
  const close = () => {
    if (!installing) update.open = false;
  };
  /** Notas del CHANGELOG sin la sintaxis de Markdown. */
  const notes = $derived(
    (update.info?.notes ?? "")
      .replace(/^#+\s*/gm, "")
      .replace(/\*\*(.+?)\*\*/g, "$1")
      .replace(/`([^`]+)`/g, "$1")
      .replace(/^- /gm, "• ")
      .trim(),
  );
</script>

{#if update.info}
  <Modal title={t("ui.update.title", { v: update.info.version })} onclose={close}>
    <p class="faint">{t("ui.update.current", { v: app.info?.version ?? "?" })}</p>
    {#if notes}
      <div class="notes">{notes}</div>
    {/if}
    {#if !update.info.installable}
      <p class="faint small">{t("ui.update.portable_hint")}</p>
    {/if}
    {#if installing}
      <div class="bar" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={update.fraction === null ? undefined : Math.round(update.fraction * 100)}>
        <span class:indeterminate={update.fraction === null} style:width={update.fraction === null ? undefined : `${update.fraction * 100}%`}></span>
      </div>
      <p class="faint small num">
        {t("ui.update.installing")}
        {#if update.progress}· {formatBytes(update.progress.done)}{#if update.progress.total} / {formatBytes(update.progress.total)}{/if}{/if}
      </p>
    {/if}
    {#if update.status === "error" && update.error}
      <p class="error">{update.error}</p>
    {/if}
    {#snippet buttons()}
      <button class="btn" disabled={installing} onclick={close}>{t("ui.update.later")}</button>
      <button class="btn primary" disabled={installing} onclick={() => update.install()}>
        <Icon name={update.info?.installable ? "up" : "external"} size={14} />
        {update.info?.installable ? t("ui.update.install") : t("ui.update.download")}
      </button>
    {/snippet}
  </Modal>
{/if}

<style>
  .notes {
    white-space: pre-wrap; font-size: 13px; line-height: 1.5; padding: 10px 12px; margin: 8px 0;
    border: 1px solid var(--line); border-radius: 10px; max-height: 300px; overflow: auto;
  }
  .bar { height: 6px; border-radius: 99px; background: var(--line); overflow: hidden; margin: 12px 0 6px; }
  .bar span { display: block; height: 100%; background: var(--accent, #2563eb); border-radius: inherit; transition: width 0.2s; }
  .bar .indeterminate { width: 30%; animation: slide 1.2s ease-in-out infinite; }
  @keyframes slide { from { transform: translateX(-100%); } to { transform: translateX(340%); } }
</style>
