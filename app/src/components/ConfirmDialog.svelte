<script lang="ts">
  import type { Finding } from "../lib/api";
  import { riskLabel, formatBytes } from "../lib/format";
  import { t, tn } from "../lib/i18n.svelte";
  import Modal from "./Modal.svelte";

  interface Props {
    findings: Finding[];
    onconfirm: () => void;
    oncancel: () => void;
  }
  let { findings, onconfirm, oncancel }: Props = $props();

  const bytes = $derived(findings.reduce((s, f) => s + f.bytes, 0));
  const risky = $derived(findings.filter((f) => f.risk !== "safe"));
</script>

<Modal title={t("ui.confirm.title", { size: formatBytes(bytes) })} onclose={oncancel}>
  <p class="dim">{tn("ui.confirm.text", findings.length)}</p>
  {#if risky.length > 0}
    <div class="warn">
      <p><strong>{tn("ui.confirm.risky", risky.length)}</strong></p>
      <ul>
        {#each risky.slice(0, 6) as f (f.id)}
          <li><span class="risk {f.risk}">{riskLabel(f.risk)}</span> {f.label} · {formatBytes(f.bytes)}</li>
        {/each}
        {#if risky.length > 6}<li class="faint">{t("ui.common.and_more", { n: risky.length - 6 })}</li>{/if}
      </ul>
    </div>
  {/if}
  {#snippet buttons()}
    <button class="btn" onclick={oncancel}>{t("ui.common.cancel")}</button>
    <button class="btn accent" onclick={onconfirm}>{t("ui.clean.clean")}</button>
  {/snippet}
</Modal>

<style>
  .warn { margin-top: 16px; padding: 12px 14px; border-radius: 10px; background: color-mix(in srgb, var(--warn) 10%, transparent); font-size: 13px; }
  ul { list-style: none; margin: 8px 0 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  li { display: flex; gap: 8px; align-items: center; }
</style>
