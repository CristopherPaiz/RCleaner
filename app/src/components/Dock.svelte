<script lang="ts">
  // Barra inferior: acción secundaria a la izquierda, botón circular al
  // centro y un resumen a la derecha. Siempre en el mismo sitio.
  import type { Snippet } from "svelte";

  interface Props {
    left?: Snippet;
    center?: Snippet;
    right?: Snippet;
  }
  let { left, center, right }: Props = $props();
</script>

<div class="dock">
  <div class="side">{@render left?.()}</div>
  <div class="center">{@render center?.()}</div>
  <div class="side right">{@render right?.()}</div>
</div>

<style>
  .dock {
    display: grid; grid-template-columns: 1fr auto 1fr; align-items: center; gap: 20px;
    padding: 14px 32px 22px; flex-shrink: 0; position: relative;
  }
  .side { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .right { justify-content: flex-end; text-align: right; }
  .center { display: grid; place-items: center; min-height: 92px; }
</style>
