<script lang="ts">
  // Arranque: qué se abre con Windows. Sólo se activa/desactiva (nada se
  // borra), igual que en el Administrador de tareas.
  import { onMount } from "svelte";
  import { app } from "../lib/app.svelte";
  import { api, type ServiceItem, type StartupApp, type StartupTask } from "../lib/api";
  import { t, tn } from "../lib/i18n.svelte";
  import { menu, type MenuEntry } from "../lib/menu.svelte";
  import { copyText } from "../lib/activity.svelte";
  import Icon from "../components/Icon.svelte";
  import FileIcon from "../components/FileIcon.svelte";
  import Switch from "../components/Switch.svelte";

  type Tab = "apps" | "tasks" | "services";
  let tab = $state<Tab>("apps");
  let query = $state("");
  let apps = $state<StartupApp[] | null>(null);
  let tasks = $state<StartupTask[] | null>(null);
  let services = $state<ServiceItem[] | null>(null);
  let loading = $state(false);
  let busy = $state<string | null>(null);

  const admin = $derived(app.info?.is_admin ?? false);
  const q = $derived(query.trim().toLowerCase());
  const has = (...s: (string | null | undefined)[]) => !q || s.some((x) => x?.toLowerCase().includes(q));

  // Activos primero y, dentro, por nombre. Se ordena al cargar: así una fila
  // no salta de sitio al activarla o desactivarla.
  const sortApps = (list: StartupApp[]) =>
    list.sort((a, b) => Number(b.enabled) - Number(a.enabled) || a.name.localeCompare(b.name));
  const shownApps = $derived((apps ?? []).filter((a) => has(a.name, a.command)));
  const shownTasks = $derived((tasks ?? []).filter((x) => has(x.name, x.folder, x.command, x.author)));
  const shownServices = $derived((services ?? []).filter((s) => has(s.display, s.name, s.description, s.path)));
  const appsOn = $derived((apps ?? []).filter((a) => a.enabled).length);

  async function load(which: Tab = tab, force = false) {
    if (!force && ((which === "apps" && apps) || (which === "tasks" && tasks) || (which === "services" && services))) return;
    loading = true;
    try {
      if (which === "apps") apps = sortApps(await api.startupApps());
      else if (which === "tasks") tasks = await api.startupTasks();
      else services = await api.startupServices();
    } catch (e) {
      app.error = String(e);
    } finally {
      loading = false;
    }
  }

  function show(next: Tab) {
    tab = next;
    void load(next);
  }

  async function act(key: string, run: () => Promise<void>, after: () => void) {
    busy = key;
    try {
      await run();
      after();
    } catch (e) {
      app.error = deniedMessage(String(e));
      return false;
    } finally {
      busy = null;
    }
    return true;
  }

  /** Windows responde "Acceso denegado" cuando hace falta administrador. */
  function deniedMessage(msg: string) {
    return /denied|denegado|0x80070005/i.test(msg) ? t("ui.startup.needs_admin") : msg;
  }

  const toggleApp = (a: StartupApp, on: boolean) =>
    act(a.id, () => api.setStartupApp(a.id, on), () => (a.enabled = on));
  const toggleTask = (x: StartupTask, on: boolean) =>
    act(x.id, () => api.setStartupTask(x.id, on), () => (x.enabled = on));
  async function setStart(s: ServiceItem, select: HTMLSelectElement) {
    const ok = await act(s.name, () => api.setServiceStart(s.name, select.value), () => (s.start = select.value));
    if (!ok) select.value = s.start;
  }

  function appMenu(e: MouseEvent, a: StartupApp) {
    const items: (MenuEntry | false)[] = [
      !!a.file && !a.missing && { label: t("ui.common.show_in_explorer"), icon: "external", action: () => api.reveal(a.file!) },
      { label: t("ui.startup.copy_command"), icon: "copy", action: () => copyText(a.command) },
    ];
    menu.show(e, items.filter((x): x is MenuEntry => !!x));
  }

  /** Cambiar algo de todos los usuarios necesita administrador. */
  const locked = (a: StartupApp) => a.scope === "machine" && !admin;

  onMount(() => void load("apps"));
</script>

