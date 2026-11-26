<script lang="ts">
  // Barra superior propia (la ventana no tiene marco de Windows). Toda la
  // franja sirve para arrastrar; doble clic maximiza (lo hace Tauri con
  // data-tauri-drag-region). Botones propios a la derecha.
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { t } from "../lib/i18n.svelte";

  const win = getCurrentWindow();
  let maximized = $state(false);

  onMount(() => {
    const sync = () => void win.isMaximized().then((m) => (maximized = m));
    sync();
    const off = win.onResized(sync);
    return () => void off.then((u) => u());
  });
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="controls">
    <button class="ctl" title={t("ui.window.minimize")} aria-label={t("ui.window.minimize")} onclick={() => win.minimize()}>
      <svg viewBox="0 0 12 12" width="12" height="12"><path d="M2 6h8" /></svg>
    </button>
    <button
      class="ctl"
      title={t(maximized ? "ui.window.restore" : "ui.window.maximize")}
      aria-label={t(maximized ? "ui.window.restore" : "ui.window.maximize")}
      onclick={() => win.toggleMaximize()}
    >
      {#if maximized}
        <svg viewBox="0 0 12 12" width="12" height="12"><path d="M3.5 3.5V2h6.5v6.5H8.5M2 3.5h6.5V10H2z" /></svg>
      {:else}
        <svg viewBox="0 0 12 12" width="12" height="12"><rect x="2" y="2" width="8" height="8" rx="1.5" /></svg>
      {/if}
    </button>
    <button class="ctl close" title={t("ui.window.close")} aria-label={t("ui.window.close")} onclick={() => win.close()}>
      <svg viewBox="0 0 12 12" width="12" height="12"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7" /></svg>
    </button>
  </div>
</div>

<style>
  .titlebar {
    height: 40px; flex-shrink: 0; display: flex; justify-content: flex-end; align-items: center;
    padding: 0 8px; position: relative; z-index: 10;
  }
  .controls { display: flex; gap: 4px; }
  .ctl {
    width: 34px; height: 28px; border: 0; border-radius: 8px; background: transparent;
    color: var(--text-3); display: grid; place-items: center; cursor: pointer;
    transition: background 0.12s, color 0.12s;
  }
  .ctl svg { fill: none; stroke: currentColor; stroke-width: 1.3; stroke-linecap: round; }
  .ctl:hover { background: var(--hover); color: var(--text); }
  .ctl.close:hover { background: #e5484d; color: #fff; }
</style>
