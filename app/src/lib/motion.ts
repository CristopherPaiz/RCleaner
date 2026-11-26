import { cubicOut } from "svelte/easing";

/** Cambio de icono o texto: el viejo se encoge y difumina, el nuevo crece desde el desenfoque. */
export function morph(_node: Element, { duration = 380, scale = 0.55, blur = 8, rotate = 0 } = {}) {
  return {
    duration,
    easing: cubicOut,
    css: (t: number, u: number) =>
      `opacity:${t};filter:blur(${u * blur}px);transform:scale(${scale + (1 - scale) * t}) rotate(${u * rotate}deg)`,
  };
}
