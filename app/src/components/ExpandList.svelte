<script lang="ts" generics="T">
  // Lista corta con "y N más" que se puede pulsar para ver todo.
  import type { Snippet } from "svelte";
  import { t } from "../lib/i18n.svelte";

  interface Props {
    items: T[];
    limit?: number;
    row: Snippet<[T]>;
    /** Alto máximo al desplegar (con scroll). */
    maxHeight?: number;
  }
  let { items, limit = 6, row, maxHeight = 260 }: Props = $props();
  let all = $state(false);

  const shown = $derived(all ? items : items.slice(0, limit));
</script>

<ul class="xl" class:all style:max-height={all ? `${maxHeight}px` : undefined}>
  {#each shown as item, i (i)}
    <li>{@render row(item)}</li>
  {/each}
</ul>
{#if items.length > limit}
  <button class="link more" onclick={() => (all = !all)}>
    {all ? t("ui.common.show_less") : t("ui.common.and_more", { n: items.length - limit })}
  </button>
{/if}

<style>
  .xl { list-style: none; margin: 12px 0 0; padding: 0; display: flex; flex-direction: column; gap: 4px; font-size: 13px; }
  .xl.all { overflow: auto; padding-right: 6px; }
  li { display: flex; justify-content: space-between; align-items: center; gap: 12px; min-width: 0; }
  .more { margin-top: 6px; font-size: 13px; align-self: flex-start; }
</style>
