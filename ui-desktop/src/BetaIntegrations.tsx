import ThemedSwitch from './ThemedSwitch';
import { useEffect, useRef, useState } from "react";
import { call, errorText, native } from "./bridge";
import type { IntegrationRequest, IntegrationStatus } from "./types";
import { Icon } from "./components";

export default function BetaIntegrations() {
  const [status, setStatus] = useState<IntegrationStatus | null>(null);
  const [id, setId] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [authorizing, setAuthorizing] = useState(false);
  const [key, setKey] = useState("");
  const [secret, setSecret] = useState("");
  const mounted = useRef(true);
  const requestInFlight = useRef(false);
  useEffect(() => {
    mounted.current = true;
    let first = true;
    let timer: ReturnType<typeof setTimeout>;
    async function refresh() {
      try {
        const next = await call("integration_status");
        if (!mounted.current) return;
        if (requestInFlight.current) { timer = setTimeout(refresh, 3000); return; }
        setStatus(next);
        if (first) { setId(next.config.discord_client_id); first = false; }
      } catch (e) { if (mounted.current) setError(errorText(e)); }
      if (mounted.current) timer = setTimeout(refresh, 3000);
    }
    void refresh();
    return () => { mounted.current = false; clearTimeout(timer); };
  }, []);
  async function action(request: IntegrationRequest) {
    const previous = status;
    requestInFlight.current = true;
    setBusy(true); setError("");
    if (request.action === "configure" && previous) setStatus({...previous, config:{...previous.config, discord_enabled:request.discord_enabled, discord_client_id:request.discord_client_id, lastfm_enabled:request.lastfm_enabled}});
    try {
      const response = await call("integration_action", { request });
      if (!mounted.current) return;
      setStatus(response.status);
      if (response.auth_url) {
        setAuthorizing(true);
        setKey(""); setSecret("");
        await call("open_provider", { url: response.auth_url });
      }
      if (request.action === "finish_authorization" || request.action === "disconnect_lastfm") setAuthorizing(false);
    } catch (e) { if (mounted.current) { setError(errorText(e)); if (request.action === "configure") setStatus(previous); } }
    finally { requestInFlight.current = false; if (mounted.current) setBusy(false); }
  }
  function configure(change: Partial<{ discord_enabled: boolean; lastfm_enabled: boolean }>) {
    if (!status) return;
    void action({ action: "configure", discord_enabled: status.config.discord_enabled,
      lastfm_enabled: status.config.lastfm_enabled, discord_client_id: id, ...change });
  }
  async function link(url: string) {
    try { await call("open_provider", { url }); } catch (e) { setError(errorText(e)); }
  }
  return <section className="beta-integrations" aria-labelledby="labs-title">
    <header className="labs-heading"><div><span className="eyebrow">A little more connected</span><h2 id="labs-title">MusicOS Labs <span className="beta-badge">Beta</span></h2></div><Icon name="Sparklist" /></header>
    <p className="labs-intro">Optional connections for the way you listen. Everything starts off.</p>
    {!native && <p className="labs-note">Connections run in the MusicOS desktop app.</p>}
    <div className="integration-list" aria-busy={busy}>
      <article className="integration-card">
        <div className="integration-heading"><div><h3>Discord presence</h3><p>Let your friends see what’s playing in MusicOS.</p></div>
          <ThemedSwitch label="Discord presence" checked={status?.config.discord_enabled??false} disabled={!status||busy} onChange={checked=>configure({discord_enabled:checked})}/>
        </div>
        <p className="integration-status" role="status">{status?.discord ?? "Loading connection…"}</p>
        <details><summary>Set up Discord</summary><p>Create a Discord application named MusicOS, then paste its Application ID. Keep Discord desktop running. Presence shares the song, artist, album, listening time, and matching public album art. Artist and album names are used to find artwork in Apple’s catalog; local cover files stay on your device.</p>
          <button className="text-link" onClick={() => void link("https://discord.com/developers/applications")}>Open Discord developer portal ↗</button>
          <label className="field-label" htmlFor="discord-id">Application ID</label><div className="integration-field"><input id="discord-id" inputMode="numeric" autoComplete="off" value={id} placeholder="Your MusicOS application ID" onChange={e => setId(e.target.value)} maxLength={20}/><button disabled={!status || busy || id===status.config.discord_client_id} onClick={() => configure({})}>Save</button></div>
        </details>
      </article>
      <article className="integration-card">
        <div className="integration-heading"><div><h3>Last.fm scrobbling</h3><p>Keep a listening history, wherever the music takes you.</p></div>
          <ThemedSwitch label="Last.fm scrobbling" checked={status?.config.lastfm_enabled??false} disabled={!status?.config.lastfm_username||busy} onChange={checked=>configure({lastfm_enabled:checked})}/>
        </div>
        <p className="integration-status" role="status">{status?.config.lastfm_username && <strong>{status.config.lastfm_username} · </strong>}{status?.lastfm ?? "Loading connection…"}{!!status?.queued && ` · ${status.queued} queued`}</p>
        {status?.config.lastfm_username ? <button disabled={busy} onClick={() => void action({action:"disconnect_lastfm"})}>Disconnect & clear queued listens</button> : <details open={authorizing || undefined}><summary>Connect Last.fm</summary>
          <p>This beta uses your own Last.fm application credentials. MusicOS stores the session in your system keychain. When enabled, it sends track metadata to Last.fm after you listen to half the song or four minutes. Offline listens wait for a connection.</p>
          <button className="text-link" onClick={() => void link("https://www.last.fm/api/account/create")}>Create a Last.fm API account ↗</button>
          {authorizing ? <div className="authorization-step"><p>Approve MusicOS in your browser, then finish connecting here.</p><button disabled={busy} onClick={() => void action({action:"finish_authorization"})}>I’ve authorized MusicOS</button></div> : <form onSubmit={e => {e.preventDefault(); void action({action:"authorize",api_key:key,shared_secret:secret});}}>
            <label className="field-label" htmlFor="lastfm-key">API key</label><input id="lastfm-key" type="password" autoComplete="off" value={key} onChange={e => setKey(e.target.value)} maxLength={32}/>
            <label className="field-label" htmlFor="lastfm-secret">Shared secret</label><input id="lastfm-secret" type="password" autoComplete="new-password" value={secret} onChange={e => setSecret(e.target.value)} maxLength={32}/>
            <button disabled={!status || busy || key.trim().length!==32 || secret.trim().length!==32}>Authorize MusicOS ↗</button>
          </form>}
        </details>}
      </article>
    </div>
    {error && <p className="inline-error" role="alert">{error}</p>}
  </section>;
}
