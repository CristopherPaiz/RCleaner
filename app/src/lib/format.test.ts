import { afterEach, describe, expect, it } from "vitest";
import { fileIcon, formatBytes, splitPath } from "./format";
import { i18n } from "./i18n.svelte";

const lang = i18n.lang;
afterEach(() => {
  i18n.lang = lang;
});

describe("formatBytes", () => {
  it("usa unidades binarias y el separador del idioma", () => {
    i18n.lang = "en";
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1023)).toBe("1023 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(5 * 1024 ** 3)).toBe("5 GB");
    i18n.lang = "es";
    expect(formatBytes(1536)).toBe("1,5 KB");
  });

  it("sin decimales a partir de 100", () => {
    i18n.lang = "en";
    expect(formatBytes(150.4 * 1024 ** 2)).toBe("150 MB");
  });
});

describe("rutas y tipos", () => {
  it("separa carpeta y nombre", () => {
    expect(splitPath("C:\\Users\\a\\video.mp4")).toEqual(["C:\\Users\\a", "video.mp4"]);
    expect(splitPath("archivo")).toEqual(["", "archivo"]);
  });

  it("elige el icono por extensión", () => {
    expect(fileIcon("a.MP4")).toBe("film");
    expect(fileIcon("foto.jpeg")).toBe("image");
    expect(fileIcon("disco.iso")).toBe("archive");
    expect(fileIcon("sin-extension")).toBe("file");
  });
});
