import AppleEnrichment from './AppleEnrichment';
import discordIcon from '../../src-tauri/icons/generated/128x128@2x.png';
import {useViewActive} from './ViewScope';
import ThemedSwitch from './ThemedSwitch';
import { useEffect, useRef, useState } from "react";
import { call, errorText, native } from "./bridge";
import type { IntegrationRequest, IntegrationStatus } from "./types";
import { Icon } from "./components";

export default function BetaIntegrations({cover}:{cover?:string|null}) {
 const active=useViewActive();
  const [status, setStatus] = useState<IntegrationStatus | null>(null);
  const [id, setId] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [authorizing, setAuthorizing] = useState(false);
  const [key, setKey] = useState("");
  const [secret, setSecret] = useState("");
  const mounted = useRef(true),lastRead=useRef(0);
  const requestInFlight = useRef(false),idLoaded=useRef(false);
  useEffect(() => {
    if(!active){setKey("");setSecret("");setAuthorizing(false);return;}
    mounted.current = true;
    if(!requestInFlight.current)setBusy(false);
    let alive=true;
    let timer: ReturnType<typeof setTimeout>;
    async function refresh() {
      if(!alive)return;
      if(requestInFlight.current){timer=setTimeout(refresh,3000);return;}
      try {
        const next = await call("integration_status");
        if (!alive) return;
        if (requestInFlight.current) { timer = setTimeout(refresh, 3000); return; }
        lastRead.current=Date.now();setStatus(next);
        if (!idLoaded.current) { setId(next.config.discord_client_id); idLoaded.current=true; }
      } catch (e) { if (mounted.current) setError(errorText(e)); }
      if (alive) timer = setTimeout(refresh, 3000);
    }
    if(status&&Date.now()-lastRead.current<3000)timer=setTimeout(refresh,3000-(Date.now()-lastRead.current));else void refresh();
    return () => { alive=false;mounted.current = false; clearTimeout(timer); };
  }, [active]);
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
  return <section className="beta-integrations" aria-label="Connected services">
    <div className="integration-list" aria-busy={busy}>
      <article className="integration-card">
        <div className="integration-heading"><Icon name="Discord"/><div><h3>Discord presence</h3><p>Share what’s playing through Vesktop or Discord.</p></div>
          <ThemedSwitch label="Discord presence" checked={status?.config.discord_enabled??false} disabled={!status||busy} onChange={checked=>configure({discord_enabled:checked})}/>
        </div>
        <p className="integration-status" role="status">{!status?"Checking connection…":!status.config.discord_client_id?"Setup needed":!status.config.discord_enabled?"Off":/^Connected|^Listening|^Ready|^Sharing/i.test(status.discord)?"Connected":"Needs attention"}</p>
        <details className="integration-setup"><summary><Icon name="Back"/>{status?.config.discord_client_id?"Manage Discord":"Set up Discord"}</summary><div className="integration-setup-body"><div className="discord-icon-setup"><img src={discordIcon} alt="musicOS application icon" width="64" height="64"/><p>For the fallback image, upload this icon in your Discord application’s Rich Presence → Art Assets and name it <code>musicos</code>. Local album artwork is never uploaded.</p><a href={discordIcon} download="musicOS.png">Download musicOS icon</a></div><ol><li>Create a Discord application named musicOS.</li><li>Save its Application ID below, then enable presence.</li><li>Keep Vesktop open with Rich Presence via arRPC enabled.</li></ol><p className="integration-privacy">Shares song, artist, album and listening time. Matching public artwork comes from Apple’s catalog; local covers stay on your device.</p>
          <button className="text-link" onClick={() => void link("https://discord.com/developers/applications")}>Open Discord developer portal ↗</button>
          <label className="field-label" htmlFor="discord-id">Application ID</label><div className="integration-field"><input id="discord-id" inputMode="numeric" autoComplete="off" value={id} placeholder="Your musicOS application ID" onChange={e => {idLoaded.current=true;setId(e.target.value);}} maxLength={20}/><button disabled={!status || busy || id===status.config.discord_client_id} onClick={() => configure({})}>Save</button></div>
        </div></details>
      </article>
      <article className="integration-card">
        <div className="integration-heading"><Icon name="LastFM"/><div><h3>Last.fm</h3><p>Save your listening history and discover your habits.</p></div>
          <ThemedSwitch label="Last.fm scrobbling" checked={status?.config.lastfm_enabled??false} disabled={!status?.config.lastfm_username||busy} onChange={checked=>configure({lastfm_enabled:checked})}/>
        </div>
        <p className="integration-status" role="status">{status?.config.lastfm_username && <strong>{status.config.lastfm_username} · </strong>}{!status?"Checking connection…":!status.config.lastfm_username?"Not connected":status.config.lastfm_enabled?"On":"Connected · Off"}</p>
        {status?.config.lastfm_username ? <button disabled={busy} onClick={() => void action({action:"disconnect_lastfm"})}>Disconnect & clear queued listens</button> : <details className="integration-setup" open={authorizing || undefined}><summary><Icon name="Back"/>Connect Last.fm</summary><div className="integration-setup-body">
          <ol><li>Create a Last.fm API account.</li><li>Enter its credentials, then approve musicOS in your browser.</li></ol><p className="integration-privacy">Your session stays in the system keychain. Enabled listens are sent after half the song or four minutes; offline listens wait for a connection.</p>
          <button className="text-link" onClick={() => void link("https://www.last.fm/api/account/create")}>Create a Last.fm API account ↗</button>
          {authorizing ? <div className="authorization-step"><p>Approve musicOS in your browser, then finish connecting here.</p><button disabled={busy} onClick={() => void action({action:"finish_authorization"})}>I’ve authorized musicOS</button></div> : <form onSubmit={e => {e.preventDefault(); void action({action:"authorize",api_key:key,shared_secret:secret});}}>
            <label className="field-label" htmlFor="lastfm-key">API key</label><input id="lastfm-key" type="password" autoComplete="off" value={key} onChange={e => setKey(e.target.value)} maxLength={32}/>
            <label className="field-label" htmlFor="lastfm-secret">Shared secret</label><input id="lastfm-secret" type="password" autoComplete="new-password" value={secret} onChange={e => setSecret(e.target.value)} maxLength={32}/>
            <button disabled={!status || busy || key.trim().length!==32 || secret.trim().length!==32}>Authorize musicOS ↗</button>
          </form>}
        </div></details>}
      </article>
    </div>
    <AppleEnrichment/>
    {!native&&<p className="integration-preview-note">Preview only. Connections run in the desktop app.</p>}
    {status&&<details className="integration-diagnostics"><summary><Icon name="Back"/>Connection details</summary><p>Discord: {status.discord}</p><p>Artwork: {status.discord_artwork??"Waiting for playback"}</p><p>Activity delivery: {status.discord_rpc??"Not connected"}</p><p>Last.fm: {status.lastfm}</p><p>{status.queued} listens waiting to sync.</p></details>}
    {error && <div className="inline-error" role="alert"><p>Couldn’t update this connection. Your previous settings are retained.</p><details><summary>Details</summary>{error}</details></div>}
  </section>;
}
