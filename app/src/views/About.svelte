<script lang="ts">
  // Acerca de: versión, enlaces del proyecto en GitHub y colaboradores.
  import logo from "../assets/logo.png";
  import { api, type Link } from "../lib/api";
  import { app } from "../lib/app.svelte";
  import { t, tn } from "../lib/i18n.svelte";
  import Icon from "../components/Icon.svelte";

  /** Colaboradores del proyecto (el resto, en GitHub). */
  const PEOPLE = [{ login: "CristopherPaiz", name: "Cristopher Paiz", role: "ui.about.role_creator" }];

  const LINKS: { id: Link; icon: string; label: string; text: string }[] = [
    { id: "repo", icon: "github", label: "ui.about.repo", text: "ui.about.repo_text" },
    { id: "releases", icon: "rocket", label: "ui.about.releases", text: "ui.about.releases_text" },
    { id: "new_issue", icon: "alert", label: "ui.about.issue", text: "ui.about.issue_text" },
    { id: "changelog", icon: "list", label: "ui.about.changelog", text: "ui.about.changelog_text" },
  ];

  const failed = $state<Record<string, boolean>>({});
  const initials = (name: string) =>
    name
      .split(/\s+/)
      .map((w) => w[0])
      .slice(0, 2)
      .join("")
      .toUpperCase();
</script>

<div class="page">
  <section class="hero">
    <img class="logo" src={logo} width="88" height="88" alt="" />
    <div>
      <h1>RCleaner</h1>
      <p class="dim">{t("ui.about.tagline")}</p>
      <p class="faint small num">
        {t("ui.about.version", { v: app.info?.version ?? "" })} · {tn("ui.settings.module_count", app.modules.length)} ·
        <button class="link" onclick={() => api.openLink("license")}>{t("ui.about.license")}</button>
      </p>
    </div>
  </section>

  <section>
    <h2>{t("ui.about.project")}</h2>
    <div class="links">
      {#each LINKS as l (l.id)}
        <button class="card" onclick={() => api.openLink(l.id)}>
          <span class="c-ic"><Icon name={l.icon} size={18} /></span>
          <span class="c-text">
            <strong>{t(l.label)}</strong>
            <span class="faint">{t(l.text)}</span>
          </span>
          <Icon name="external" size={14} />
        </button>
      {/each}
    </div>
  </section>

  <section>
    <div class="sec-head">
      <h2>{t("ui.about.contributors")}</h2>
      <button class="btn small quiet" onclick={() => api.openLink("contributors")}><Icon name="users" size={14} /> {t("ui.about.all_contributors")}</button>
    </div>
    <div class="people">
      {#each PEOPLE as p (p.login)}
        <button class="person" onclick={() => api.openLink(`user:${p.login}`)} title="github.com/{p.login}">
          {#if failed[p.login]}
            <span class="avatar fallback">{initials(p.name)}</span>
          {:else}
            <img
              class="avatar"
              src="https://avatars.githubusercontent.com/{p.login}?s=128"
              alt=""
              width="56"
              height="56"
              onerror={() => (failed[p.login] = true)}
            />
          {/if}
          <span class="p-text">
            <strong>{p.name}</strong>
            <span class="faint mono">@{p.login}</span>
            <span class="role">{t(p.role)}</span>
          </span>
        </button>
      {/each}
      <button class="person join" onclick={() => api.openLink("repo")}>
        <span class="avatar fallback"><Icon name="heart" size={22} /></span>
        <span class="p-text">
          <strong>{t("ui.about.join")}</strong>
          <span class="faint">{t("ui.about.join_text")}</span>
        </span>
      </button>
    </div>
  </section>

  <section>
    <h2>{t("ui.about.credits")}</h2>
    <p class="faint credits">{t("ui.about.credits_text")}</p>
  </section>
</div>

<style>
  .page { flex: 1; overflow: auto; padding: 4px 32px 40px; display: flex; flex-direction: column; gap: 30px; max-width: 900px; width: 100%; margin: 0 auto; }
  h2 { font-size: 15px; margin-bottom: 10px; }
  .small { font-size: 12px; margin-top: 4px; }
  .hero { display: flex; align-items: center; gap: 22px; padding-top: 8px; }
  .hero h1 { font-size: 32px; }
  .logo { filter: drop-shadow(0 12px 28px color-mix(in srgb, var(--a) 45%, transparent)); flex-shrink: 0; }
  .links { display: grid; grid-template-columns: repeat(auto-fill, minmax(250px, 1fr)); gap: 8px; }
  .card, .person {
    display: flex; align-items: center; gap: 12px; padding: 14px; border-radius: 14px; text-align: left; cursor: pointer;
    border: 1px solid var(--line); background: var(--raise); color: var(--text-3); transition: background 0.12s, border-color 0.12s;
  }
  .card:hover, .person:hover { background: var(--raise-2); border-color: var(--line-2); }
  .c-ic {
    width: 38px; height: 38px; border-radius: 11px; display: grid; place-items: center; flex-shrink: 0; color: #fff;
    background: linear-gradient(145deg, var(--a), var(--b));
  }
  .c-text, .p-text { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .c-text strong, .p-text strong { color: var(--text); }
  .c-text span, .p-text span { font-size: 12px; }
  .sec-head { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-bottom: 10px; }
  .sec-head h2 { margin: 0; }
  .people { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 8px; }
  .avatar { width: 56px; height: 56px; border-radius: 50%; flex-shrink: 0; object-fit: cover; background: var(--raise-2); }
  .avatar.fallback { display: grid; place-items: center; font-weight: 700; color: #fff; background: linear-gradient(145deg, var(--a), var(--b)); }
  .role { color: var(--a); font-weight: 600; margin-top: 2px; }
  .join { border-style: dashed; }
  .credits { font-size: 13px; max-width: 70ch; }
  @media (max-width: 640px) { .hero { flex-direction: column; text-align: center; } }
</style>
