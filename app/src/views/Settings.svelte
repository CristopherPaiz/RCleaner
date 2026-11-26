<script lang="ts">
  import { api, type Mode, type Settings } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import Icon from "../components/Icon.svelte";
  import ModeEditor from "../components/ModeEditor.svelte";

  let editor = $state<{ initial: Mode | null; editing: boolean } | null>(null);

  const auto = $derived(app.settings.dev_roots.length === 0);
  const roots = $derived(auto ? (app.info?.auto_dev_roots ?? []) : app.settings.dev_roots);
  const custom = $derived(app.modes.filter((m) => m.custom));
  const builtins = $derived(app.modes.filter((m) => !m.custom));

  async function update(patch: Partial<Settings>) {
    try {
      await app.saveSettings({ ...app.settings, ...patch });
    } catch (e) {
      app.error = String(e);
    }
  }

  async function addFolder(key: "dev_roots" | "exclusions") {
    const dir = await api.pickFolder();
    if (!dir) return;
    const base = key === "dev_roots" && auto ? roots : app.settings[key];
    if (base.some((p) => p.toLowerCase() === dir.toLowerCase())) return;
    await update({ [key]: [...base, dir] });
  }

  async function removeFolder(key: "dev_roots" | "exclusions", path: string) {
    const base = key === "dev_roots" && auto ? roots : app.settings[key];
    await update({ [key]: base.filter((p) => p !== path) });
  }

  async function saveMode(m: Mode) {
    await api.saveMode(m);
    await app.reloadModes();
    editor = null;
  }

  async function deleteMode(m: Mode) {
    if (!confirm(`¿Borrar el modo "${m.name}"?`)) return;
    try {
      await api.deleteMode(m.id);
      await app.reloadModes();
      if (app.settings.default_mode === m.id) await update({ default_mode: null });
    } catch (e) {
      app.error = String(e);
    }
  }
</script>

