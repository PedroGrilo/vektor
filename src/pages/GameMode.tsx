import { useEffect, useMemo, useState } from "react";
import { Cpu, Loader2, MonitorUp, Palette, Play, Power, Save, Square, XCircle } from "lucide-react";
import type { DisplayMode, SessionConfig } from "../api";
import { backend } from "../mock";
import { PageHead, Toggle, useLoad, usePoll, useToast } from "../ui";

const modeKey = (m: DisplayMode) => `${m.width}x${m.height}@${m.refresh}`;

export default function GameMode() {
  const toast = useToast();
  const [live] = usePoll(() => backend.sessionStatus(), 2000);
  const topo = useLoad(() => backend.sessionTopology());
  const displays = useLoad(() => backend.displayList());
  const apps = useLoad(() => backend.listBackgroundApps());
  const vib = useLoad(() => backend.vibranceInfo());
  const [cfg, setCfg] = useState<SessionConfig | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (live && !cfg) setCfg(live.config);
  }, [live, cfg]);

  const primary = displays.data?.find((d) => d.primary) ?? displays.data?.[0];
  // Selectable apps = running known apps + ones already chosen (may not be running now).
  const appChoices = useMemo(() => {
    const names = new Map<string, string>();
    for (const a of apps.data ?? []) names.set(a.name, a.label);
    for (const n of cfg?.closeApps ?? []) if (!names.has(n)) names.set(n, n.replace(/\.exe$/i, ""));
    return [...names.entries()];
  }, [apps.data, cfg?.closeApps]);

  if (!cfg) return <div className="empty"><Loader2 className="spin" /></div>;
  const dirty = JSON.stringify(cfg) !== JSON.stringify(live?.config);

  const save = async () => {
    setSaving(true);
    try {
      setCfg((await backend.sessionSetConfig(cfg)).config);
      toast("ok", "Modo jogo guardado");
    } catch (e) {
      toast("err", String(e));
    } finally {
      setSaving(false);
    }
  };

  const flipApp = (name: string) =>
    setCfg({ ...cfg, closeApps: cfg.closeApps.includes(name) ? cfg.closeApps.filter((n) => n !== name) : [...cfg.closeApps, name] });

  return (
    <>
      <PageHead eyebrow="// modo jogo" title="Modo Jogo"
        desc="Quando o CS2 arranca, o Vektor aplica estas definições. Quando fecha, repõe tudo. O Vektor tem de estar aberto ou na bandeja."
        right={
          <div className="row">
            <button className="btn" onClick={() => backend.cs2Launch([]).catch((e) => toast("err", String(e)))}><Play size={15} /> Jogar</button>
            <button className="btn primary" disabled={!dirty || saving} onClick={save}>
              {saving ? <Loader2 size={15} className="spin" /> : <Save size={15} />} Guardar
            </button>
          </div>
        } />

      <div className={`panel ${live?.active ? "corners" : ""}`} style={{ marginBottom: 16 }}>
        <div className="row between">
          <div className="row">
            <Power size={20} color={live?.active ? "var(--ok)" : "var(--text-faint)"} />
            <div>
              <h3>{live?.active ? "Sessão ativa" : "À espera do CS2"}</h3>
              <div className="sub">
                {live?.active && live.since ? `Desde ${new Date(live.since).toLocaleTimeString("pt-PT")} · PID ${live.cs2Pid ?? "?"}` : "Abre o CS2 para ativar automaticamente."}
              </div>
            </div>
          </div>
          <div className="row">
            <span className="label">automático</span>
            <Toggle label="Modo jogo automático" on={cfg.enabled} onChange={(v) => setCfg({ ...cfg, enabled: v })} />
            {live?.active && (
              <button className="btn danger" onClick={() => backend.sessionStop().catch((e) => toast("err", String(e)))}><Square size={14} /> Terminar</button>
            )}
          </div>
        </div>
        {live?.active && live.actions.length > 0 && (
          <div className="meta" style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 12 }}>
            {live.actions.map((a, i) => <span key={i} className="chip on">{a}</span>)}
          </div>
        )}
      </div>

      <div className="grid-2">
        <div className="panel">
          <h3 className="row"><XCircle size={16} color="var(--accent)" /> Fechar apps durante o jogo</h3>
          <div className="sub" style={{ marginBottom: 10 }}>Só apps conhecidas. Steam e CS2 nunca são fechados.</div>
          {appChoices.length === 0 && <div className="sub">Nenhuma app conhecida a correr agora.</div>}
          <div className="stack" style={{ gap: 4 }}>
            {appChoices.map(([name, label]) => (
              <label key={name} className="row" style={{ cursor: "pointer", padding: "4px 2px" }}>
                <input type="checkbox" checked={cfg.closeApps.includes(name)} onChange={() => flipApp(name)} /> {label}
              </label>
            ))}
          </div>
          <div className="row between" style={{ marginTop: 12 }}>
            <span className="sub">Reabrir quando o jogo fechar</span>
            <Toggle label="Reabrir apps" on={cfg.reopenApps} onChange={(v) => setCfg({ ...cfg, reopenApps: v })} />
          </div>
        </div>

        <div className="panel">
          <h3 className="row"><Cpu size={16} color="var(--accent)" /> Afinidade de CPU</h3>
          <div className="sub" style={{ marginBottom: 12 }}>Fixa o CS2 em núcleos específicos. Útil em Ryzen com 2 CCDs ou 3D V-Cache.</div>
          <div className="tabs" style={{ marginBottom: 12 }}>
            {(["off", "auto", "custom"] as const).map((m) => (
              <button key={m} className={cfg.affinity === m ? "active" : ""} onClick={() => setCfg({ ...cfg, affinity: m })}>
                {m === "off" ? "Desligado" : m === "auto" ? "Automático" : "Manual"}
              </button>
            ))}
          </div>
          {topo.data && (
            <div className="sub">
              {topo.data.logicalCount} threads · {topo.data.groups.length} grupo(s) L3
              {topo.data.groups.map((g) => <span key={g.id} className="chip" style={{ marginLeft: 6 }}>L3 {g.l3Mb} MB · {g.logicalCount}t</span>)}
              {cfg.affinity === "auto" && <div style={{ marginTop: 8 }}>Automático: <b style={{ color: "var(--accent)" }}>{topo.data.recommendedLabel}</b> <span className="mono faint">{topo.data.recommendedMask}</span></div>}
            </div>
          )}
          {cfg.affinity === "custom" && (
            <input className="input" style={{ marginTop: 10 }} placeholder="0xFFFC" value={cfg.affinityMask ?? ""}
              onChange={(e) => setCfg({ ...cfg, affinityMask: e.target.value.trim() || null })} />
          )}
        </div>

        <div className="panel">
          <h3 className="row"><MonitorUp size={16} color="var(--accent)" /> Resolução durante o jogo</h3>
          <div className="sub" style={{ marginBottom: 12 }}>
            Ex.: 4:3 esticado (1280×960). Repõe a resolução nativa quando o jogo fecha. Para esticar, ativa o dimensionamento "Ecrã inteiro" no painel da gráfica.
          </div>
          <select className="input" value={cfg.resolution ? modeKey(cfg.resolution) : ""}
            onChange={(e) => {
              const m = primary?.modes.find((x) => modeKey(x) === e.target.value) ?? null;
              setCfg({ ...cfg, resolution: m });
            }}>
            <option value="">Não alterar{primary ? ` (${primary.current.width}×${primary.current.height}@${primary.current.refresh})` : ""}</option>
            {primary?.modes.map((m) => <option key={modeKey(m)} value={modeKey(m)}>{m.width}×{m.height} @ {m.refresh} Hz</option>)}
          </select>
        </div>

        <div className="panel">
          <div className="row between">
            <h3 className="row"><Palette size={16} color="var(--accent)" /> Digital vibrance</h3>
            <Toggle label="Vibrance" on={cfg.vibrance !== null} disabled={!vib.data?.supported}
              onChange={(v) => setCfg({ ...cfg, vibrance: v ? 70 : null })} />
          </div>
          <div className="sub" style={{ marginBottom: 12 }}>
            {vib.data?.supported
              ? "Cores mais vivas só enquanto o CS2 está em primeiro plano. 50% = normal."
              : vib.data?.vendor === "nvidia"
                ? "Disponível só em AMD: a NVIDIA não publica esta função na API pública. Em NVIDIA, usa \"Ajustar definições de cor\" no Painel de Controlo."
                : "Não suportado nesta gráfica."}
          </div>
          {cfg.vibrance !== null && (
            <>
              <div className="row between" style={{ marginBottom: 6 }}>
                <span className="sub">Nível</span>
                <span className="mono" style={{ color: "var(--accent)" }}>{cfg.vibrance}%</span>
              </div>
              <input type="range" min={50} max={100} value={cfg.vibrance} onChange={(e) => setCfg({ ...cfg, vibrance: Number(e.target.value) })} />
              <div className="row" style={{ justifyContent: "flex-end", gap: 8, marginTop: 10 }}>
                <button className="btn ghost" onClick={() => backend.vibranceReset().catch((e) => toast("err", String(e)))}>Repor</button>
                <button className="btn" onClick={() => backend.vibrancePreview(cfg.vibrance ?? 50).catch((e) => toast("err", String(e)))}>Pré-visualizar</button>
              </div>
            </>
          )}
        </div>
      </div>
    </>
  );
}
