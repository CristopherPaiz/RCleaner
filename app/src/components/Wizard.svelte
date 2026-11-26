<script lang="ts">
  // Asistente de la primera vez: idioma, tema y cómo debe comportarse RCleaner.
  import logo from "../assets/logo.png";
  import { app } from "../lib/app.svelte";
  import { LANGUAGES, resolveLang, t } from "../lib/i18n.svelte";
  import Icon from "./Icon.svelte";
  import Switch from "./Switch.svelte";
  import TitleBar from "./TitleBar.svelte";

  let step = $state(0);
  const STEPS = 4;
  let prefs = $state({
    tray: app.settings.tray,
    background: app.settings.background.enabled,
    autostart: app.settings.autostart,
    always_admin: app.settings.always_admin,
  });
  let saving = $state(false);

  const systemLang = LANGUAGES.find((l) => l.code === resolveLang(navigator.language))?.name ?? "";
  const THEMES = [
    { id: null, label: "ui.settings.theme_system", icon: "monitor" },
    { id: "light", label: "ui.settings.theme_light", icon: "sun" },
    { id: "dark", label: "ui.settings.theme_dark", icon: "moon" },
  ] as const;

  async function finish() {
    saving = true;
    try {
      await app.saveSettings({
        ...app.settings,
        tray: prefs.tray,
        autostart: prefs.autostart,
        always_admin: prefs.always_admin,
        background: { ...app.settings.background, enabled: prefs.background },
        onboarded: true,
      });
    } catch (e) {
      app.error = String(e);
      await app.saveSettings({ ...app.settings, onboarded: true }).catch(() => {});
    } finally {
      saving = false;
    }
  }
</script>