<div class="page">
  <header>
    <h1>Ajustes</h1>
    <p class="muted">Se guardan al momento en <span class="mono">{app.info?.config_dir}</span></p>
  </header>

  <section class="card">
    <div class="head">
      <div>
        <h2>Carpetas de proyectos</h2>
        <p class="muted">Dónde busca <span class="mono">node_modules</span>, <span class="mono">target</span>, <span class="mono">__pycache__</span>, <span class="mono">.venv</span>… (módulo "Artefactos de proyectos").</p>
      </div>
      <button class="ghost sm" onclick={() => addFolder("dev_roots")}><Icon name="plus" size={14} /> Añadir</button>
    </div>
    {#if auto}<p class="badge">Detección automática</p>{/if}
    <ul>
      {#each roots as p (p)}
        <li>
          <Icon name="folder" size={15} />
          <span class="path">{p}</span>
          <button class="icon-btn" title="Quitar" onclick={() => removeFolder("dev_roots", p)}><Icon name="x" size={14} /></button>
        </li>
      {:else}
        <li class="muted">Ninguna carpeta.</li>
      {/each}
    </ul>
    {#if !auto}
      <button class="link" onclick={() => update({ dev_roots: [] })}>Volver a detección automática</button>
    {/if}
  </section>

  <section class="card">
    <div class="head">
      <div>
        <h2>Exclusiones</h2>
        <p class="muted">Carpetas que Clean-CRP nunca analiza ni borra (ni su contenido).</p>
      </div>
      <button class="ghost sm" onclick={() => addFolder("exclusions")}><Icon name="plus" size={14} /> Añadir</button>
    </div>
    <ul>
      {#each app.settings.exclusions as p (p)}
        <li>
          <Icon name="shield" size={15} />
          <span class="path">{p}</span>
          <button class="icon-btn" title="Quitar" onclick={() => removeFolder("exclusions", p)}><Icon name="x" size={14} /></button>
        </li>
      {:else}
        <li class="muted">Sin exclusiones.</li>
      {/each}
    </ul>
  </section>

  <section class="card">
    <h2>Modo al abrir</h2>
    <select
      value={app.settings.default_mode ?? ""}
      onchange={(e) => update({ default_mode: (e.currentTarget as HTMLSelectElement).value || null })}
    >
      <option value="">Rápido (por defecto)</option>
      {#each app.modes as m (m.id)}<option value={m.id}>{m.name}</option>{/each}
    </select>
  </section>

  <section class="card">
    <div class="head">
      <div>
        <h2>Modos personalizados</h2>
        <p class="muted">Combina los módulos que quieras. Se guardan como TOML en <span class="mono">modes\</span>.</p>
      </div>
      <button class="primary sm" onclick={() => (editor = { initial: null, editing: false })}>
        <Icon name="plus" size={14} /> Nuevo modo
      </button>
    </div>
    <ul>
      {#each custom as m (m.id)}
        <li>
          <Icon name={m.icon || "star"} size={15} />
          <span class="path"><strong>{m.name}</strong> <span class="muted">· {m.modules.length} selectores</span></span>
          <button class="icon-btn" title="Editar" onclick={() => (editor = { initial: m, editing: true })}><Icon name="edit" size={14} /></button>
          <button class="icon-btn" title="Borrar" onclick={() => deleteMode(m)}><Icon name="trash" size={14} /></button>
        </li>
      {:else}
        <li class="muted">Aún no tienes modos propios. Puedes partir de uno incluido:</li>
      {/each}
    </ul>
    <div class="dup">
      {#each builtins as m (m.id)}
        <button class="ghost sm" onclick={() => (editor = { initial: m, editing: false })}>Duplicar "{m.name}"</button>
      {/each}
    </div>
    {#each app.modeErrors as e (e)}<p class="error">{e}</p>{/each}
  </section>

  <section class="card about">
    <h2>Acerca de</h2>
    <p>Clean-CRP v{app.info?.version} · {app.modules.length} módulos de limpieza</p>
    {#if app.info}
      <button class="link" onclick={() => api.reveal(app.info!.config_dir)}>Abrir carpeta de ajustes</button>
    {/if}
  </section>
</div>

{#if editor}
  <ModeEditor initial={editor.initial} editing={editor.editing} onsave={saveMode} oncancel={() => (editor = null)} />
{/if}

<style>
  .page { padding: 22px 28px 32px; overflow: auto; display: flex; flex-direction: column; gap: 14px; }
  header h1 { margin: 0 0 4px; font-size: 22px; }
  header p { margin: 0 0 4px; }
  .card { background: var(--panel); border: 1px solid var(--border); border-radius: 12px; padding: 16px 18px; }
  h2 { font-size: 15px; margin: 0 0 4px; }
  .card p { margin: 0 0 10px; font-size: 13px; }
  .head { display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; }
  .head button { flex-shrink: 0; display: flex; align-items: center; gap: 4px; }
  .badge { display: inline-block; font-size: 12px; padding: 2px 8px; border-radius: 99px; background: var(--accent-bg); color: var(--accent); }
  ul { list-style: none; margin: 0 0 8px; padding: 0; }
  li { display: flex; align-items: center; gap: 8px; padding: 7px 0; border-bottom: 1px solid var(--border-soft); color: var(--muted); }
  li:last-child { border-bottom: 0; }
  .path { flex: 1; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .icon-btn { background: none; border: 0; color: var(--muted); cursor: pointer; display: grid; place-items: center; padding: 4px; border-radius: 6px; }
  .icon-btn:hover { background: var(--hover); color: var(--danger); }
  select { font: inherit; color: var(--text); background: var(--bg); border: 1px solid var(--border); border-radius: 8px; padding: 7px 10px; min-width: 240px; }
  .dup { display: flex; flex-wrap: wrap; gap: 6px; }
  .error { color: var(--danger); }
</style>
