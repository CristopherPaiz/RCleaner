<script lang="ts">
  // Mapa radial: el centro es la carpeta actual; cada anillo, un nivel más.
  // Clic en una carpeta para entrar; clic en el centro para subir.
  import type { ViewNode } from "../lib/api";
  import { CENTER, layout, type Arc } from "../lib/sunburst";
  import { formatBytes } from "../lib/format";
  import { t, tn } from "../lib/i18n.svelte";

  interface Props {
    view: ViewNode;
    rings: number;
    canUp: boolean;
    onopen: (id: number) => void;
    onup: () => void;
    /** Id resaltado desde la lista. */
    highlight?: number | null;
  }
  let { view, rings, canUp, onopen, onup, highlight = null }: Props = $props();

  const geo = $derived(layout(view, rings));
  let hover = $state<Arc | null>(null);

  const label = $derived(
    hover
      ? { name: hover.name, size: hover.size, sub: t(hover.kind === "dir" ? "ui.sunburst.open" : hover.kind === "rest" ? "ui.sunburst.grouped" : "ui.sunburst.file") }
      : { name: view.name, size: view.size, sub: tn("ui.common.files", view.items) },
  );

  /** ¿El arco `key` es `base` o está dentro de él? */
  function within(key: string, base: string) {
    return key === base || key.startsWith(base + "/");
  }

  function click(a: Arc) {
    if (a.kind === "dir" && a.id !== null) onopen(a.id);
  }
</script>

<div class="wrap">
  <svg viewBox="-1 -1 2 2" role="img" aria-label={t("ui.sunburst.aria", { name: view.name })}>
    {#key view.id}
      <g class="rings">
        {#each geo.arcs as a (a.key)}
          <path
            d={a.d}
            fill={a.color}
            class:dir={a.kind === "dir"}
            class:dim={(hover && !within(a.key, hover.key)) || (highlight !== null && !within(a.key, `/${highlight}`))}
            role="presentation"
            onmouseenter={() => (hover = a)}
            onmouseleave={() => (hover = null)}
            onclick={() => click(a)}
          />
        {/each}
      </g>
    {/key}
    <circle
      r={CENTER - 0.02}
      class="hub"
      class:up={canUp}
      role="presentation"
      onclick={() => canUp && onup()}
    />
  </svg>
  <div class="center" aria-live="polite">
    <span class="c-size num">{formatBytes(label.size)}</span>
    <span class="c-name ellipsis">{label.name}</span>
    <span class="c-sub faint">{label.sub}</span>
    {#if canUp && !hover}<span class="c-up faint">↑ {t("ui.sunburst.up")}</span>{/if}
  </div>
</div>

<style>
  .wrap { position: relative; width: 100%; aspect-ratio: 1; max-width: 520px; max-height: 100%; margin: 0 auto; }
  svg { width: 100%; height: 100%; display: block; overflow: visible; }
  .rings { animation: grow 0.35s cubic-bezier(0.2, 0.8, 0.2, 1); transform-origin: 0 0; }
  @keyframes grow { from { transform: scale(0.86) rotate(-8deg); opacity: 0; } }
  path { stroke: var(--bg); stroke-width: 0.006; transition: opacity 0.12s, filter 0.12s; }
  path.dir { cursor: pointer; }
  path:hover { filter: brightness(1.15); }
  path.dim { opacity: 0.35; }
  .hub { fill: var(--raise); stroke: var(--line-2); stroke-width: 0.004; }
  .hub.up { cursor: pointer; }
  .hub.up:hover { fill: var(--raise-2); }
  .center {
    position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%);
    width: 26%; display: flex; flex-direction: column; align-items: center; text-align: center; pointer-events: none;
  }
  .c-size { font-family: var(--font-display); font-size: clamp(15px, 2.2vw, 22px); font-weight: 600; }
  .c-name { max-width: 100%; font-size: 12px; color: var(--text-2); }
  .c-sub { font-size: 11px; }
  .c-up { font-size: 11px; margin-top: 2px; }
</style>
