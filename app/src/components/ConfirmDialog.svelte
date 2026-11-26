<script lang="ts">
  // Confirmación de la limpieza: lo que se va a borrar (desplegable) y aviso
  // de lo que no es de riesgo "seguro".
  import type { Finding } from "../lib/api";
  import { riskLabel, formatBytes } from "../lib/format";
  import { t, tn } from "../lib/i18n.svelte";
  import ExpandList from "./ExpandList.svelte";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";

  interface Props {
    findings: Finding[];
    onconfirm: () => void;
    oncancel: () => void;
  }
  let { findings, onconfirm, oncancel }: Props = $props();

  const bytes = $derived(findings.reduce((s, f) => s + f.bytes, 0));
  const sorted = $derived([...findings].sort((a, b) => b.bytes - a.bytes));
  const risky = $derived(sorted.filter((f) => f.risk !== "safe"));
</script>

<Modal title={t("ui.confirm.title", { size: formatBytes(bytes) })} onclose={oncancel}>
  <p class="dim">{tn("ui.confirm.text", findings.length)}</p>
  <ExpandList items={sorted}>
    {#snippet row(f)}
      <span class="ellipsis" title={f.path ?? f.label}>{f.label}</span>
      <span class="num faint">{formatBytes(f.bytes)}</span>
    {/snippet}
  </ExpandList>
  {#if risky.length > 0}
    <div class="warn">
      <p><Icon name="alert" size={14} /> <strong>{tn("ui.confirm.risky", risky.length)}</strong></p>
      <ExpandList items={risky} limit={4}>
        {#snippet row(f)}
          <span class="ellipsis"><span class="risk {f.risk}">{riskLabel(f.risk)}</span> {f.label}</span>
          <span class="num faint">{formatBytes(f.bytes)}</span>
        {/snippet}
      </ExpandList>
    </div>
  {/if}
  <p class="faint note">{t("ui.confirm.permanent")}</p>
  {#snippet buttons()}
    <button class="btn" onclick={oncancel}>{t("ui.common.cancel")}</button>
    <button class="btn accent" onclick={onconfirm}>{t("ui.clean.clean")}</button>
  {/snippet}
</Modal>

<style>
  .warn { margin-top: 16px; padding: 12px 14px; border-radius: 10px; background: color-mix(in srgb, var(--warn) 10%, transparent); font-size: 13px; }
  .warn > p { display: flex; align-items: center; gap: 6px; color: var(--warn); }
  .note { margin-top: 12px; font-size: 12px; }
</style>
