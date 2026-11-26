<script lang="ts">
  // Baldosa con degradado de la sección y un icono blanco: la "cara" de cada
  // pantalla. Con `busy`, un anillo gira alrededor mientras se trabaja.
  import Icon from "./Icon.svelte";

  interface Props {
    icon: string;
    size?: number;
    busy?: boolean;
    /** Usa el verde de "correcto" en lugar de los colores de la sección. */
    success?: boolean;
  }
  let { icon, size = 128, busy = false, success = false }: Props = $props();
</script>

<div class="glyph" class:busy class:success style:--s="{size}px">
  {#if busy}<span class="ring" aria-hidden="true"></span>{/if}
  <span class="tile">
    <Icon name={icon} size={Math.round(size * 0.44)} stroke={1.5} />
  </span>
</div>

<style>
  .glyph { position: relative; width: var(--s); height: var(--s); display: grid; place-items: center; flex-shrink: 0; }
  .tile {
    width: 100%; height: 100%; display: grid; place-items: center; color: #fff;
    border-radius: 30%;
    background:
      radial-gradient(120% 90% at 30% 10%, rgb(255 255 255 / 0.35), transparent 55%),
      linear-gradient(145deg, var(--a), var(--b));
    box-shadow:
      inset 0 1px 0 rgb(255 255 255 / 0.4),
      inset 0 -10px 24px rgb(0 0 0 / 0.12),
      0 22px 50px -14px color-mix(in srgb, var(--a) 70%, transparent);
    animation: rise 0.35s ease-out;
  }
  .success .tile { --a: #10b98a; --b: #7be495; }
  .busy .tile { transform: scale(0.84); transition: transform 0.3s; }
  .ring {
    position: absolute; inset: -4%; border-radius: 50%;
    background: conic-gradient(from 0deg, transparent 0 55%, var(--b) 85%, var(--a));
    -webkit-mask: radial-gradient(farthest-side, transparent calc(100% - 3px), #000 calc(100% - 2px));
    mask: radial-gradient(farthest-side, transparent calc(100% - 3px), #000 calc(100% - 2px));
    animation: spin 1.1s linear infinite;
  }
</style>
