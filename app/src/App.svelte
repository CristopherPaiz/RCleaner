<script lang="ts">
  import logo from "./assets/logo.png";
  import { onMount } from "svelte";
  import { api } from "./lib/api";
  import { app, type View } from "./lib/app.svelte";
  import { space } from "./lib/space.svelte";
  import { dupes } from "./lib/dupes.svelte";
  import Icon from "./components/Icon.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import { modeColors } from "./lib/format";
  import { t } from "./lib/i18n.svelte";
  import Home from "./views/Home.svelte";
  import Clean from "./views/Clean.svelte";
  import Space from "./views/Space.svelte";
  import Dupes from "./views/Dupes.svelte";
  import SettingsView from "./views/Settings.svelte";

  let elevating = $state(false);

  onMount(() => {
    const subs = [
      api.onProgress((e) => app.onProgress(e)),
      api.onDiskProgress((p) => (space.progress = p)),
      api.onDupesProgress((p) => (dupes.progress = p)),
    ];
    void app.load();
    return () => subs.forEach((s) => void s.then((u) => u()));
  });

  const NAV: { view: View; label: string; icon: string }[] = [
    { view: "home", label: "ui.nav.home", icon: "home" },
    { view: "clean", label: "ui.nav.clean", icon: "sparkles" },
    { view: "space", label: "ui.nav.space", icon: "pie" },
    { view: "dupes", label: "ui.nav.dupes", icon: "copy" },
  ];

  /** Indicador de trabajo en curso en el riel. */
  function working(v: View) {
    if (v === "clean") return app.busy;
    if (v === "space") return space.phase === "scanning";
    if (v === "dupes") return dupes.phase === "scanning";
    return false;
  }

  // En Limpieza, toda la interfaz toma el color del modo elegido.
  const tint = $derived(app.view === "clean" && app.mode ? modeColors(app.mode.icon) : null);

  async function elevate() {
    if (!confirm(t("ui.admin.confirm"))) return;
    elevating = true;
    try {
      await api.restartAsAdmin();
    } catch (e) {
      app.error = String(e);
      elevating = false;
    }
  }
</script>