<div class="wizard" data-section="home">
  <TitleBar />
  <div class="body">
    {#key step}
      <div class="step">
        {#if step === 0}
          <img class="logo" src={logo} width="112" height="112" alt="" />
          <h1>{t("ui.wizard.welcome")}</h1>
          <p class="dim lead">{t("ui.wizard.welcome_text")}</p>
        {:else if step === 1}
          <span class="glyph"><Icon name="globe" size={28} /></span>
          <h1>{t("ui.wizard.language")}</h1>
          <div class="options">
            <button class="opt" class:on={app.settings.language === null} onclick={() => app.setLanguage(null)}>
              <Icon name="monitor" size={18} /> <span>{t("ui.settings.language_system", { name: systemLang })}</span>
            </button>
            {#each LANGUAGES as l (l.code)}
              <button class="opt" class:on={app.settings.language === l.code} onclick={() => app.setLanguage(l.code)}>
                <span class="code">{l.code.toUpperCase()}</span> <span>{l.name}</span>
              </button>
            {/each}
          </div>
        {:else if step === 2}
          <span class="glyph"><Icon name="sun" size={28} /></span>
          <h1>{t("ui.wizard.theme")}</h1>
          <div class="themes">
            {#each THEMES as th (th.label)}
              <button class="theme" class:on={app.settings.theme === th.id} onclick={() => app.setTheme(th.id)}>
                <span class="preview {th.id ?? 'system'}"><span></span><span></span><span></span></span>
                <span class="th-label"><Icon name={th.icon} size={14} /> {t(th.label)}</span>
              </button>
            {/each}
          </div>
        {:else}
          <span class="glyph"><Icon name="settings" size={28} /></span>
          <h1>{t("ui.wizard.prefs")}</h1>
          <div class="prefs">
            <div class="pref">
              <span><strong>{t("ui.settings.tray")}</strong><span class="faint">{t("ui.settings.tray_hint")}</span></span>
              <Switch checked={prefs.tray} label={t("ui.settings.tray")} onchange={(v) => (prefs.tray = v)} />
            </div>
            <div class="pref">
              <span><strong>{t("ui.settings.background_on")}</strong><span class="faint">{t("ui.settings.background_hint")}</span></span>
              <Switch checked={prefs.background} label={t("ui.settings.background_on")} onchange={(v) => (prefs.background = v)} />
            </div>
            <div class="pref">
              <span><strong>{t("ui.settings.autostart")}</strong><span class="faint">{t("ui.settings.autostart_hint")}</span></span>
              <Switch checked={prefs.autostart} label={t("ui.settings.autostart")} onchange={(v) => (prefs.autostart = v)} />
            </div>
            <div class="pref">
              <span><strong>{t("ui.settings.always_admin")}</strong><span class="faint">{t("ui.settings.always_admin_hint")}</span></span>
              <Switch checked={prefs.always_admin} label={t("ui.settings.always_admin")} onchange={(v) => (prefs.always_admin = v)} />
            </div>
          </div>
        {/if}
      </div>
    {/key}
  </div>

  <footer>
    <button class="btn quiet" class:hide={step === 0} onclick={() => step--}><Icon name="back" size={15} /> {t("ui.wizard.back")}</button>
    <div class="dots" aria-hidden="true">
      {#each { length: STEPS } as _, i (i)}<span class:on={i === step}></span>{/each}
    </div>
    {#if step < STEPS - 1}
      <button class="btn accent" onclick={() => step++}>{t(step === 0 ? "ui.wizard.start" : "ui.wizard.next")} <Icon name="forward" size={15} /></button>
    {:else}
      <button class="btn accent" disabled={saving} onclick={finish}><Icon name="check" size={15} /> {t("ui.wizard.finish")}</button>
    {/if}
  </footer>
</div>

<style>
  .wizard {
    position: fixed; inset: 0; z-index: 30; display: flex; flex-direction: column; background: var(--bg);
    background-image: radial-gradient(800px 420px at 50% -10%, color-mix(in srgb, var(--a) 22%, transparent), transparent 70%);
  }
  .body { flex: 1; min-height: 0; overflow: auto; display: grid; place-items: center; padding: 0 32px; }
  .step { display: flex; flex-direction: column; align-items: center; gap: 12px; text-align: center; width: min(640px, 100%); animation: rise 0.3s ease-out; padding: 12px 0; }
  .logo { filter: drop-shadow(0 16px 36px color-mix(in srgb, var(--a) 45%, transparent)); margin-bottom: 8px; }
  h1 { font-size: 30px; }
  .lead { font-size: 15px; max-width: 52ch; }
  .glyph {
    width: 60px; height: 60px; border-radius: 18px; display: grid; place-items: center; color: #fff; margin-bottom: 6px;
    background: radial-gradient(100% 80% at 30% 10%, rgb(255 255 255 / 0.3), transparent 60%), linear-gradient(145deg, var(--a), var(--b));
  }
  .options { display: flex; flex-direction: column; gap: 6px; width: min(360px, 100%); margin-top: 10px; }
  .opt, .theme, .pref {
    border: 1px solid var(--line-2); background: var(--raise); border-radius: 12px; color: var(--text); cursor: pointer;
  }
  .opt { display: flex; align-items: center; gap: 12px; padding: 12px 14px; text-align: left; }
  .opt:hover, .theme:hover { background: var(--raise-2); }
  .opt.on, .theme.on { border-color: color-mix(in srgb, var(--a) 75%, transparent); background: color-mix(in srgb, var(--a) 12%, var(--raise)); }
  .code { font-family: var(--mono); font-size: 11px; color: var(--text-3); width: 22px; }
  .themes { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; width: 100%; margin-top: 10px; }
  .theme { display: flex; flex-direction: column; gap: 10px; padding: 10px; }
  .preview { display: grid; grid-template-columns: 22% 1fr; grid-template-rows: 1fr 1fr; gap: 5px; height: 90px; padding: 6px; border-radius: 8px; }
  .preview span { border-radius: 4px; }
  .preview span:first-child { grid-row: span 2; }
  .preview.dark { background: #0b0c10; }
  .preview.dark span { background: #1d2029; }
  .preview.light { background: #f5f6f8; }
  .preview.light span { background: #dfe2e8; }
  .preview.system { background: linear-gradient(115deg, #f5f6f8 50%, #0b0c10 50%); }
  .preview.system span { background: rgb(128 128 128 / 0.35); }
  .th-label { display: flex; align-items: center; justify-content: center; gap: 6px; font-size: 13px; }
  .prefs { display: flex; flex-direction: column; gap: 6px; width: 100%; margin-top: 10px; text-align: left; }
  .pref { display: flex; align-items: center; gap: 14px; padding: 12px 14px; cursor: default; }
  .pref > span { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .pref > span span { font-size: 12px; }
  footer { display: grid; grid-template-columns: 1fr auto 1fr; align-items: center; padding: 16px 32px 24px; gap: 12px; }
  footer .btn:last-child { justify-self: end; }
  .hide { visibility: hidden; }
  .dots { display: flex; gap: 6px; }
  .dots span { width: 7px; height: 7px; border-radius: 50%; background: var(--line-2); transition: width 0.2s, background 0.2s; }
  .dots span.on { width: 22px; border-radius: 99px; background: linear-gradient(90deg, var(--a), var(--b)); }
  @media (max-width: 640px) { .themes { grid-template-columns: 1fr; } }
</style>
