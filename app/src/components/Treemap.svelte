<script lang="ts">
  // Mapa de rectángulos (estilo WinDirStat). Cada archivo es un rectángulo
  // proporcional a su tamaño, coloreado por tipo; los cojines agrupan lo que
  // está en la misma carpeta. Clic para entrar en la carpeta de primer nivel.
  import { onMount, untrack } from "svelte";
  import type { Treemap } from "../lib/api";
  import { layoutTreemap, paintTreemap, type Layout } from "../lib/treemap";
  import { formatBytes } from "../lib/format";
  import { t, tn } from "../lib/i18n.svelte";

  interface Props {
    data: Treemap;
    /** Puesto de cada extensión (por espacio) para su color. */
    ranks: Map<string, number>;
    onopen: (id: number) => void;
    /** Id resaltado desde la lista. */
    highlight?: number | null;
    /** Clic derecho sobre un archivo o carpeta. */
    onmenu?: (e: MouseEvent, id: number) => void;
  }
  let { data, ranks, onopen, highlight = null, onmenu }: Props = $props();

  let box: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let size = $state({ w: 0, h: 0, dpr: 1 });
  let lay: Layout | null = null;
  let hit = new Int32Array(0);
  let hover = $state<{ i: number; x: number; y: number } | null>(null);
  let version = $state(0); // cambia al repintar (para los contornos)

  onMount(() => {
    const ro = new ResizeObserver(() => {
      const r = box.getBoundingClientRect();
      size = { w: Math.floor(r.width), h: Math.floor(r.height), dpr: window.devicePixelRatio || 1 };
    });
    ro.observe(box);
    return () => ro.disconnect();
  });

  $effect(() => {
    const { w, h, dpr } = size;
    if (!w || !h || !canvas) return;
    const W = Math.round(w * dpr);
    const H = Math.round(h * dpr);
    canvas.width = W;
    canvas.height = H;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const rank = new Int32Array(data.exts.length);
    data.exts.forEach((e, k) => (rank[k] = ranks.get(e) ?? -1));
    lay = layoutTreemap(data, W, H);
    hit = new Int32Array(W * H);
    const img = ctx.createImageData(W, H);
    paintTreemap(data, lay, rank, img, hit);
    ctx.putImageData(img, 0, 0);
    untrack(() => {
      hover = null;
      version++;
    });
  });

  /** Rectángulo de la pieza `i` en píxeles CSS. */
  function box_of(i: number) {
    void version;
    if (!lay || i < 0) return null;
    const d = size.dpr;
    const r = lay.rects;
    return { x: r[i * 4] / d, y: r[i * 4 + 1] / d, w: (r[i * 4 + 2] - r[i * 4]) / d, h: (r[i * 4 + 3] - r[i * 4 + 1]) / d };
  }

  /** Antepasado de primer nivel (hijo directo de la carpeta actual). */
  function top(i: number) {
    while (i > 0 && data.parent[i] !== 0) i = data.parent[i];
    return i;
  }

  function path(i: number) {
    const parts: string[] = [];
    for (let p = data.parent[i]; p > 0; p = data.parent[p]) parts.push(data.name[p]);
    return parts.reverse().join("\\");
  }

  function move(e: MouseEvent) {
    const r = canvas.getBoundingClientRect();
    const x = e.clientX - r.left;
    const y = e.clientY - r.top;
    const px = Math.floor(x * size.dpr);
    const py = Math.floor(y * size.dpr);
    const i = hit[py * canvas.width + px] ?? -1;
    hover = i >= 0 ? { i, x, y } : null;
  }

  function click() {
    if (!hover) return;
    const tp = top(hover.i);
    if (tp > 0 && data.kind[tp] === "dir") onopen(data.id[tp]);
  }

  function context(e: MouseEvent) {
    e.preventDefault();
    if (!hover || !onmenu) return;
    const i = hover.i;
    if (i > 0 && data.kind[i] !== "rest") onmenu(e, data.id[i]);
  }

  const highlighted = $derived(highlight === null ? -1 : data.id.indexOf(highlight));
  const hoverBox = $derived(hover ? box_of(hover.i) : null);
  const groupBox = $derived(hover && top(hover.i) !== hover.i ? box_of(top(hover.i)) : null);
  const listBox = $derived(highlighted > 0 ? box_of(highlighted) : null);
  const tip = $derived.by(() => {
    if (!hover) return null;
    const i = hover.i;
    const kind = data.kind[i];
    const tp = top(i);
    return {
      name: kind === "rest" ? t("ui.sunburst.grouped") : data.name[i],
      size: data.size[i],
      dir: path(i),
      action: tp > 0 && data.kind[tp] === "dir" ? `${t("ui.sunburst.open")}: ${data.name[tp]}` : "",
      left: hover.x > size.w / 2,
      top: hover.y > size.h / 2,
    };
  });
