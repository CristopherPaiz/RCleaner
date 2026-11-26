<script lang="ts">
  import { api, type Mode, type Settings } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { LANGUAGES, resolveLang, t, tn } from "../lib/i18n.svelte";
  import Icon from "../components/Icon.svelte";
  import ModeEditor from "../components/ModeEditor.svelte";

  let editor = $state<{ initial: Mode | null; editing: boolean } | null>(null);

  const auto = $derived(app.settings.dev_roots.length === 0);
  const roots = $derived(auto ? (app.info?.auto_dev_roots ?? []) : app.settings.dev_roots);
  const custom = $derived(app.modes.filter((m) => m.custom));
  const builtins = $derived(app.modes.filter((m) => !m.custom));
  const systemLang = $derived(LANGUAGES.find((l) => l.code === resolveLang(navigator.language))?.name ?? "");
  const THEMES = [
    { id: null, label: "ui.settings.theme_system", icon: "monitor" },
    { id: "light", label: "ui.settings.theme_light", icon: "sun" },
    { id: "dark", label: "ui.settings.theme_dark", icon: "moon" },
  ] as const;

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
    if (!confirm(t("ui.settings.delete_mode", { name: m.name }))) return;
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
    <h1>{t("ui.nav.settings")}</h1>
    <p class="faint">{t("ui.settings.saved_in")} <span class="mono">{app.info?.config_dir}</span></p>
  </header>

  <section>
    <h2>{t("ui.settings.appearance")}</h2>
    <div class="group">
      <div class="item">
        <span class="grow">{t("ui.settings.theme")}</span>
        <div class="segmented">
          {#each THEMES as th (th.label)}
            <button class:on={app.settings.theme === th.id} onclick={() => app.setTheme(th.id)}>
              <Icon name={th.icon} size={14} /> {t(th.label)}
            </button>
          {/each}
        </div>
      </div>
      <div class="item">
        <span class="grow">
          {t("ui.settings.language")}
          <span class="faint block">{t("ui.settings.language_hint")}</span>
        </span>
        <select
          class="field"
          value={app.settings.language ?? ""}
          disabled={app.busy}
          onchange={(e) => app.setLanguage((e.currentTarget as HTMLSelectElement).value || null)}
        >
          <option value="">{t("ui.settings.language_system", { name: systemLang })}</option>
          {#each LANGUAGES as l (l.code)}<option value={l.code}>{l.name}</option>{/each}
        </select>
      </div>
    </div>
  </section>

  <section>
    <div class="sec-head">
      <div>
        <h2>{t("ui.settings.dev_roots")}</h2>
        <p class="faint">{t("ui.settings.dev_roots_hint")} <span class="mono">node_modules</span>, <span class="mono">target</span>, <span class="mono">__pycache__</span>, <span class="mono">.venv</span>…</p>
      </div>
      <button class="btn small" onclick={() => addFolder("dev_roots")}><Icon name="plus" size={14} /> {t("ui.common.add")}</button>
    </div>
    <div class="group">
      {#each roots as p (p)}
        <div class="item">
          <Icon name="folder" size={16} />
          <span class="ellipsis grow">{p}</span>
          {#if auto}<span class="faint small">{t("ui.settings.auto")}</span>{/if}
          <button class="icon-btn" title={t("ui.common.remove")} onclick={() => removeFolder("dev_roots", p)}><Icon name="x" size={14} /></button>
        </div>
      {:else}
        <div class="item faint">{t("ui.settings.no_folders")}</div>
      {/each}
    </div>
    {#if !auto}
      <button class="link small" onclick={() => update({ dev_roots: [] })}>{t("ui.settings.back_to_auto")}</button>
    {/if}
  </section>

  <section>
    <div class="sec-head">
      <div>
        <h2>{t("ui.settings.exclusions")}</h2>
        <p class="faint">{t("ui.settings.exclusions_hint")}</p>
      </div>
      <button class="btn small" onclick={() => addFolder("exclusions")}><Icon name="plus" size={14} /> {t("ui.common.add")}</button>
    </div>
    <div class="group">
      {#each app.settings.exclusions as p (p)}
        <div class="item">
          <Icon name="shield" size={16} />
          <span class="ellipsis grow">{p}</span>
          <button class="icon-btn" title={t("ui.common.remove")} onclick={() => removeFolder("exclusions", p)}><Icon name="x" size={14} /></button>
        </div>
      {:else}
        <div class="item faint">{t("ui.settings.no_exclusions")}</div>
      {/each}
    </div>
  </section>

  <section>
    <h2>{t("ui.nav.clean")}</h2>
    <div class="group">
      <div class="item">
        <span class="grow">
          {t("ui.settings.default_mode")}
          <span class="faint block">{t("ui.settings.default_mode_hint")}</span>
        </span>
        <select
          class="field"
          value={app.settings.default_mode ?? ""}
          onchange={(e) => update({ default_mode: (e.currentTarget as HTMLSelectElement).value || null })}
        >
          <option value="">{app.modes.find((m) => m.id === "rapido")?.name ?? "—"}</option>
          {#each app.modes.filter((m) => m.id !== "rapido") as m (m.id)}<option value={m.id}>{m.name}</option>{/each}
        </select>
      </div>
    </div>
  </section>

  <section>
    <div class="sec-head">
      <div>
        <h2>{t("ui.settings.custom_modes")}</h2>
        <p class="faint">{t("ui.settings.custom_modes_hint")} <span class="mono">modes</span>.</p>
      </div>
      <button class="btn small accent" onclick={() => (editor = { initial: null, editing: false })}>
        <Icon name="plus" size={14} /> {t("ui.mode_editor.new")}
      </button>
    </div>
    <div class="group">
      {#each custom as m (m.id)}
        <div class="item">
          <Icon name={m.icon || "star"} size={16} />
          <span class="grow">{m.name} <span class="faint">· {tn("ui.settings.selectors", m.modules.length)}</span></span>
          <button class="icon-btn" title={t("ui.common.edit")} onclick={() => (editor = { initial: m, editing: true })}><Icon name="edit" size={14} /></button>
          <button class="icon-btn" title={t("ui.common.delete")} onclick={() => deleteMode(m)}><Icon name="trash" size={14} /></button>
        </div>
      {/each}
      <div class="item dup">
        <span class="faint">{t("ui.settings.start_from")}</span>
        {#each builtins as m (m.id)}
          <button class="btn quiet small" onclick={() => (editor = { initial: m, editing: false })}>
            <Icon name={m.icon || "star"} size={14} /> {m.name}
          </button>
        {/each}
      </div>
    </div>
    {#each app.modeErrors as e (e)}<p class="error">{e}</p>{/each}
  </section>

  <section>
    <h2>{t("ui.settings.about")}</h2>
    <div class="group">
      <div class="item">
        <span class="grow">RCleaner {app.info?.version} <span class="faint">· {tn("ui.settings.module_count", app.modules.length)}</span></span>
        {#if app.info}
          <button class="btn small" onclick={() => api.reveal(app.info!.config_dir)}><Icon name="external" size={14} /> {t("ui.settings.config_folder")}</button>
        {/if}
      </div>
    </div>
  </section>
</div>

{#if editor}
  <ModeEditor initial={editor.initial} editing={editor.editing} onsave={saveMode} oncancel={() => (editor = null)} />
{/if}

<style>
  .page { flex: 1; overflow: auto; padding: 4px 32px 40px; display: flex; flex-direction: column; gap: 28px; max-width: 900px; width: 100%; margin: 0 auto; }
  header h1 { font-size: 28px; margin-bottom: 4px; }
  h2 { font-size: 15px; margin-bottom: 4px; }
  section > h2 { margin-bottom: 10px; }
  .sec-head { display: flex; justify-content: space-between; align-items: flex-end; gap: 16px; margin-bottom: 10px; }
  .sec-head p { font-size: 13px; }
  .group { display: flex; flex-direction: column; gap: 2px; border-radius: 12px; overflow: hidden; }
  .item {
    display: flex; align-items: center; gap: 12px; padding: 12px 14px; min-height: 52px;
    background: var(--raise); color: var(--text-3);
  }
  .item .grow { color: var(--text); }
  .grow { flex: 1; min-width: 0; }
  .block { display: block; font-size: 12px; }
  .small { font-size: 12px; }
  .link.small { margin-top: 8px; }
  .dup { flex-wrap: wrap; gap: 4px; }
  .dup > .faint { margin-right: 6px; font-size: 13px; }
  .field { min-width: 200px; }
  .error { color: var(--bad); font-size: 13px; margin-top: 8px; }
</style>
