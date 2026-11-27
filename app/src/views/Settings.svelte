<script lang="ts">
  import { api, type Mode, type Settings } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { update as upd } from "../lib/update.svelte";
  import { LANGUAGES, i18n, resolveLang, t, tn } from "../lib/i18n.svelte";
  import Icon from "../components/Icon.svelte";
  import ModeEditor from "../components/ModeEditor.svelte";
  import Switch from "../components/Switch.svelte";
  import Modal from "../components/Modal.svelte";
  import DeletionLog from "../components/DeletionLog.svelte";
  import { formatBytes, formatDate } from "../lib/format";
  import { onMount } from "svelte";
  import type { BackgroundStatus } from "../lib/api";

  let editor = $state<{ initial: Mode | null; editing: boolean } | null>(null);
  let askSystem = $state(false);
  let agreed = $state(false);
  let showLog = $state(false);
  /** Opciones de cuántos borrados guardar. */
  const LOG_SIZES = [0, 100, 1000, 10000, 100000];
  /** Cambio de tamaño del registro pendiente de confirmar (borraría registros). */
  let askLog = $state<{ keep: number; drop: number } | null>(null);

  async function setLogSize(keep: number) {
    const count = await api.deletionsCount();
    if (count > keep) {
      askLog = { keep, drop: count - keep };
      return;
    }
    await update({ deletion_log: keep });
  }

  async function clearLog() {
    const count = await api.deletionsCount();
    askLog = { keep: -1, drop: count };
  }

  async function applyLog() {
    if (!askLog) return;
    const { keep } = askLog;
    askLog = null;
    try {
      await api.trimDeletions(Math.max(keep, 0));
      if (keep >= 0) await update({ deletion_log: keep });
    } catch (e) {
      app.error = String(e);
    }
  }
  let bgStatus = $state<BackgroundStatus | null>(null);

  onMount(() => {
    void api.backgroundStatus().then((s) => (bgStatus = s));
  });

  const HOURS = [1, 3, 6, 12, 24, 72];
  const THRESHOLDS = [512, 1024, 2048, 5120, 10240];

  function setBackground(patch: Partial<Settings["background"]>) {
    void update({ background: { ...app.settings.background, ...patch } });
  }

  function toggleSystem(on: boolean) {
    if (!on) void update({ allow_system: false });
    else {
      agreed = false;
      askSystem = true;
    }
  }

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
    <h2>{t("ui.settings.system")}</h2>
    <div class="group">
      <div class="item">
        <span class="grow">
          {t("ui.settings.always_admin")}
          <span class="faint block">{t("ui.settings.always_admin_hint")}</span>
        </span>
        <Switch checked={app.settings.always_admin} label={t("ui.settings.always_admin")} onchange={(v) => update({ always_admin: v })} />
      </div>
      <div class="item">
        <span class="grow">
          {t("ui.settings.tray")}
          <span class="faint block">{t("ui.settings.tray_hint")}</span>
        </span>
        <Switch checked={app.settings.tray} label={t("ui.settings.tray")} onchange={(v) => update({ tray: v })} />
      </div>
      <div class="item">
        <span class="grow">
          {t("ui.settings.autostart")}
          <span class="faint block">{t("ui.settings.autostart_hint")}</span>
        </span>
        <Switch checked={app.settings.autostart} label={t("ui.settings.autostart")} onchange={(v) => update({ autostart: v })} />
      </div>
    </div>
  </section>

  <section>
    <h2>{t("ui.settings.background")}</h2>
    <div class="group">
      <div class="item">
        <span class="grow">
          {t("ui.settings.background_on")}
          <span class="faint block">{t("ui.settings.background_hint")}</span>
        </span>
        <Switch checked={app.settings.background.enabled} label={t("ui.settings.background_on")} onchange={(v) => setBackground({ enabled: v })} />
      </div>
      {#if app.settings.background.enabled}
        <div class="item">
          <span class="grow">{t("ui.settings.every")}</span>
          <select class="field" value={app.settings.background.every_hours} onchange={(e) => setBackground({ every_hours: Number((e.currentTarget as HTMLSelectElement).value) })}>
            {#each HOURS as h (h)}<option value={h}>{tn("ui.settings.hours", h)}</option>{/each}
          </select>
        </div>
        <div class="item">
          <span class="grow">
            {t("ui.settings.threshold")}
            <span class="faint block">{t("ui.settings.threshold_hint")}</span>
          </span>
          <select class="field" value={app.settings.background.threshold_mb} onchange={(e) => setBackground({ threshold_mb: Number((e.currentTarget as HTMLSelectElement).value) })}>
            {#each THRESHOLDS as mb (mb)}<option value={mb}>{formatBytes(mb * 1024 * 1024)}</option>{/each}
          </select>
        </div>
        {#if bgStatus}
          <div class="item faint small">
            <Icon name="activity" size={14} />
            {t("ui.settings.last_check", { date: formatDate(bgStatus.timestamp), size: formatBytes(bgStatus.bytes) })}
          </div>
        {/if}
      {/if}
    </div>
  </section>

  <section>
    <h2>{t("ui.settings.safety")}</h2>
    <div class="group">
      <div class="item">
        <span class="grow">
          {t("ui.settings.allow_system")}
          <span class="faint block">{t("ui.settings.allow_system_hint")}</span>
        </span>
        <Switch checked={app.settings.allow_system} label={t("ui.settings.allow_system")} onchange={toggleSystem} />
      </div>
      <div class="item">
        <span class="grow">
          {t("ui.settings.privacy")}
          <span class="faint block">{t("ui.settings.privacy_hint")}</span>
        </span>
        <Switch checked={app.settings.privacy} label={t("ui.settings.privacy")} onchange={(v) => update({ privacy: v })} />
      </div>
      <div class="item">
        <span class="grow">
          {t("ui.log.title")}
          <span class="faint block">{t("ui.log.hint")}</span>
        </span>
        <button class="btn small" onclick={() => (showLog = true)}><Icon name="list" size={14} /> {t("ui.log.open")}</button>
      </div>
      <div class="item">
        <span class="grow">
          {t("ui.log.keep")}
          <span class="faint block">{t("ui.log.keep_hint")}</span>
        </span>
        <select
          class="field"
          value={app.settings.deletion_log}
          onchange={(e) => setLogSize(Number((e.currentTarget as HTMLSelectElement).value))}
        >
          {#each LOG_SIZES as n (n)}
            <option value={n}>{n === 0 ? t("ui.log.keep_none") : t("ui.log.keep_n", { n: n.toLocaleString(i18n.lang) })}</option>
          {/each}
        </select>
        <button class="btn small quiet" onclick={clearLog}><Icon name="trash" size={14} /> {t("ui.log.clear")}</button>
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
      <div class="item excl">
        <Icon name="shield" size={16} />
        <span class="grow">{tn("ui.excl.count", app.settings.exclusions.length)}</span>
        <button class="btn small" onclick={() => (app.view = "exclusions")}>{t("ui.excl.manage")} <Icon name="chevron" size={12} /></button>
      </div>
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
        <button class="btn small" onclick={() => (app.view = "about")}><Icon name="info" size={14} /> {t("ui.about.open")}</button>
      </div>
      <div class="item">
        <span class="grow">
          {t("ui.update.auto")}
          <span class="faint block">
            {#if upd.status === "checking"}{t("ui.update.checking")}
            {:else if upd.status === "latest"}{t("ui.update.latest")}
            {:else if upd.info}{t("ui.update.title", { v: upd.info.version })}
            {:else if upd.status === "error"}{t("ui.update.error")}
            {:else}{t("ui.update.auto_hint")}{/if}
          </span>
        </span>
        {#if upd.info}
          <button class="btn small primary" onclick={() => (upd.open = true)}><Icon name="up" size={14} /> {t("ui.update.label")}</button>
        {:else}
          <button class="btn small" disabled={upd.status === "checking"} onclick={() => upd.check()}><Icon name="refresh" size={14} /> {t("ui.update.check")}</button>
        {/if}
        <Switch checked={app.settings.check_updates} label={t("ui.update.auto")} onchange={(v) => update({ check_updates: v })} />
      </div>
      <div class="item">
        <span class="grow">{t("ui.settings.wizard")}</span>
        <button class="btn small" onclick={() => update({ onboarded: false })}><Icon name="refresh" size={14} /> {t("ui.settings.wizard_again")}</button>
      </div>
    </div>
  </section>
</div>

{#if editor}
  <ModeEditor initial={editor.initial} editing={editor.editing} onsave={saveMode} oncancel={() => (editor = null)} />
{/if}

{#if showLog}
  <DeletionLog onclose={() => (showLog = false)} />
{/if}

{#if askLog}
  <Modal title={askLog.keep === 0 || askLog.keep === -1 ? t("ui.log.clear_title") : t("ui.log.trim_title")} onclose={() => (askLog = null)}>
    {#if askLog.drop === 0}
      <p>{t("ui.log.empty")}</p>
    {:else}
      <p>
        {askLog.keep > 0
          ? tn("ui.log.trim_text", askLog.drop)
          : tn("ui.log.clear_text", askLog.drop)}
      </p>
      <p class="faint small">{t("ui.log.no_undo")}</p>
    {/if}
    {#snippet buttons()}
      {#if askLog?.drop === 0}
        <button class="btn" onclick={() => (askLog = null)}>{t("ui.common.close")}</button>
      {:else}
        <button class="btn" onclick={() => (askLog = null)}>{t("ui.common.cancel")}</button>
        <button class="btn danger" onclick={applyLog}>{t("ui.log.confirm_delete")}</button>
      {/if}
    {/snippet}
  </Modal>
{/if}

{#if askSystem}
  <Modal title={t("ui.settings.allow_system")} onclose={() => (askSystem = false)}>
    <div class="danger-box">
      <Icon name="alert" size={20} />
      <p>{t("ui.settings.allow_system_warn")}</p>
    </div>
    <label class="agree">
      <input type="checkbox" bind:checked={agreed} />
      <span>{t("ui.system.agree")}</span>
    </label>
    {#snippet buttons()}
      <button class="btn" onclick={() => (askSystem = false)}>{t("ui.common.cancel")}</button>
      <button
        class="btn danger"
        disabled={!agreed}
        onclick={() => {
          askSystem = false;
          void update({ allow_system: true });
        }}>{t("ui.settings.allow_system_enable")}</button
      >
    {/snippet}
  </Modal>
{/if}

<style>
  .page { flex: 1; min-height: 0; overflow: auto; padding: 4px 32px 40px; display: flex; flex-direction: column; gap: 28px; max-width: 900px; width: 100%; margin: 0 auto; }
  .page > * { flex-shrink: 0; }
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
  .item.excl :global(svg) { color: var(--excl); }
  .item.small { min-height: 40px; font-size: 12px; }
  .danger-box {
    display: flex; gap: 12px; padding: 12px 14px; border-radius: 12px; color: var(--bad); font-size: 13px;
    background: color-mix(in srgb, var(--bad) 10%, transparent); border: 1px solid color-mix(in srgb, var(--bad) 30%, transparent);
  }
  .danger-box p { color: var(--text); }
  .agree { display: flex; gap: 10px; align-items: flex-start; margin-top: 14px; font-size: 13px; cursor: pointer; }
  .btn.danger { background: var(--bad); border: 0; color: #fff; font-weight: 600; }
  @media (max-width: 700px) {
    .item { flex-wrap: wrap; }
    .field { min-width: 0; width: 100%; }
  }
</style>