</script>

<div class="wrap" bind:this={box}>
  <canvas
    bind:this={canvas}
    style:width="{size.w}px"
    style:height="{size.h}px"
    onmousemove={move}
    onmouseleave={() => (hover = null)}
    onclick={click}
    oncontextmenu={context}
    aria-label={t("ui.sunburst.aria", { name: data.name[0] })}
  ></canvas>
  {#if listBox}
    <span class="outline list" style:left="{listBox.x}px" style:top="{listBox.y}px" style:width="{listBox.w}px" style:height="{listBox.h}px"></span>
  {/if}
  {#if groupBox}
    <span class="outline group" style:left="{groupBox.x}px" style:top="{groupBox.y}px" style:width="{groupBox.w}px" style:height="{groupBox.h}px"></span>
  {/if}
  {#if hoverBox}
    <span class="outline" style:left="{hoverBox.x}px" style:top="{hoverBox.y}px" style:width="{hoverBox.w}px" style:height="{hoverBox.h}px"></span>
  {/if}
  {#if tip && hover}
    <div
      class="tip"
      style:left="{hover.x}px"
      style:top="{hover.y}px"
      class:flip-x={tip.left}
      class:flip-y={tip.top}
    >
      <strong class="ellipsis">{tip.name}</strong>
      <span class="num">{formatBytes(tip.size)}</span>
      {#if tip.dir}<span class="faint ellipsis mono">{tip.dir}</span>{/if}
      {#if tip.action}<span class="faint">{tip.action}</span>{/if}
    </div>
  {/if}
  <span class="sr-only">{tn("ui.common.files", data.parent.length)}</span>
</div>

<style>
  .wrap { position: relative; width: 100%; height: 100%; min-height: 240px; border-radius: 10px; overflow: hidden; background: #000; }
  canvas { display: block; cursor: pointer; }
  .outline { position: absolute; pointer-events: none; box-sizing: border-box; border: 2px solid #fff; box-shadow: 0 0 0 1px rgb(0 0 0 / 0.5); }
  .outline.group { border: 1px dashed rgb(255 255 255 / 0.75); box-shadow: none; }
  .outline.list { border: 2px solid rgb(255 255 255 / 0.9); background: rgb(255 255 255 / 0.12); }
  .tip {
    position: absolute; pointer-events: none; z-index: 2; transform: translate(14px, 14px);
    display: flex; flex-direction: column; gap: 1px; max-width: 320px; padding: 8px 10px;
    border-radius: 9px; background: var(--raise-2); border: 1px solid var(--line-2);
    box-shadow: 0 10px 30px rgb(0 0 0 / 0.35); font-size: 12px; color: var(--text);
  }
  .tip.flip-x { transform: translate(calc(-100% - 14px), 14px); }
  .tip.flip-y { transform: translate(14px, calc(-100% - 14px)); }
  .tip.flip-x.flip-y { transform: translate(calc(-100% - 14px), calc(-100% - 14px)); }
  .tip strong { font-size: 13px; }
  .mono { font-family: var(--mono); font-size: 11px; }
  .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
</style>
