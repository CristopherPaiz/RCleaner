<script lang="ts">
  import { untrack } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import type { Category, Mode } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { CATEGORY_LABEL, CATEGORY_ORDER, MODE_ICONS, RISK_LABEL, slugify } from "../lib/format";
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
  let name = $state(isEdit ? (start?.name ?? "") : start ? `${start.name} (copia)` : "");
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
    if (!name.trim()) return void (error = "Ponle un nombre.");
    if (!id) return void (error = "El nombre necesita letras o números.");
    if (chosen.size === 0) return void (error = "Elige al menos un módulo.");
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
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Editor de modo">
    <h2>{isEdit ? "Editar modo" : "Nuevo modo"}</h2>

    <div class="fields">
      <label>
        Nombre
        <input bind:value={name} placeholder="Mi limpieza semanal" maxlength="60" />
        <span class="muted small">id: {id || "—"}</span>
      </label>
      <label>
        Descripción
        <input bind:value={description} placeholder="Qué limpia y cuándo usarlo" />
      </label>
      <div>
        <span>Icono</span>
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
            <strong>{CATEGORY_LABEL[g.category]}</strong>
            <span class="muted small">{n}/{ids.length}</span>
          </label>
          {#each g.items as m (m.id)}
            <label class="mod" title={m.description}>
              <input
                type="checkbox"
                checked={chosen.has(m.id)}
                onchange={() => (chosen.has(m.id) ? chosen.delete(m.id) : chosen.add(m.id))}
              />
              <span>{m.name}</span>
              <span class="tag {m.risk}">{RISK_LABEL[m.risk]}</span>
              {#if m.requires_admin}<span class="tag admin">Admin</span>{/if}
            </label>
          {/each}
        </section>
      {/each}
    </div>

    {#if error}<p class="error">{error}</p>{/if}

    <div class="buttons">
      <span class="muted small">{chosen.size} módulos</span>
      <button class="ghost" onclick={oncancel} disabled={saving}>Cancelar</button>
      <button class="primary" onclick={save} disabled={saving}>{saving ? "Guardando…" : "Guardar"}</button>
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: rgb(0 0 0 / 0.45); display: grid; place-items: center; z-index: 10; }
  .dialog { width: min(760px, 94vw); max-height: 90vh; display: flex; flex-direction: column; background: var(--panel); border: 1px solid var(--border); border-radius: 14px; padding: 20px 22px; }
  h2 { margin: 0 0 14px; }
  .fields { display: grid; grid-template-columns: 1fr 1fr; gap: 12px 16px; margin-bottom: 14px; }
  .fields > div { grid-column: 1 / -1; }
  label { display: flex; flex-direction: column; gap: 4px; }
  input:not([type]) { font: inherit; color: var(--text); background: var(--bg); border: 1px solid var(--border); border-radius: 8px; padding: 7px 10px; }
  .small { font-size: 12px; }
  .icons { display: flex; gap: 6px; margin-top: 4px; }
  .ic { width: 36px; height: 36px; display: grid; place-items: center; border-radius: 9px; border: 1px solid var(--border); background: var(--bg); color: var(--muted); cursor: pointer; }
  .ic.on { border-color: var(--accent); color: var(--accent); background: var(--accent-bg); }
  .modules { overflow: auto; display: grid; grid-template-columns: 1fr 1fr; gap: 12px; padding-right: 4px; }
  section { border: 1px solid var(--border); border-radius: 10px; padding: 8px 10px; }
  .group, .mod { flex-direction: row; align-items: center; gap: 8px; cursor: pointer; }
  .group { padding: 4px 0 8px; border-bottom: 1px solid var(--border-soft); margin-bottom: 4px; }
  .group strong { flex: 1; }
  .mod { padding: 4px 0; font-size: 13px; }
  .mod span:first-of-type { flex: 1; }
  .tag { font-size: 10px; padding: 1px 7px; border-radius: 99px; }
  .tag.safe { background: var(--safe-bg); color: var(--safe); }
  .tag.moderate { background: var(--warn-bg); color: var(--warn); }
  .tag.advanced { background: var(--danger-bg); color: var(--danger); }
  .tag.admin { background: var(--hover); color: var(--muted); }
  .error { color: var(--danger); margin: 10px 0 0; }
  .buttons { display: flex; align-items: center; justify-content: flex-end; gap: 8px; margin-top: 16px; }
  .buttons .muted { margin-right: auto; }
</style>
