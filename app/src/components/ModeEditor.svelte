<script lang="ts">
  import { untrack } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import type { Category, Mode } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { CATEGORY_ORDER, MODE_ICONS, categoryLabel, riskLabel, slugify } from "../lib/format";
  import { t, tn } from "../lib/i18n.svelte";
  import { compact, resolve } from "../lib/selectors";
  import Icon from "./Icon.svelte";

  interface Props {
    /** Modo a editar (custom) o plantilla para uno nuevo. */
    initial: Mode | null;
    /** `true` si se edita un modo existente (id fijo). */
    editing: boolean;
    onsave: (m: Mode) => Promise<void>;
    oncancel: () => void;
  }
  let { initial, editing, onsave, oncancel }: Props = $props();

  // Copia local editable de los valores iniciales: el diálogo se monta de nuevo
  // en cada edición, así que leerlos una sola vez (untrack) es lo correcto.
  const start = untrack(() => $state.snapshot(initial));
  const isEdit = untrack(() => editing);
  let name = $state(isEdit ? (start?.name ?? "") : start ? t("ui.mode_editor.copy_of", { name: start.name }) : "");
  let description = $state(start?.description ?? "");
  let icon = $state(start?.icon || "star");
  const chosen = new SvelteSet<string>(start ? resolve(start.modules, app.modules) : []);
  let saving = $state(false);
  let error = $state<string | null>(null);

  const id = $derived(isEdit && start ? start.id : slugify(name));
  const groups = $derived(
    CATEGORY_ORDER.map((c) => ({ category: c as Category, items: app.modules.filter((m) => m.category === c) })).filter(
      (g) => g.items.length > 0,
    ),
  );

  function toggleGroup(ids: string[]) {
    const all = ids.every((i) => chosen.has(i));
    for (const i of ids) all ? chosen.delete(i) : chosen.add(i);
  }

  async function save() {
    error = null;
    if (!name.trim()) return void (error = t("ui.mode_editor.err_name"));
    if (!id) return void (error = t("ui.mode_editor.err_id"));
    if (chosen.size === 0) return void (error = t("ui.mode_editor.err_modules"));
    saving = true;
    try {
      await onsave({
        id,
        name: name.trim(),
        description: description.trim(),
        icon,
        modules: compact(chosen, app.modules),
        custom: true,
      });
    } catch (e) {
      error = String(e);
      saving = false;
    }
  }
</script>

<div class="backdrop" role="presentation">
  <div class="dialog" role="dialog" aria-modal="true" aria-label={t("ui.mode_editor.aria")}>
    <h2>{t(isEdit ? "ui.mode_editor.edit" : "ui.mode_editor.new")}</h2>

    <div class="fields">
      <label>
        {t("ui.mode_editor.name")}
        <input bind:value={name} placeholder={t("ui.mode_editor.name_ph")} maxlength="60" />
        <span class="faint small">id: {id || "—"}</span>
      </label>
      <label>
        {t("ui.mode_editor.desc")}
        <input bind:value={description} placeholder={t("ui.mode_editor.desc_ph")} />
      </label>
      <div>
        <span>{t("ui.mode_editor.icon")}</span>
        <div class="icons">
          {#each MODE_ICONS as ic (ic)}
            <button class="ic" class:on={icon === ic} onclick={() => (icon = ic)} aria-label={ic}>
              <Icon name={ic} />
            </button>
          {/each}
        </div>
      </div>
    </div>

    <div class="modules">
      {#each groups as g (g.category)}
        {@const ids = g.items.map((m) => m.id)}
        {@const n = ids.filter((i) => chosen.has(i)).length}
        <section>
          <label class="group">
            <input
              type="checkbox"
              checked={n === ids.length}
              indeterminate={n > 0 && n < ids.length}
              onchange={() => toggleGroup(ids)}
            />
            <strong>{categoryLabel(g.category)}</strong>
            <span class="faint small">{n}/{ids.length}</span>
          </label>
          {#each g.items as m (m.id)}
            <label class="mod" title={m.description}>
              <input
                type="checkbox"
                checked={chosen.has(m.id)}
                onchange={() => (chosen.has(m.id) ? chosen.delete(m.id) : chosen.add(m.id))}
              />
              <span>{m.name}</span>
              <span class="tag {m.risk}">{riskLabel(m.risk)}</span>
              {#if m.requires_admin}<span class="tag admin">{t("ui.admin.label")}</span>{/if}
            </label>
          {/each}
        </section>
      {/each}
    </div>

    {#if error}<p class="error">{error}</p>{/if}

    <div class="buttons">
      <span class="faint small">{tn("ui.common.modules", chosen.size)}</span>
      <button class="btn" onclick={oncancel} disabled={saving}>{t("ui.common.cancel")}</button>
      <button class="btn accent" onclick={save} disabled={saving}>{t(saving ? "ui.common.saving" : "ui.common.save")}</button>
    </div>
  </div>
</div>

<style>
  .backdrop { z-index: 30; }
  .dialog { width: min(780px, 94vw); }
  h2 { margin-bottom: 18px !important; }
  .fields { display: grid; grid-template-columns: 1fr 1fr; gap: 14px 16px; margin-bottom: 16px; }
  .fields > div { grid-column: 1 / -1; }
  label { display: flex; flex-direction: column; gap: 6px; font-size: 13px; color: var(--text-2); }
  input:not([type]) { font: inherit; color: var(--text); background: var(--bg); border: 1px solid var(--line-2); border-radius: 9px; padding: 8px 11px; outline: 0; }
  input:not([type]):focus { border-color: color-mix(in srgb, var(--a) 70%, transparent); }
  .small { font-size: 12px; }
  .icons { display: flex; gap: 6px; margin-top: 6px; }
  .ic { width: 38px; height: 38px; display: grid; place-items: center; border-radius: 10px; border: 1px solid var(--line); background: var(--bg); color: var(--text-3); cursor: pointer; }
  .ic:hover { color: var(--text); }
  .ic.on { border-color: transparent; color: #fff; background: linear-gradient(145deg, var(--a), var(--b)); }
  .modules { overflow: auto; display: grid; grid-template-columns: 1fr 1fr; gap: 10px 18px; padding-right: 4px; }
  section { padding: 4px 0; }
  .group, .mod { flex-direction: row; align-items: center; gap: 10px; cursor: pointer; color: var(--text); }
  .group { padding: 6px 0 8px; border-bottom: 1px solid var(--line); margin-bottom: 4px; }
  .group strong { flex: 1; }
  .mod { padding: 5px 0; font-size: 13px; }
  .mod span:first-of-type { flex: 1; }
  .tag { font-size: 11px; color: var(--text-3); }
  .tag.safe { color: var(--ok); }
  .tag.moderate { color: var(--warn); }
  .tag.advanced { color: var(--bad); }
  .error { color: var(--bad); margin: 10px 0 0; font-size: 13px; }
  .buttons { display: flex; align-items: center; justify-content: flex-end; gap: 8px; margin-top: 18px; }
  .buttons .faint { margin-right: auto; }
</style>
