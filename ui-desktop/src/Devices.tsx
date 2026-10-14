import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { call, errorText, native } from "./bridge";
import { Icon } from "./components";
import type { DeviceStatus, SyncPlan } from "./types";
export default function Devices({
  onSync,
  busy,
}: {
  onSync: (root: string) => void;
  busy: boolean;
}) {
  const [panel, setPanel] = useState<"transfer" | "preview">("transfer");
  const [status, setStatus] = useState<DeviceStatus | null>(null);
  const [target, setTarget] = useState("http://127.0.0.1:18080");
  const [plan, setPlan] = useState<SyncPlan | null>(null);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState("");
  async function operate(action: string, address?: string) {
    setWorking(true);
    setError("");
    try {
      const result = (await call("device", {
        action,
        target: address,
      })) as DeviceStatus;
      if (action === "status") {
        setStatus(result);
        setTarget(result.target);
      } else {
        const next = (await call("device", {
          action: "status",
        })) as DeviceStatus;
        setStatus(next);
      }
    } catch (e) {
      setError(errorText(e));
    } finally {
      setWorking(false);
    }
  }
  useEffect(() => {
    void operate("status");
  }, []);
  useEffect(() => {
    let gone = false;
    let inFlight = false;
    const timer = setInterval(() => {
      if (working || inFlight) return;
      inFlight = true;
      void call("device", { action: "status" })
        .then((result) => {
          if (!gone) setStatus(result as DeviceStatus);
        })
        .catch(() => {})
        .finally(() => {
          inFlight = false;
        });
    }, 3000);
    return () => {
      gone = true;
      clearInterval(timer);
    };
  }, [working]);
  async function planSync() {
    setError("");
    try {
      if (!native) throw Error("Open the desktop app to choose a sync folder.");
      const folder = await open({
        directory: true,
        multiple: false,
        title: "Choose music to sync",
      });
      if (typeof folder !== "string") return;
      setWorking(true);
      setPlan(
        (await call("device", { action: "plan", root: folder })) as SyncPlan,
      );
    } catch (e) {
      setError(errorText(e));
    } finally {
      setWorking(false);
    }
  }
  return (
    <section className="devices">
      <header className="device-toolbar">
        <span>MusicOS player</span>
        <span className={`connection ${status?.connected ? "connected" : ""}`}>
          {status?.connected ? "Connected" : "Not connected"}
        </span>
      </header>
      <div className="device-layout">
        <aside className="device-list">
          <header className="device-identity">
            <Icon name="device" />
            <div><h2>MusicOS player</h2><p>{status?.connected ? "Ready for your music" : "Waiting for a connection"}</p></div>
          </header>
          <section className="device-settings-panel">
            <h3>Connection</h3>
            <label className="field-label" htmlFor="device-address">Device address</label>
            <input id="device-address" value={target} onChange={(e) => setTarget(e.target.value)} spellCheck={false} />
            <button className="primary" disabled={working} onClick={() => void operate("status", target)}>
              {working ? "Connecting…" : "Connect"}
            </button>
            {status?.error && <small>{status.error}</small>}
          </section>
          <section className="device-settings-panel">
            <h3>Device preview</h3>
            <p>Try MusicOS without a connected player.</p>
            <button disabled={working} onClick={() => void operate(status?.preview.running ? "preview_stop" : "preview_start")}>
              <Icon name="autostart" />
              {working ? "Working…" : status?.preview.running ? "Stop preview" : "Launch preview"}
            </button>
            {status?.preview.last_error && <small role="alert">{status.preview.last_error}</small>}
          </section>
          <div className="device-connection-summary">
            <Icon name={status?.connected ? "device" : "NetSearch"} />
            <strong>{status?.connected ? "Connected" : "Offline"}</strong>
          </div>
        </aside>
        <div className="device-panels">
          <div className="device-mode" aria-label="Device tools">
            <button aria-pressed={panel === "transfer"} onClick={() => setPanel("transfer")}><Icon name="Storage" />Music transfer</button>
            <button aria-pressed={panel === "preview"} onClick={() => setPanel("preview")}><Icon name="device" />Device preview</button>
          </div>
          {panel === "transfer" ? <section className="device-transfer">
            <header><h2>Sync your music</h2><button disabled={working || busy} onClick={() => void planSync()}>Review sync</button></header>
            {plan ? <div className="sync-plan">
              <strong>{plan.files} files · {(plan.bytes / 1024 / 1024).toFixed(1)} MB</strong>
              <span className="sync-folder">{plan.root}</span>
              <p>Review this folder, then transfer its music to your player.</p>
              <button disabled={!status?.connected || busy} onClick={() => onSync(plan.root)}>Sync to player</button>
              {!status?.connected && <small>Connect your player to start the transfer.</small>}
            </div> : <div className="device-empty"><Icon name="sync" /><p>Choose a music folder to review before transferring it to your player.</p><small>Your local library stays in place.</small></div>}
          </section> : <section className="device-transfer">
            <header><h2>Device preview</h2></header>
            <div className="device-empty"><Icon name="device" /><p>The embedded MusicOS interface runs in its own window.</p><button disabled={working} onClick={() => void operate(status?.preview.running ? "preview_stop" : "preview_start")}>{status?.preview.running ? "Stop preview" : "Launch preview"}</button></div>
          </section>}
          <section className="device-quality"><h2>Transfer format</h2><strong>{plan ? plan.profile.replaceAll("_", " ") : "Choose a folder to review"}</strong><p>{plan ? "Using your saved device quality profile." : "The sync review shows your saved quality profile and transfer size."}</p></section>
        </div>
      </div>
      {error && <p className="inline-error" role="alert">{error}</p>}
    </section>
  );
}
