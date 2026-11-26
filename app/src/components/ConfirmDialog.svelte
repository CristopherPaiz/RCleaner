<script lang="ts">
  import type { Finding } from "../lib/api";
  import { formatBytes } from "../lib/format";

  interface Props {
    findings: Finding[];
    onconfirm: () => void;
    oncancel: () => void;
  }
  let { findings, onconfirm, oncancel }: Props = $props();

  const bytes = $derived(findings.reduce((s, f) => s + f.bytes, 0));
  const risky = $derived(findings.filter((f) => f.risk !== "safe"));
</script>

<div class="backdrop" role="presentation" onclick={oncancel}>
  <div class="dialog" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.key === "Escape" && oncancel()}>
    <h2>¿Limpiar {formatBytes(bytes)}?</h2>
    <p>Se eliminarán {findings.length} elementos. Esta acción no se puede deshacer.</p>
    {#if risky.length > 0}
      <div class="warn">
        <strong>{risky.length} elementos no son "Seguros":</strong>
        <ul>
          {#each risky.slice(0, 6) as f (f.id)}
            <li>{f.label} · {formatBytes(f.bytes)}</li>
          {/each}
          {#if risky.length > 6}<li>… y {risky.length - 6} más</li>{/if}
        </ul>
      </div>
    {/if}
    <div class="buttons">
      <button class="ghost" onclick={oncancel}>Cancelar</button>
      <button class="danger" onclick={onconfirm}>Limpiar</button>
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgb(0 0 0 / 0.45); display: grid; place-items: center; }
  .dialog { width: min(520px, 92vw); background: var(--panel); border: 1px solid var(--border); border-radius: 14px; padding: 22px 24px; }
  h2 { margin: 0 0 8px; }
  .warn { margin: 14px 0; padding: 10px 14px; border-radius: 8px; background: var(--warn-bg); color: var(--warn); font-size: 13px; }
  ul { margin: 6px 0 0; padding-left: 18px; }
  .buttons { display: flex; justify-content: flex-end; gap: 8px; margin-top: 18px; }
</style>
