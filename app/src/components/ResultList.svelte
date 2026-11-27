<script lang="ts">
  // Resultados agrupados por categoría: una fila por categoría (plegable) con
  // su total; dentro, cada hallazgo con riesgo, antigüedad y avisos.
  import { untrack } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import type { Category, Finding, ScanReport } from "../lib/api";
  import { api } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { CATEGORY_ICON, CATEGORY_ORDER, categoryLabel, riskLabel, formatAge, formatBytes } from "../lib/format";
  import { t, tn } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";
  import FileIcon from "./FileIcon.svelte";
  import { folderSvg } from "../lib/icons.svelte";
  import { menu, type MenuEntry } from "../lib/menu.svelte";
  import { copyText } from "../lib/activity.svelte";
  import { bySubgroup, groupCandidates, shortLabel } from "../lib/results";

  interface Props {
    report: ScanReport;
    query?: string;
    sortBy?: "size" | "age";
  }
  let { report, query = "", sortBy = "size" }: Props = $props();

  // Desplegadas al inicio las categorías cortas; las largas, plegadas.
  const open = new SvelteSet<Category>(
    untrack(() => CATEGORY_ORDER.filter((c) => report.findings.filter((f) => f.category === c).length <= 8)),
  );

  const moduleName = (id: string) => app.modules.find((m) => m.id === id)?.name ?? id;

  /** Categorías cuya lista se agrupa por programa (un navegador, su sublista). */
  const BY_MODULE: Category[] = ["browser"];
  /** Sub-grupos plegados (por id de módulo). */
  const closedSub = new SvelteSet<string>();

  const subgroups = (items: Finding[]) => bySubgroup(items, moduleName);

  function moduleIcon(id: string): string | undefined {
    const proc = app.modules.find((m) => m.id === id)?.processes[0];
    return proc ? `app:${proc}` : undefined;
  }

  /** Icono de un hallazgo: carpeta conocida, si no la app del limpiador. */
  function findingIcon(f: Finding): { src?: string; key?: string } {
    const last = f.path?.split("\\").filter(Boolean).pop();
    const src = last ? folderSvg(last) : undefined;
    if (src) return { src };
    const proc = app.modules.find((m) => m.id === f.module_id)?.processes[0];
    return proc ? { key: `app:${proc}` } : {};
  }

  const groups = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const match = (f: Finding) =>
      !q || f.label.toLowerCase().includes(q) || (f.detail ?? "").toLowerCase().includes(q) || f.module_id.includes(q);
    const order = (a: Finding, b: Finding) =>
      sortBy === "age" ? (b.age_days ?? -1) - (a.age_days ?? -1) || b.bytes - a.bytes : b.bytes - a.bytes;
    return CATEGORY_ORDER.map((c) => {
      const items = report.findings.filter((f) => f.category === c && match(f)).sort(order);
      return { category: c as Category, items, bytes: items.reduce((s, f) => s + f.bytes, 0) };
    })
      .filter((g) => g.items.length > 0)
      .sort((a, b) => b.bytes - a.bytes);
  });

  // Con filtro activo, todo desplegado.
  const isOpen = (c: Category) => query.trim() !== "" || open.has(c);

  function toggleOpen(c: Category) {
    if (open.has(c)) open.delete(c);
    else open.add(c);
  }

  function toggle(f: Finding) {
    if (app.selected.has(f.id)) app.selected.delete(f.id);
    else app.selected.add(f.id);
  }

  function groupState(items: Finding[]) {
    const ok = groupCandidates(items, (f) => app.selectable(f), app.selected);
    const n = ok.filter((f) => app.selected.has(f.id));
    return {
      all: ok.length > 0 && n.length === ok.length,
      some: n.length > 0 && n.length < ok.length,
      bytes: n.reduce((s, f) => s + f.bytes, 0),
      warnings: items.filter((f) => f.warning).length,
    };
  }

  function rowMenu(e: MouseEvent, f: Finding) {
    const on = app.selected.has(f.id);
    const excl = f.path ? app.exclusionFor(f.path) : undefined;
    const items: (MenuEntry | false)[] = [
      on
        ? { label: t("ui.menu.unmark"), icon: "minus", action: () => void app.selected.delete(f.id) }
        : { label: t("ui.menu.mark"), icon: "check", disabled: !app.selectable(f), action: () => void app.selected.add(f.id) },
      "-",
      !!f.path && { label: t("ui.common.show_in_explorer"), icon: "external", action: () => api.reveal(f.path!) },
      !!f.path && { label: t("ui.menu.copy_path"), icon: "copy", action: () => copyText(f.path!) },
      "-",
      !!f.path &&
        (excl
          ? { label: t("ui.menu.unexclude"), icon: "ban", action: () => app.removeExclusion(excl) }
          : {
              label: t("ui.menu.exclude"),
              icon: "shield",
              hint: t("ui.menu.exclude_hint"),
              action: async () => {
                app.selected.delete(f.id);
                await app.addExclusion(f.path!);
              },
            }),
    ];
    menu.show(e, items.filter((x): x is MenuEntry => !!x));
  }

  /** Marcar un grupo entero no marca datos del usuario (cookies, historial…). */
  function toggleGroup(items: Finding[]) {
    const ok = groupCandidates(items, (f) => app.selectable(f), app.selected);
    const all = ok.every((f) => app.selected.has(f.id));
    for (const f of ok) all ? app.selected.delete(f.id) : app.selected.add(f.id);
  }
