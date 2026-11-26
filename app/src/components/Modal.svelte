<script lang="ts">
  // Diálogo modal: fondo desenfocado, cierra con Escape o clic fuera.
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    onclose: () => void;
    children: Snippet;
    buttons: Snippet;
  }
  let { title, onclose, children, buttons }: Props = $props();
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="backdrop" role="presentation" onclick={onclose}>
  <div class="dialog" role="dialog" aria-modal="true" aria-label={title} tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
    <h2>{title}</h2>
    {@render children()}
    <div class="buttons">{@render buttons()}</div>
  </div>
</div>
