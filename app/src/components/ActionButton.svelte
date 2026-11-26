<script lang="ts">
  // Botón circular principal de cada pantalla (Analizar, Limpiar, Detener…).
  interface Props {
    label: string;
    onclick: () => void;
    disabled?: boolean;
    /** `stop`: neutro, para cancelar. */
    variant?: "go" | "stop";
  }
  let { label, onclick, disabled = false, variant = "go" }: Props = $props();
</script>

<button class="action {variant}" {disabled} {onclick}>
  <span>{label}</span>
</button>

<style>
  .action {
    position: relative; width: 92px; height: 92px; border-radius: 50%; border: 0; cursor: pointer;
    font-family: var(--font-display); font-weight: 600; font-size: 14px; color: #fff;
    display: grid; place-items: center; padding: 0 8px; line-height: 1.15;
    transition: transform 0.15s ease, filter 0.15s, box-shadow 0.2s;
  }
  /* Halo exterior fino, como un bisel. */
  .action::before {
    content: ""; position: absolute; inset: -7px; border-radius: 50%;
    border: 1px solid var(--line-2); pointer-events: none;
  }
  .go {
    background:
      radial-gradient(100% 80% at 30% 15%, rgb(255 255 255 / 0.3), transparent 60%),
      linear-gradient(145deg, var(--a), var(--b));
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.35), 0 16px 40px -12px color-mix(in srgb, var(--a) 80%, transparent);
  }
  .stop {
    background: var(--raise-2); color: var(--text);
    box-shadow: inset 0 1px 0 var(--line-2), 0 10px 30px -12px rgb(0 0 0 / 0.6);
  }
  .action:hover:not(:disabled) { transform: scale(1.04); filter: brightness(1.06); }
  .action:active:not(:disabled) { transform: scale(0.98); }
  .action:disabled { opacity: 0.4; cursor: default; filter: grayscale(0.4); }
</style>