<div class="page">
  <header class="head">
    <div>
      <h1>{t("ui.startup.title")}</h1>
      <p class="dim">{t("ui.startup.lead")}</p>
    </div>
    <button class="btn small quiet" disabled={loading} onclick={() => load(tab, true)}>
      <Icon name="refresh" size={14} /> {t("ui.startup.reload")}
    </button>
  </header>

  <div class="bar">
    <div class="tabs" role="tablist">
      <button role="tab" aria-selected={tab === "apps"} class:on={tab === "apps"} onclick={() => show("apps")}>
        {t("ui.startup.apps")}{#if apps}<span class="count">{appsOn}</span>{/if}
      </button>
      <button role="tab" aria-selected={tab === "tasks"} class:on={tab === "tasks"} onclick={() => show("tasks")}>
        {t("ui.startup.tasks")}{#if tasks}<span class="count">{tasks.length}</span>{/if}
      </button>
      <button role="tab" aria-selected={tab === "services"} class:on={tab === "services"} onclick={() => show("services")}>
        {t("ui.startup.services")}{#if services}<span class="count">{services.length}</span>{/if}
      </button>
    </div>
    <label class="search">
      <Icon name="search" size={14} />
      <input type="search" placeholder={t("ui.startup.filter")} bind:value={query} />
    </label>
  </div>

  <p class="faint small hint">{t(`ui.startup.${tab}_hint`)}</p>

  <div class="group">
    {#if loading && ((tab === "apps" && !apps) || (tab === "tasks" && !tasks) || (tab === "services" && !services))}
      <div class="empty"><span class="spinner"></span><p class="faint">{t("ui.startup.loading")}</p></div>
    {:else if tab === "apps"}
      {#each shownApps as a (a.id)}
        <div class="row" class:off={!a.enabled} role="listitem" oncontextmenu={(e) => appMenu(e, a)}>
          <span class="ic"><FileIcon key="startup:{a.id}" fallback="rocket" size={24} /></span>
          <span class="main">
            <span class="name ellipsis">{a.name}</span>
            <span class="faint small ellipsis" title={a.command}>{a.command}</span>
          </span>
          <span class="chips">
            {#if a.missing}<span class="chip warn">{t("ui.startup.missing")}</span>{/if}
            <span class="chip">{t(`ui.startup.scope_${a.scope}`)}</span>
          </span>
          <span class="ctl" title={locked(a) ? t("ui.startup.needs_admin") : undefined}>
            {#if locked(a)}<Icon name="lock" size={13} />{/if}
            <Switch checked={a.enabled} label={a.name} disabled={locked(a) || busy === a.id} onchange={(v) => toggleApp(a, v)} />
          </span>
        </div>
      {:else}
        <div class="empty"><Icon name="rocket" size={26} /><p class="faint">{t("ui.startup.none")}</p></div>
      {/each}
    {:else if tab === "tasks"}
      {#each shownTasks as x (x.id)}
        <div class="row" class:off={!x.enabled} role="listitem">
          <span class="ic"><Icon name="clock" size={18} /></span>
          <span class="main">
            <span class="name ellipsis">{x.name}</span>
            <span class="faint small ellipsis" title={x.command}>{x.folder ? `${x.folder} · ` : ""}{x.command || x.author || ""}</span>
          </span>
          <span class="chips">
            {#if x.running}<span class="chip ok">{t("ui.startup.running")}</span>{/if}
            {#each x.triggers as k (k)}<span class="chip">{t(`ui.startup.trigger_${k}`)}</span>{/each}
          </span>
          <span class="ctl">
            <Switch checked={x.enabled} label={x.name} disabled={busy === x.id} onchange={(v) => toggleTask(x, v)} />
          </span>
        </div>
      {:else}
        <div class="empty"><Icon name="clock" size={26} /><p class="faint">{t("ui.startup.none")}</p></div>
      {/each}
    {:else}
      {#each shownServices as s (s.name)}
        <div class="row" class:off={s.start === "disabled"} role="listitem">
          <span class="ic"><Icon name="cpu" size={18} /></span>
          <span class="main">
            <span class="name ellipsis">{s.display}</span>
            <span class="faint small ellipsis" title={s.description ?? s.path}>{s.description ?? s.path}</span>
          </span>
          <span class="chips">
            <span class="chip" class:ok={s.running}>{s.running ? t("ui.startup.running") : t("ui.startup.stopped")}</span>
          </span>
          <span class="ctl" title={!admin ? t("ui.startup.needs_admin") : undefined}>
            {#if !admin}<Icon name="lock" size={13} />{/if}
            <select
              value={["auto", "manual", "disabled"].includes(s.start) ? s.start : ""}
              disabled={!admin || busy === s.name}
              aria-label={s.display}
              onchange={(e) => setStart(s, e.currentTarget)}
            >
              {#if !["auto", "manual", "disabled"].includes(s.start)}<option value="" disabled>{s.start}</option>{/if}
              <option value="auto">{t("ui.startup.start_auto")}</option>
              <option value="manual">{t("ui.startup.start_manual")}</option>
              <option value="disabled">{t("ui.startup.start_disabled")}</option>
            </select>
          </span>
        </div>
      {:else}
        <div class="empty"><Icon name="cpu" size={26} /><p class="faint">{t("ui.startup.none")}</p></div>
      {/each}
    {/if}
  </div>
  {#if tab === "apps" && apps}
    <p class="faint small">{tn("ui.startup.apps_on", appsOn)}</p>
  {/if}
</div>

<style>
  .page { flex: 1; min-height: 0; overflow: auto; padding: 4px 32px 32px; display: flex; flex-direction: column; gap: 14px; max-width: 980px; width: 100%; margin: 0 auto; }
  .page > * { flex-shrink: 0; }
  .head { display: flex; justify-content: space-between; align-items: flex-end; gap: 16px; flex-wrap: wrap; }
  .head h1 { font-size: 28px; margin-bottom: 4px; }
  .bar { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; }
  .tabs { display: flex; gap: 4px; padding: 3px; border: 1px solid var(--line); border-radius: 11px; background: var(--raise); }
  .tabs button { border: 0; background: none; padding: 6px 12px; border-radius: 8px; cursor: pointer; color: var(--text-2); font: inherit; font-size: 13px; display: flex; align-items: center; gap: 6px; }
  .tabs button.on { background: var(--hover); color: var(--text); font-weight: 600; }
  .count { font-size: 11px; padding: 0 6px; border-radius: 99px; background: color-mix(in srgb, var(--a) 18%, transparent); color: var(--a); }
  .search { display: flex; align-items: center; gap: 6px; padding: 0 10px; border: 1px solid var(--line); border-radius: 10px; background: var(--raise); color: var(--text-3); }
  .search input { border: 0; background: none; padding: 7px 0; outline: none; color: var(--text); font: inherit; font-size: 13px; width: 200px; }
  .hint { margin-top: -4px; }
  .group { display: flex; flex-direction: column; border: 1px solid var(--line); border-radius: var(--radius); overflow: hidden; background: var(--raise); }
  .row { display: grid; grid-template-columns: 28px minmax(0, 1fr) auto auto; align-items: center; gap: 12px; padding: 9px 14px; border-top: 1px solid var(--line); }
  .row:first-child { border-top: 0; }
  .row:hover { background: var(--hover); }
  .row.off .main, .row.off .ic { opacity: 0.55; }
  .ic { display: grid; place-items: center; color: var(--a); }
  .main { display: flex; flex-direction: column; min-width: 0; }
  .name { font-weight: 600; }
  .chips { display: flex; gap: 6px; flex-wrap: wrap; justify-content: flex-end; }
  .chip { font-size: 11px; padding: 2px 8px; border-radius: 99px; border: 1px solid var(--line-2); color: var(--text-2); white-space: nowrap; }
  .chip.ok { color: var(--ok); border-color: color-mix(in srgb, var(--ok) 40%, transparent); }
  .chip.warn { color: var(--warn); border-color: color-mix(in srgb, var(--warn) 40%, transparent); }
  .ctl { display: flex; align-items: center; gap: 6px; color: var(--text-3); }
  select { font: inherit; font-size: 13px; padding: 5px 8px; border-radius: 8px; border: 1px solid var(--line-2); background: var(--bg); color: var(--text); }
  .empty { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 36px 16px; color: var(--text-2); }
  .spinner { width: 24px; height: 24px; border-radius: 50%; border: 3px solid var(--line-2); border-top-color: var(--a); animation: spin 0.8s linear infinite; }
  .small { font-size: 12px; }
  @media (max-width: 760px) {
    .row { grid-template-columns: 24px minmax(0, 1fr) auto; }
    .chips { display: none; }
  }
</style>
