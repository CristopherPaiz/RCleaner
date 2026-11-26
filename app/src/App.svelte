<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./lib/api";
  import { app, type View } from "./lib/app.svelte";
  import Icon from "./components/Icon.svelte";
  import Home from "./views/Home.svelte";
  import Clean from "./views/Clean.svelte";
  import SettingsView from "./views/Settings.svelte";

  let elevating = $state(false);

  onMount(() => {
    const unlisten = api.onProgress((e) => app.onProgress(e));
    void app.load();
    return () => void unlisten.then((u) => u());
  });

  const NAV: { view: View; label: string; icon: string }[] = [
    { view: "home", label: "Inicio", icon: "home" },
    { view: "clean", label: "Limpiar", icon: "sparkles" },
    { view: "settings", label: "Ajustes", icon: "settings" },
  ];

  async function elevate() {
    elevating = true;
    try {
      await api.restartAsAdmin();
    } catch (e) {
      app.error = String(e);
      elevating = false;
    }
  }
</script>

<div class="shell">
  <aside>
    <div class="brand">
      <span class="logo" aria-hidden="true"></span>
      <div>
        <strong>Clean-CRP</strong>
        <small>v{app.info?.version ?? "…"}</small>
      </div>
    </div>

    <nav>
      {#each NAV as item (item.view)}
        <button class="nav" class:active={app.view === item.view} onclick={() => (app.view = item.view)}>
          <Icon name={item.icon} />
          {item.label}
        </button>
        {#if item.view === "clean"}
          <div class="modes">
            {#each app.modes as m (m.id)}
              <button
                class="mode"
                class:active={app.view === "clean" && app.mode?.id === m.id}
                disabled={app.busy}
                title={m.description}
                onclick={() => app.openMode(m)}
              >
                <Icon name={m.icon || "circle"} size={15} />
                <span>{m.name}</span>
                {#if m.custom}<span class="dot" title="Personalizado"></span>{/if}
              </button>
            {/each}
          </div>
        {/if}
      {/each}
    </nav>

    <div class="admin" class:on={app.info?.is_admin}>
      {#if app.info?.is_admin}
        <Icon name="shield" size={15} /> Administrador
      {:else}
        <span>Sin permisos de administrador</span>
        <button class="link" disabled={elevating || app.busy} onclick={elevate}>
          {elevating ? "Reiniciando…" : "Reiniciar como administrador"}
        </button>
      {/if}
    </div>
  </aside>

  <main>
    {#if app.error}
      <div class="banner error">
        <span>{app.error}</span>
        <button class="link" onclick={() => (app.error = null)}>Cerrar</button>
      </div>
    {/if}
    {#if app.view === "home"}
      <Home />
    {:else if app.view === "clean"}
      <Clean />
    {:else}
      <SettingsView />
    {/if}
  </main>
</div>

<style>
  .shell { display: grid; grid-template-columns: 240px 1fr; height: 100vh; }
  aside { background: var(--panel); border-right: 1px solid var(--border); padding: 18px 12px; display: flex; flex-direction: column; gap: 16px; overflow-y: auto; }
  .brand { display: flex; gap: 10px; align-items: center; padding: 0 8px; }
  .brand small { display: block; color: var(--muted); font-size: 12px; }
  .logo { width: 30px; height: 30px; border-radius: 9px; background: linear-gradient(135deg, var(--accent), var(--accent-2)); }
  nav { display: flex; flex-direction: column; gap: 2px; }
  button.nav, button.mode { display: flex; gap: 10px; align-items: center; text-align: left; background: none; border: 0; border-radius: 8px; color: var(--text); font: inherit; cursor: pointer; }
  button.nav { padding: 9px 10px; font-weight: 600; }
  button.mode { padding: 7px 10px 7px 34px; font-size: 13px; color: var(--muted); }
  button.mode span:first-of-type { flex: 1; }
  button.nav:hover, button.mode:hover:not(:disabled) { background: var(--hover); }
  button.nav.active { background: var(--hover); }
  button.mode.active { background: var(--accent-bg); color: var(--accent); font-weight: 600; }
  button.mode:disabled { opacity: 0.5; cursor: default; }
  .modes { display: flex; flex-direction: column; gap: 1px; margin: 2px 0 6px; }
  .dot { width: 6px; height: 6px; border-radius: 50%; background: var(--accent-2); }
  .admin { margin-top: auto; font-size: 12px; color: var(--muted); padding: 10px; border-radius: 8px; background: var(--bg); display: flex; flex-direction: column; gap: 4px; }
  .admin.on { color: var(--safe); flex-direction: row; align-items: center; gap: 6px; }
  main { display: flex; flex-direction: column; min-width: 0; overflow: hidden; }
  .banner.error { display: flex; justify-content: space-between; gap: 12px; margin: 14px 28px 0; padding: 10px 14px; border-radius: 8px; background: var(--danger-bg); color: var(--danger); }
</style>
