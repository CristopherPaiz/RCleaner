<script lang="ts">
  // Icono de un archivo, carpeta o app: el incluido para carpetas conocidas,
  // si no el de Windows, y mientras tanto (o si no hay) el propio de la UI.
  import { icons } from "../lib/icons.svelte";
  import Icon from "./Icon.svelte";

  interface Props {
    /** Clave para el icono de Windows (`ext:zip`, `node:12`, `app:chrome.exe`…). */
    key?: string | null;
    /** Icono incluido (tiene prioridad). */
    src?: string | null;
    /** Icono propio de la UI de reserva. */
    fallback: string;
    size?: number;
  }
  let { key = null, src = null, fallback, size = 16 }: Props = $props();

  const url = $derived(src ?? (key ? icons.get(key) : undefined));
</script>

{#if url}
  <img src={url} width={size} height={size} alt="" draggable="false" />
{:else}
  <Icon name={fallback} {size} />
{/if}

<style>
  img { display: block; flex-shrink: 0; object-fit: contain; }
</style>