</script>

{#snippet row(f: Finding, short: boolean)}
  {@const off = !app.selectable(f)}
  {@const ic = findingIcon(f)}
  <div class="row" class:off class:excluded={!!f.path && !!app.exclusionFor(f.path)} oncontextmenu={(e) => rowMenu(e, f)} role="listitem">
    <input
      type="checkbox"
      checked={app.selected.has(f.id)}
      disabled={off}
      onchange={() => toggle(f)}
      aria-label={f.label}
    />
    <span class="f-ic"><FileIcon src={ic.src} key={ic.key} fallback={CATEGORY_ICON[f.category]} size={20} /></span>
    <div class="main">
      <div class="label ellipsis">{short ? shortLabel(f.label, moduleName(f.module_id)) : f.label}</div>
      <div class="detail ellipsis faint">
        {#if f.age_days !== undefined}<span class="age">{formatAge(f.age_days)}</span> · {/if}{f.detail ?? (short ? riskLabel(f.risk) : moduleName(f.module_id))}
      </div>
      {#if f.warning}
        <div class="warning"><Icon name="alert" size={13} /> {f.warning}</div>
      {/if}
      {#if off}<div class="faint small">{t("ui.results.requires_admin")}</div>{/if}
    </div>
    <span class="risk {f.risk}">{riskLabel(f.risk)}</span>
    <span class="size num">{formatBytes(f.bytes)}</span>
    {#if f.path}
      <button class="icon-btn" title={t("ui.common.show_in_explorer")} onclick={() => api.reveal(f.path!)}>
        <Icon name="external" size={15} />
      </button>
    {:else}
      <span></span>
    {/if}
  </div>
{/snippet}

<div class="list">
  {#each groups as g, gi (g.category)}
    {@const st = groupState(g.items)}
    {@const expanded = isOpen(g.category)}
    <section style:animation-delay="{gi * 30}ms">
      <div class="cat" class:expanded>
        <input
          type="checkbox"
          checked={st.all}
          indeterminate={st.some}
          onchange={() => toggleGroup(g.items)}
          aria-label={t("ui.results.select_group", { name: categoryLabel(g.category) })}
        />
        <button class="cat-main" onclick={() => toggleOpen(g.category)} aria-expanded={expanded}>
          <span class="cat-ic"><Icon name={CATEGORY_ICON[g.category]} size={18} /></span>
          <span class="cat-text">
            <span class="cat-name">{categoryLabel(g.category)}</span>
            <span class="cat-sub faint">
              {tn("ui.common.items", g.items.length)}
              {#if st.warnings > 0}<span class="warn-inline"> · {tn("ui.results.warnings", st.warnings)}</span>{/if}
            </span>
          </span>
          <span class="cat-size num">
            {formatBytes(g.bytes)}
            {#if st.some}<small class="faint">{t("ui.results.marked", { size: formatBytes(st.bytes) })}</small>{/if}
          </span>
          <span class="chev" class:rot={expanded}><Icon name="chevron" size={16} /></span>
        </button>
      </div>

      {#if expanded}
        <div class="items">
          {#if BY_MODULE.includes(g.category) && query.trim() === ""}
            {#each subgroups(g.items) as sg (sg.id)}
              {@const sst = groupState(sg.items)}
              {@const sopen = !closedSub.has(sg.id)}
              <div class="sub">
                <div class="sub-head">
                  <input
                    type="checkbox"
                    checked={sst.all}
                    indeterminate={sst.some}
                    onchange={() => toggleGroup(sg.items)}
                    aria-label={t("ui.results.select_group", { name: sg.name })}
                  />
                  <button
                    class="sub-main"
                    aria-expanded={sopen}
                    onclick={() => (sopen ? closedSub.add(sg.id) : closedSub.delete(sg.id))}
                  >
                    <span class="f-ic"><FileIcon key={moduleIcon(sg.id)} fallback={CATEGORY_ICON[g.category]} size={20} /></span>
                    <span class="sub-name">{sg.name}</span>
                    <span class="faint small">{tn("ui.results.options", sg.items.length)}</span>
                    {#if sst.warnings > 0}<span class="warn-inline small"><Icon name="alert" size={12} /></span>{/if}
                    <span class="grow"></span>
                    <span class="size num">{formatBytes(sg.bytes)}</span>
                    <span class="chev" class:rot={sopen}><Icon name="chevron" size={14} /></span>
                  </button>
                </div>
                {#if sopen}
                  <div class="sub-items">
                    {#each sg.items as f (f.id)}
                      {@render row(f, true)}
                    {/each}
                  </div>
                {/if}
              </div>
            {/each}
          {:else}
            {#each g.items as f (f.id)}
              {@render row(f, false)}
            {/each}
          {/if}
        </div>
      {/if}
    </section>
  {:else}
    <p class="faint empty">{t("ui.common.no_match")}</p>
  {/each}

  {#if report.errors.length > 0}
    <details class="errors">
      <summary>{tn("ui.results.errors", report.errors.length)}</summary>
      {#each report.errors as e (e.module_id)}
        <div class="mono">{e.module_id}: {e.message}</div>
      {/each}
    </details>
  {/if}
</div>

<style>
  .list { overflow: auto; flex: 1; padding: 4px 32px 12px; }
  section { border-bottom: 1px solid var(--line); animation: rise 0.25s ease-out backwards; }
  .cat { display: flex; align-items: center; gap: 14px; padding: 0 4px 0 6px; }
  .cat-main {
    flex: 1; display: flex; align-items: center; gap: 14px; min-width: 0;
    padding: 14px 6px; border: 0; background: none; cursor: pointer; text-align: left; border-radius: 10px;
  }
  .cat-main:hover { background: var(--hover); }
  .cat-ic {
    width: 38px; height: 38px; border-radius: 11px; display: grid; place-items: center; flex-shrink: 0;
    color: var(--a); background: color-mix(in srgb, var(--a) 14%, transparent);
  }
  .cat-text { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .cat-name { font-size: 15px; font-weight: 600; }
  .cat-sub { font-size: 12px; }
  .warn-inline { color: var(--warn); }
  .cat-size { display: flex; flex-direction: column; align-items: flex-end; font-size: 15px; font-weight: 600; }
  .cat-size small { font-size: 11px; font-weight: 400; }
  .chev { color: var(--text-3); transition: transform 0.15s; }
  .chev.rot { transform: rotate(90deg); }

  .items { padding: 0 0 10px 58px; }
  .row {
    display: grid; grid-template-columns: 18px 22px 1fr 92px 84px 30px; gap: 12px; align-items: center;
    padding: 9px 10px; border-radius: 9px;
  }
  .row:hover { background: var(--hover); }
  .row.off { opacity: 0.55; }
  .row.excluded .label { color: var(--excl); font-style: italic; }
  .main { min-width: 0; }
  .f-ic { display: grid; place-items: center; color: var(--text-3); }
  .label { font-weight: 500; }
  .detail { font-size: 12px; }
  .age { color: var(--text-2); }
  .small { font-size: 11px; }
  .warning { display: flex; align-items: center; gap: 5px; font-size: 12px; color: var(--warn); margin-top: 2px; }
  .size { text-align: right; font-weight: 600; }
  .empty { padding: 30px 6px; }
  .sub + .sub { margin-top: 2px; }
  .sub-head { display: flex; align-items: center; gap: 12px; padding: 0 10px; }
  .sub-main {
    flex: 1; display: flex; align-items: center; gap: 10px; min-width: 0; padding: 8px 6px;
    border: 0; background: none; cursor: pointer; text-align: left; border-radius: 9px; color: inherit; font: inherit;
  }
  .sub-main:hover { background: var(--hover); }
  .sub-name { font-weight: 600; }
  .sub-items { padding-left: 34px; border-left: 1px solid var(--line); margin-left: 18px; }
  .grow { flex: 1; }
  .errors { margin: 14px 6px; color: var(--bad); font-size: 13px; }
  .errors summary { cursor: pointer; }
  .errors .mono { color: var(--text-2); margin: 4px 0; }
</style>
