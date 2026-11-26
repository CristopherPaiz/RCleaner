<script lang="ts">
  import type { Category, Finding, ScanReport } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { api } from "../lib/api";
  import { CATEGORY_LABEL, CATEGORY_ORDER, RISK_LABEL, formatAge, formatBytes } from "../lib/format";
  import Icon from "./Icon.svelte";

  interface Props {
    report: ScanReport;
    query?: string;
    sortBy?: "size" | "age";
  }
  let { report, query = "", sortBy = "size" }: Props = $props();

  const groups = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const match = (f: Finding) =>
      !q || f.label.toLowerCase().includes(q) || (f.detail ?? "").toLowerCase().includes(q) || f.module_id.includes(q);
    const order = (a: Finding, b: Finding) =>
      sortBy === "age" ? (b.age_days ?? -1) - (a.age_days ?? -1) || b.bytes - a.bytes : b.bytes - a.bytes;
    return CATEGORY_ORDER.map((c) => {
      const items = report.findings.filter((f) => f.category === c && match(f)).sort(order);
      return { category: c as Category, items, bytes: items.reduce((s, f) => s + f.bytes, 0) };
    }).filter((g) => g.items.length > 0);
  });

  function toggle(f: Finding) {
    if (app.selected.has(f.id)) app.selected.delete(f.id);
    else app.selected.add(f.id);
  }

  function groupState(items: Finding[]) {
    const ok = items.filter((f) => app.selectable(f));
    const n = ok.filter((f) => app.selected.has(f.id)).length;
    return { all: ok.length > 0 && n === ok.length, some: n > 0 && n < ok.length };
  }

  function toggleGroup(items: Finding[]) {
    const ok = items.filter((f) => app.selectable(f));
    const all = ok.every((f) => app.selected.has(f.id));
    for (const f of ok) all ? app.selected.delete(f.id) : app.selected.add(f.id);
  }
</script>

<div class="list">
  {#each groups as g (g.category)}
    {@const st = groupState(g.items)}
    <section>
      <label class="group">
        <input type="checkbox" checked={st.all} indeterminate={st.some} onchange={() => toggleGroup(g.items)} />
        <span class="title">{CATEGORY_LABEL[g.category]}</span>
        <span class="muted count">{g.items.length}</span>
        <span class="size">{formatBytes(g.bytes)}</span>
      </label>
      {#each g.items as f (f.id)}
        <div class="row" class:off={!app.selectable(f)}>
          <input
            type="checkbox"
            checked={app.selected.has(f.id)}
            disabled={!app.selectable(f)}
            onchange={() => toggle(f)}
            aria-label={f.label}
          />
          <div class="main">
            <div class="label">{f.label}</div>
            <div class="detail">
              {#if f.age_days !== undefined}<span class="age">{formatAge(f.age_days)}</span>{/if}
              {f.detail ?? ""}
            </div>
            {#if f.warning}
              <div class="warning"><Icon name="alert" size={13} /> {f.warning}</div>
            {/if}
          </div>
          {#if f.requires_admin}<span class="tag admin">Admin</span>{/if}
          <span class="tag {f.risk}">{RISK_LABEL[f.risk]}</span>
          <span class="size">{formatBytes(f.bytes)}</span>
          {#if f.path}
            <button class="reveal" title="Abrir en el Explorador" onclick={() => api.reveal(f.path!)}>
              <Icon name="external" size={15} />
            </button>
          {:else}
            <span></span>
          {/if}
        </div>
      {/each}
    </section>
  {:else}
    <p class="muted">Nada coincide con el filtro.</p>
  {/each}

  {#each report.errors as e (e.module_id)}
    <div class="err">{e.module_id}: {e.message}</div>
  {/each}
</div>

<style>
  .list { overflow: auto; padding: 0 28px 20px; flex: 1; }
  section { margin-bottom: 16px; border: 1px solid var(--border); border-radius: 12px; background: var(--panel); overflow: hidden; }
  .group { display: flex; align-items: center; gap: 10px; padding: 11px 14px; border-bottom: 1px solid var(--border); cursor: pointer; position: sticky; top: 0; background: var(--panel); z-index: 1; }
  .group .title { font-weight: 600; }
  .count { flex: 1; font-size: 12px; }
  .row { display: grid; grid-template-columns: 20px 1fr auto auto 90px 28px; gap: 10px; align-items: center; padding: 8px 14px; border-bottom: 1px solid var(--border-soft); }
  .row:last-child { border-bottom: 0; }
  .row.off { opacity: 0.55; }
  .main { min-width: 0; }
  .label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .detail { font-size: 12px; color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .age { color: var(--text); margin-right: 6px; }
  .warning { display: flex; align-items: center; gap: 5px; font-size: 12px; color: var(--warn); margin-top: 2px; }
  .size { font-variant-numeric: tabular-nums; text-align: right; font-weight: 600; }
  .tag { font-size: 11px; padding: 2px 8px; border-radius: 99px; white-space: nowrap; }
  .tag.safe { background: var(--safe-bg); color: var(--safe); }
  .tag.moderate { background: var(--warn-bg); color: var(--warn); }
  .tag.advanced { background: var(--danger-bg); color: var(--danger); }
  .tag.admin { background: var(--hover); color: var(--muted); }
  .reveal { background: none; border: 0; color: var(--muted); cursor: pointer; display: grid; place-items: center; }
  .reveal:hover { color: var(--accent); }
  .err { font-size: 12px; color: var(--danger); padding: 4px 0; }
</style>
