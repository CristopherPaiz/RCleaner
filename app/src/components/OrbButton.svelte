<script lang="ts">
  // Botón principal de una herramienta. En reposo es una baldosa con el icono
  // y la acción ("Analizar"); al pulsarlo se convierte en círculo, aparece un
  // anillo girando alrededor y dentro el progreso. Pulsarlo otra vez detiene.
  import Icon from "./Icon.svelte";
  import { morph } from "../lib/motion";
  import { t } from "../lib/i18n.svelte";

  interface Props {
    icon: string;
    label: string;
    busy?: boolean;
    /** 0–100 mientras trabaja; sin valor muestra sólo el anillo. */
    percent?: number | null;
    disabled?: boolean;
    size?: number;
    onclick: () => void;
  }
  let { icon, label, busy = false, percent = null, disabled = false, size = 176, onclick }: Props = $props();
</script>

<div class="orb-wrap" style:--s="{size}px">
  {#if busy}<span class="ring" aria-hidden="true"></span>{/if}
  <button
    class="orb"
    class:busy
    {disabled}
    {onclick}
    aria-label={busy ? t("ui.common.stop") : label}
    title={busy ? t("ui.common.stop") : undefined}
  >
    <span class="face idle-face">
      <span class="stack">
        {#key icon}
          <span class="glyph" in:morph={{ rotate: -30 }} out:morph={{ rotate: 30, duration: 260 }}>
            <Icon name={icon} size={Math.round(size * 0.3)} stroke={1.6} />
          </span>
        {/key}
      </span>
      <span class="label">{label}</span>
    </span>
    <span class="face busy-face" aria-hidden={!busy}>
      <span class="center">
        {#if percent !== null}
          <span class="pct num">{percent}<small>%</small></span>
        {:else}
          <Icon name={icon} size={Math.round(size * 0.24)} stroke={1.6} />
        {/if}
      </span>
      <span class="stop">{t("ui.common.stop")}</span>
    </span>
  </button>
</div>

<style>
  .orb-wrap { position: relative; width: var(--s); height: var(--s); display: grid; place-items: center; flex-shrink: 0; }

  .orb {
    position: relative; width: 100%; height: 100%; border: 0; padding: 0; cursor: pointer; color: #fff;
    border-radius: 30%;
    background:
      radial-gradient(120% 90% at 30% 8%, rgb(255 255 255 / 0.34), transparent 55%),
      linear-gradient(145deg, var(--a), var(--b));
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.4),
      inset 0 -12px 28px rgb(0 0 0 / 0.14),
      0 24px 56px -16px color-mix(in srgb, var(--a) 75%, transparent);
    transition:
      border-radius 0.5s cubic-bezier(0.3, 0.9, 0.3, 1),
      transform 0.5s cubic-bezier(0.3, 0.9, 0.3, 1),
      box-shadow 0.25s, background 0.4s;
    animation: rise 0.35s ease-out;
  }
  .orb:not(.busy):not(:disabled):hover {
    transform: translateY(-3px) scale(1.02);
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.45),
      inset 0 -12px 28px rgb(0 0 0 / 0.14),
      0 30px 70px -14px color-mix(in srgb, var(--a) 90%, transparent);
  }
  .orb:not(.busy):active:not(:disabled) { transform: scale(0.97); }
  .orb.busy { border-radius: 50%; transform: scale(0.84); }
  .orb:disabled { opacity: 0.5; cursor: default; }

  .face {
    position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px;
    transition: opacity 0.25s, transform 0.35s;
  }
  .stack { display: grid; place-items: center; }
  .glyph { grid-area: 1 / 1; display: grid; }
  .label { font-family: var(--font-display); font-weight: 600; font-size: 15px; letter-spacing: 0.01em; }
  .busy .idle-face { opacity: 0; transform: scale(0.85); }
  .busy-face { opacity: 0; transform: scale(1.1); pointer-events: none; }
  .busy .busy-face { opacity: 1; transform: none; }
  .pct { font-family: var(--font-display); font-size: calc(var(--s) * 0.2); font-weight: 600; line-height: 1; }
  .pct small { font-size: 0.5em; margin-left: 2px; opacity: 0.85; }
  /* El progreso queda centrado; al pasar el ratón sube un poco y "Detener" entra debajo. */
  .center { display: grid; place-items: center; transition: transform 0.3s cubic-bezier(0.3, 0.9, 0.3, 1); }
  .stop {
    position: absolute; left: 50%; top: 64%; transform: translate(-50%, 8px);
    font-size: 12px; font-weight: 600; padding: 3px 10px; border-radius: 99px; white-space: nowrap;
    background: rgb(0 0 0 / 0.18); opacity: 0;
    transition: opacity 0.2s, transform 0.3s cubic-bezier(0.3, 0.9, 0.3, 1);
  }
  .orb.busy:hover .center { transform: translateY(calc(var(--s) * -0.06)); }
  .orb.busy:hover .stop { opacity: 1; transform: translate(-50%, 0); }

  .ring {
    position: absolute; inset: 2%; border-radius: 50%; pointer-events: none;
    background: conic-gradient(from 0deg, transparent 0 50%, var(--b) 82%, var(--a));
    -webkit-mask: radial-gradient(farthest-side, transparent calc(100% - 4px), #000 calc(100% - 3px));
    mask: radial-gradient(farthest-side, transparent calc(100% - 4px), #000 calc(100% - 3px));
    animation: spin 1.1s linear infinite, fade 0.4s ease-out;
  }
</style>
