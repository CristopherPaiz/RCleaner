<script lang="ts">
  // Diálogo modal: fondo desenfocado, cierra con Escape o clic fuera.
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    onclose: () => void;
    children: Snippet;
    buttons: Snippet;
    /** Más ancho (listas largas). */
    wide?: boolean;
  }
  let { title, onclose, children, buttons, wide = false }: Props = $props();
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="backdrop" role="presentation" onclick={onclose}>
  <div class="dialog" class:wide role="dialog" aria-modal="true" aria-label={title} tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
    <h2>{title}</h2>
    <div class="body">{@render children()}</div>
    <div class="buttons">{@render buttons()}</div>
  </div>
</div>

<style>
  .body { min-height: 0; overflow: auto; display: flex; flex-direction: column; margin-right: -8px; padding-right: 8px; }
</style>
