import { useEffect, useState } from "react";
import { Loader2, MemoryStick, Power, Zap } from "lucide-react";
import type { MemoryConfig } from "../api";
import { backend } from "../mock";
import { Gauge, PageHead, Toggle, usePoll, useToast } from "../ui";

export default function Memory() {
  const toast = useToast();
  const live = usePoll(() => backend.memoryStatus(), 2000);
  const [cfg, setCfg] = useState<MemoryConfig | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    if (live && !cfg) setCfg(live.config);
  }, [live, cfg]);

  const act = async (key: string, fn: () => Promise<unknown>, ok: string) => {
    setBusy(key);
    try {
      await fn();
      toast("ok", ok);
    } catch (e) {
      toast("err", String(e));
    } finally {
      setBusy(null);
    }
  };

  const s = live;
  const dirty = !!cfg && !!s && JSON.stringify(cfg) !== JSON.stringify(s.config);

  return (
    <>
      <PageHead eyebrow="// memória" title="Limpeza de Standby"
        desc="Como o ISLC: um serviço do Windows vigia a lista de standby e purga-a quando a memória livre fica baixa. Ajuda contra stutters em PCs com pouca RAM livre."
        right={
          <button className="btn" disabled={busy !== null} onClick={() => act("purge", backend.memoryPurgeNow, "Lista de standby purgada")}>
            {busy === "purge" ? <Loader2 size={15} className="spin" /> : <Zap size={15} />} Purgar agora
          </button>
        } />

      <div className="hero">
        <div className="panel corners">
          <div className="row" style={{ justifyContent: "space-around" }}>
            <Gauge value={s ? s.standbyMb / 1024 : 0} max={s ? s.totalMb / 1024 : 1} unit=" GB" label="standby" color="var(--violet)" />
            <Gauge value={s ? s.freeMb / 1024 : 0} max={s ? s.totalMb / 1024 : 1} unit=" GB" label="livre" />
          </div>
          <dl className="kv" style={{ marginTop: 18 }}>
            <dt>total</dt><dd>{s ? `${(s.totalMb / 1024).toFixed(1)} GB` : "—"}</dd>
            <dt>modificada</dt><dd>{s ? `${s.modifiedMb.toFixed(0)} MB` : "—"}</dd>
            <dt>purgas</dt><dd>{s ? s.purgeCount : "—"}{s?.lastPurge ? ` · última ${new Date(s.lastPurge).toLocaleTimeString("pt-PT")}` : ""}</dd>
          </dl>
        </div>

        <div className="stack">
          <div className="panel">
            <div className="row between">
              <div className="row"><Power size={18} color={s?.serviceRunning ? "var(--lime)" : "var(--text-faint)"} />
                <div>
                  <h3>Serviço VektorMemory</h3>
                  <div className="sub">{s?.serviceRunning ? "A correr — arranca com o Windows." : s?.serviceInstalled ? "Instalado, parado." : "Não instalado."}</div>
                </div>
              </div>
              {s?.serviceInstalled ? (
                <button className="btn danger" disabled={busy !== null} onClick={() => act("svc", backend.memoryUninstallService, "Serviço removido")}>
                  {busy === "svc" && <Loader2 size={15} className="spin" />} Remover
                </button>
              ) : (
                <button className="btn primary" disabled={busy !== null} onClick={() => act("svc", backend.memoryInstallService, "Serviço instalado e iniciado")}>
                  {busy === "svc" && <Loader2 size={15} className="spin" />} Instalar
                </button>
              )}
            </div>
          </div>

          {cfg && (
            <div className="panel">
              <div className="row between" style={{ marginBottom: 14 }}>
                <h3 className="row"><MemoryStick size={16} color="var(--cyan)" /> Regras</h3>
                <Toggle label="Ativo" on={cfg.enabled} onChange={(v) => setCfg({ ...cfg, enabled: v })} />
              </div>
              <Slider label="Purgar quando a standby passar" unit="MB" min={256} max={8192} step={256}
                value={cfg.standbyThresholdMb} onChange={(v) => setCfg({ ...cfg, standbyThresholdMb: v })} />
              <Slider label="…e a memória livre for menor que" unit="MB" min={256} max={8192} step={256}
                value={cfg.freeThresholdMb} onChange={(v) => setCfg({ ...cfg, freeThresholdMb: v })} />
              <Slider label="Verificar a cada" unit="s" min={2} max={60} step={1}
                value={cfg.intervalSec} onChange={(v) => setCfg({ ...cfg, intervalSec: v })} />
              <div className="row" style={{ justifyContent: "flex-end", marginTop: 6 }}>
                <button className="btn" disabled={!dirty || busy !== null}
                  onClick={() => act("cfg", async () => setCfg((await backend.memorySetConfig(cfg)).config), "Regras guardadas")}>
                  Guardar
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </>
  );
}

function Slider({ label, unit, min, max, step, value, onChange }: { label: string; unit: string; min: number; max: number; step: number; value: number; onChange: (v: number) => void }) {
  return (
    <div style={{ marginBottom: 14 }}>
      <div className="row between" style={{ marginBottom: 6 }}>
        <span className="sub">{label}</span>
        <span className="mono" style={{ color: "var(--cyan)" }}>{value} {unit}</span>
      </div>
      <input type="range" min={min} max={max} step={step} value={value} onChange={(e) => onChange(Number(e.target.value))} />
    </div>
  );
}
