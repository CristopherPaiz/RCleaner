<script lang="ts">
  // Aviso al elegir un archivo de Windows protegido (sólo con "archivos de
  // sistema" activado en Ajustes). Siempre pide aceptar la responsabilidad.
  import { t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";

  interface Props {
    name: string;
    reason: string;
    onaccept: () => void;
    oncancel: () => void;
  }
  let { name, reason, onaccept, oncancel }: Props = $props();
  let agreed = $state(false);
</script>

<Modal title={t("ui.system.title")} onclose={oncancel}>
  <div class="warn">
    <Icon name="alert" size={20} />
    <div>
      <strong class="ellipsis">{name}</strong>
      <p>{reason}</p>
    </div>
  </div>
  <p class="dim">{t("ui.system.text")}</p>
  <label class="agree">
    <input type="checkbox" bind:checked={agreed} />
    <span>{t("ui.system.agree")}</span>
  </label>
  {#snippet buttons()}
    <button class="btn" onclick={oncancel}>{t("ui.common.cancel")}</button>
    <button class="btn danger" disabled={!agreed} onclick={onaccept}>{t("ui.system.accept")}</button>
  {/snippet}
</Modal>

<style>
  .warn {
    display: flex; gap: 12px; padding: 12px 14px; border-radius: 12px; margin-bottom: 12px; color: var(--warn);
    background: color-mix(in srgb, var(--warn) 10%, transparent); border: 1px solid color-mix(in srgb, var(--warn) 30%, transparent);
  }
  .warn div { min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .warn strong { color: var(--text); }
  .warn p { font-size: 13px; }
  .agree { display: flex; gap: 10px; align-items: flex-start; margin-top: 14px; font-size: 13px; cursor: pointer; }
  .btn.danger { background: var(--bad); border: 0; color: #fff; font-weight: 600; }
</style>