<div class="shell" data-section={app.view} style:--a={tint?.[0]} style:--b={tint?.[1]}>
  <nav class="rail" aria-label={t("ui.nav.aria")} data-tauri-drag-region>
    <div class="logo" title="RCleaner v{app.info?.version ?? ''}" aria-hidden="true">
      <img src={logo} width="34" height="34" alt="" />
    </div>

    {#each NAV as item (item.view)}
      <button
        class="item"
        class:on={app.view === item.view}
        data-section={item.view}
        style:--a={item.view === "clean" && tint ? tint[0] : undefined}
        style:--b={item.view === "clean" && tint ? tint[1] : undefined}
        onclick={() => (app.view = item.view)}
        aria-current={app.view === item.view ? "page" : undefined}
      >
        <span class="ic">
          <Icon name={item.icon} size={20} />
          {#if working(item.view)}<span class="pulse" aria-label={t("ui.common.working")}></span>{/if}
        </span>
        <span class="lbl">{t(item.label)}</span>
      </button>
    {/each}

    <span class="grow"></span>

    {#if app.info && !app.info.is_admin}
      <button class="admin" title={t("ui.admin.restart")} disabled={elevating || app.busy} onclick={elevate}>
        <Icon name="shield" size={16} />
        <span>{elevating ? "…" : t("ui.admin.label")}</span>
      </button>
    {:else if app.info?.is_admin}
      <span class="admin on" title={t("ui.admin.running")}><Icon name="shield" size={16} /></span>
    {/if}

    <button
      class="item"
      class:on={app.view === "settings"}
      data-section="settings"
      onclick={() => (app.view = "settings")}
    >
      <span class="ic"><Icon name="settings" size={20} /></span>
      <span class="lbl">{t("ui.nav.settings")}</span>
    </button>
  </nav>

  <main>
    <div class="glow" aria-hidden="true"></div>
    <TitleBar />
    {#if app.error}
      <div class="banner" role="alert">
        <Icon name="alert" size={16} />
        <span class="ellipsis">{app.error}</span>
        <button class="icon-btn" onclick={() => (app.error = null)} aria-label={t("ui.common.close")}><Icon name="x" size={14} /></button>
      </div>
    {/if}
    {#key app.view}
      <div class="page">
        {#if app.view === "home"}
          <Home />
        {:else if app.view === "clean"}
          <Clean />
        {:else if app.view === "space"}
          <Space />
        {:else if app.view === "dupes"}
          <Dupes />
        {:else}
          <SettingsView />
        {/if}
      </div>
    {/key}
  </main>
</div>

<style>
  .shell { display: grid; grid-template-columns: 84px 1fr; height: 100vh; }

  .rail {
    background: var(--bg-rail); border-right: 1px solid var(--line);
    display: flex; flex-direction: column; align-items: center; gap: 4px; padding: 16px 0 12px;
  }
  .logo { margin: 4px 0 18px; pointer-events: none; filter: drop-shadow(0 6px 14px rgb(37 99 235 / 0.35)); }
  .item {
    width: 72px; padding: 6px 0 7px; border: 0; background: none; cursor: pointer;
    display: flex; flex-direction: column; align-items: center; gap: 5px; color: var(--text-3);
    border-radius: 12px;
  }
  .ic {
    position: relative; width: 40px; height: 40px; border-radius: 12px; display: grid; place-items: center;
    transition: background 0.15s, color 0.15s, box-shadow 0.2s;
  }
  .lbl { font-size: 11px; font-weight: 500; letter-spacing: 0.01em; }
  .item:hover .ic { background: var(--hover); color: var(--text); }
  .item:hover { color: var(--text-2); }
  .item.on { color: var(--text); }
  .item.on .ic {
    color: #fff;
    background: radial-gradient(100% 80% at 30% 10%, rgb(255 255 255 / 0.28), transparent 60%), linear-gradient(145deg, var(--a), var(--b));
    box-shadow: inset 0 1px 0 rgb(255 255 255 / 0.35), 0 8px 20px -8px color-mix(in srgb, var(--a) 80%, transparent);
  }
  .pulse {
    position: absolute; top: 3px; right: 3px; width: 8px; height: 8px; border-radius: 50%;
    background: var(--b); box-shadow: 0 0 0 2px var(--bg-rail); animation: blink 1.2s ease-in-out infinite;
  }
  @keyframes blink { 50% { opacity: 0.3; } }
  .grow { flex: 1; }
  .admin {
    display: flex; flex-direction: column; align-items: center; gap: 3px; margin-bottom: 6px;
    border: 0; background: none; color: var(--warn); font-size: 10px; font-weight: 600; cursor: pointer;
    padding: 6px 8px; border-radius: 10px;
  }
  .admin:hover:not(:disabled) { background: var(--hover); }
  .admin.on { color: var(--ok); cursor: default; }

  main { position: relative; display: flex; flex-direction: column; min-width: 0; overflow: hidden; }
  .glow {
    position: absolute; inset: 0; pointer-events: none;
    background:
      radial-gradient(700px 380px at 50% -8%, color-mix(in srgb, var(--a) 20%, transparent), transparent 70%),
      radial-gradient(500px 300px at 85% 0%, color-mix(in srgb, var(--b) 10%, transparent), transparent 70%);
    transition: background 0.4s;
  }
  .page { position: relative; flex: 1; display: flex; flex-direction: column; min-height: 0; animation: fade 0.18s ease-out; }
  .banner {
    position: relative; z-index: 5; display: flex; align-items: center; gap: 10px; margin: 0 32px 6px; padding: 8px 8px 8px 14px;
    border-radius: 10px; background: color-mix(in srgb, var(--bad) 14%, var(--raise)); color: var(--bad);
    border: 1px solid color-mix(in srgb, var(--bad) 30%, transparent);
  }
  .banner span { flex: 1; }
</style>
