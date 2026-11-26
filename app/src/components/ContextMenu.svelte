<script lang="ts">
  // Menú contextual flotante (ver lib/menu.svelte.ts). Se recoloca para no
  // salirse de la ventana; flechas, Enter y Escape funcionan.
  import { tick } from "svelte";
  import { menu, type MenuItem } from "../lib/menu.svelte";
  import Icon from "./Icon.svelte";

  let el = $state<HTMLDivElement>();
  let pos = $state({ x: 0, y: 0 });

  $effect(() => {
    const m = menu.open;
    if (!m) return;
    pos = { x: m.x, y: m.y };
    void tick().then(() => {
      if (!el) return;
      const r = el.getBoundingClientRect();
      pos = {
        x: Math.max(6, Math.min(m.x, innerWidth - r.width - 6)),
        y: Math.max(6, m.y + r.height > innerHeight - 6 ? m.y - r.height : m.y),
      };
      el.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
    });
  });

  async function run(item: MenuItem) {
    menu.close();
    await item.action?.();
  }

  function onkey(e: KeyboardEvent) {
    if (!menu.open || !el) return;
    if (e.key === "Escape") {
      e.preventDefault();
      menu.close();
      return;
    }
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    const items = [...el.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
    const i = items.indexOf(document.activeElement as HTMLButtonElement);
    const next = e.key === "ArrowDown" ? (i + 1) % items.length : (i - 1 + items.length) % items.length;
    items[next]?.focus();
  }
</script>

<svelte:window
  onkeydown={onkey}
  onblur={() => menu.close()}
  onresize={() => menu.close()}
  onwheel={() => menu.close()}
/>

{#if menu.open}
  <div class="veil" role="presentation" onmousedown={() => menu.close()} oncontextmenu={(e) => { e.preventDefault(); menu.close(); }}></div>
  <div class="menu" role="menu" bind:this={el} style:left="{pos.x}px" style:top="{pos.y}px">
    {#each menu.open.items as item, i (i)}
      {#if item === "-"}
        <div class="sep" role="separator"></div>
      {:else}
        <button role="menuitem" class:danger={item.danger} disabled={item.disabled} onclick={() => run(item)}>
          <span class="ic">{#if item.icon}<Icon name={item.icon} size={15} />{/if}</span>
          <span class="lbl">{item.label}</span>
          {#if item.hint}<span class="hint">{item.hint}</span>{/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .veil { position: fixed; inset: 0; z-index: 40; }
  .menu {
    position: fixed; z-index: 41; min-width: 220px; max-width: 360px; padding: 5px;
    display: flex; flex-direction: column; border-radius: 12px;
    background: color-mix(in srgb, var(--raise-2) 92%, transparent); backdrop-filter: blur(16px);
    border: 1px solid var(--line-2); box-shadow: 0 18px 50px -12px rgb(0 0 0 / 0.55);
    animation: pop 0.1s ease-out;
  }
  button {
    display: flex; align-items: center; gap: 10px; height: 32px; padding: 0 10px; border: 0; border-radius: 8px;
    background: none; color: var(--text); cursor: pointer; text-align: left; font-size: 13px; outline: none;
  }
  button:hover:not(:disabled), button:focus-visible { background: var(--hover); }
  button:focus-visible { box-shadow: inset 0 0 0 1px var(--line-2); }
  button:disabled { color: var(--text-3); cursor: default; }
  .ic { width: 16px; display: grid; place-items: center; color: var(--text-3); }
  .lbl { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .hint { font-size: 11px; color: var(--text-3); }
  .danger { color: var(--bad); }
  .danger .ic { color: var(--bad); }
  .sep { height: 1px; margin: 4px 6px; background: var(--line); }
</style>
