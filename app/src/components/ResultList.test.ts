import { fireEvent, render, within } from "@testing-library/svelte";
import { beforeEach, describe, expect, it } from "vitest";
import type { Finding, ModuleMeta, Risk } from "../lib/api";
import { app } from "../lib/app.svelte";
import { i18n } from "../lib/i18n.svelte";
import ResultList from "./ResultList.svelte";

const mod = (id: string, name: string): ModuleMeta => ({
  id,
  name,
  description: "",
  category: "browser",
  risk: "safe",
  requires_admin: false,
  processes: [],
});

const finding = (id: string, module_id: string, label: string, bytes: number, risk: Risk = "safe"): Finding => ({
  id,
  module_id,
  category: "browser",
  label,
  bytes,
  items: 1,
  risk,
  requires_admin: false,
  selected: risk === "safe",
});

const findings = [
  finding("brave:cache", "browser.brave", "Caché de Brave", 300),
  finding("brave:cookies", "browser.brave", "Cookies de Brave", 10, "advanced"),
  finding("chrome:cache", "browser.chrome", "Caché de Google Chrome", 100),
  finding("chrome:sw", "browser.chrome", "Service workers de Google Chrome", 50, "moderate"),
];

beforeEach(() => {
  i18n.lang = "es";
  app.modules = [mod("browser.brave", "Brave"), mod("browser.chrome", "Google Chrome")];
  app.selected.clear();
});

const renderList = () => render(ResultList, { report: { findings, errors: [], elapsed_ms: 1 } });
const texts = (root: ParentNode, sel: string) => [...root.querySelectorAll(sel)].map((e) => e.textContent?.trim());

describe("ResultList: navegadores", () => {
  it("cada navegador tiene su sublista, el más grande primero", () => {
    const { container } = renderList();
    expect(texts(container, ".sub-name")).toEqual(["Brave", "Google Chrome"]);
    expect(container.querySelector(".sub-main")?.textContent).toContain("2 opciones");
  });

  it("dentro de cada navegador las filas no repiten su nombre", () => {
    const { container } = renderList();
    expect(texts(container, ".sub-items .label")).toEqual(["Caché", "Cookies", "Caché", "Service workers"]);
  });

  it("la casilla del navegador no marca sus cookies", async () => {
    const { container } = renderList();
    const brave = container.querySelector<HTMLElement>(".sub")!;
    await fireEvent.click(within(brave).getAllByRole("checkbox")[0]);
    expect([...app.selected].sort()).toEqual(["brave:cache"]);
  });

  it("se puede plegar un navegador", async () => {
    const { container } = renderList();
    const toggle = container.querySelector<HTMLElement>(".sub-main")!;
    expect(container.querySelectorAll(".sub-items").length).toBe(2);
    await fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(container.querySelectorAll(".sub-items").length).toBe(1);
  });
});
