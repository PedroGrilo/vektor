import { useEffect, useState } from "react";
import { CircleAlert, Loader2, MemoryStick, Power, Wand2, Zap } from "lucide-react";
import type { MemoryConfig, MemoryStatus } from "../api";
import { backend } from "../mock";
import { nominalGb, suggestedRules } from "../suggest";
import { PageHead, Toggle, fmtMb, usePoll, useToast } from "../ui";

const time = (iso: string) => new Date(iso).toLocaleTimeString("pt-PT");

export default function Memory() {
  const toast = useToast();
  const [error, setError] = useState<string | null>(null);
  const [s, setLive] = usePoll(async () => {
    try {
      const v = await backend.memoryStatus();
      setError(null);
      return v;
    } catch (e) {
      setError(String(e));
      throw e;
    }
  }, 2000);
  const [cfg, setCfg] = useState<MemoryConfig | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    if (s && !cfg) setCfg(s.config);
  }, [s, cfg]);

  // Every action returns the fresh status, so the page updates without waiting for the next poll.
  const act = async (key: string, fn: () => Promise<MemoryStatus>, ok: string) => {
    setBusy(key);
    try {
      const next = await fn();
      setLive(next);
      if (key === "cfg") setCfg(next.config);
      toast("ok", ok);
    } catch (e) {
      toast("err", String(e));
    } finally {
      setBusy(null);
    }
  };

  const dirty = !!cfg && !!s && JSON.stringify(cfg) !== JSON.stringify(s.config);
  // Thresholds up to half the RAM make sense on 32/64 GB machines.
  const maxThreshold = Math.max(8192, Math.floor((s?.totalMb ?? 0) / 2 / 256) * 256);
  // Rules suggested for this PC's RAM. Applying them only moves the sliders: saving stays explicit.
  const suggestion = s ? suggestedRules(s.totalMb) : null;
  const usingSuggestion = !!cfg && !!suggestion
    && cfg.standbyThresholdMb === suggestion.standbyThresholdMb
    && cfg.freeThresholdMb === suggestion.freeThresholdMb
    && cfg.intervalSec === suggestion.intervalSec;

  return (
    <>
      <PageHead eyebrow="// memória" title="Limpeza de Standby"
        desc="Como o ISLC: um serviço do Windows vigia a lista de standby e purga-a quando a memória livre fica baixa. Ajuda contra stutters em PCs com pouca RAM livre."
        right={
          <button className="btn" disabled={busy !== null} onClick={() => act("purge", backend.memoryPurgeNow, "Lista de standby purgada")}>
            {busy === "purge" ? <Loader2 size={15} className="spin" /> : <Zap size={15} />} Purgar agora
          </button>
        } />

      {error && !s && (
        <div className="callout" style={{ marginBottom: 16 }}><CircleAlert size={18} /><span>Não foi possível ler a memória: {error}</span></div>
      )}

      <div className="hero" style={{ alignItems: "start" }}>
        <div className="stack">
          <Composition s={s} />
          {s && <RuleState s={s} />}
        </div>

        <div className="stack">
          <div className="panel">
            <div className="row between">
              <div className="row"><Power size={18} color={s?.serviceRunning ? "var(--ok)" : "var(--text-faint)"} />
                <div>
                  <h3>Serviço VektorMemory</h3>
                  <div className="sub">{!s ? "A verificar…" : s.serviceRunning ? "A correr — arranca com o Windows." : s.serviceInstalled ? "Instalado, mas parado." : "Não instalado."}</div>
                </div>
              </div>
              {s?.serviceInstalled ? (
                <div className="row" style={{ gap: 8 }}>
                  {!s.serviceRunning && (
                    <button className="btn primary" disabled={busy !== null} onClick={() => act("svc", backend.memoryInstallService, "Serviço iniciado")}>
                      {busy === "svc" && <Loader2 size={15} className="spin" />} Iniciar
                    </button>
                  )}
                  <button className="btn danger" disabled={busy !== null} onClick={() => act("rm", backend.memoryUninstallService, "Serviço removido")}>
                    {busy === "rm" && <Loader2 size={15} className="spin" />} Remover
                  </button>
                </div>
              ) : (
                <button className="btn primary" disabled={busy !== null || !s} onClick={() => act("svc", backend.memoryInstallService, "Serviço instalado e iniciado")}>
                  {busy === "svc" && <Loader2 size={15} className="spin" />} Instalar
                </button>
              )}
            </div>
            {s?.lastError && (
              <div className="callout" style={{ marginTop: 14 }}><CircleAlert size={18} /><span>O serviço reportou um erro: {s.lastError}</span></div>
            )}
          </div>

          {cfg && (
            <div className="panel">
              <div className="row between" style={{ marginBottom: 14 }}>
                <h3 className="row"><MemoryStick size={16} color="var(--accent)" /> Regras</h3>
                <Toggle label="Ativo" on={cfg.enabled} onChange={(v) => setCfg({ ...cfg, enabled: v })} />
              </div>
              <Slider label="Purgar quando a standby passar" unit="MB" min={256} max={maxThreshold} step={256}
                value={cfg.standbyThresholdMb} onChange={(v) => setCfg({ ...cfg, standbyThresholdMb: v })} />
              <Slider label="…e a memória livre for menor que" unit="MB" min={256} max={maxThreshold} step={256}
                value={cfg.freeThresholdMb} onChange={(v) => setCfg({ ...cfg, freeThresholdMb: v })} />
              <Slider label="Verificar a cada" unit="s" min={2} max={60} step={1}
                value={cfg.intervalSec} onChange={(v) => setCfg({ ...cfg, intervalSec: v })} />
              {s && suggestion && (
                <div className="suggest">
                  <div className="sub">
                    <b>Sugerido para {nominalGb(s.totalMb)} GB de RAM</b><br />
                    standby {suggestion.standbyThresholdMb} MB · livre {suggestion.freeThresholdMb} MB · a cada {suggestion.intervalSec} s
                  </div>
                  {usingSuggestion
                    ? <span className="chip on">em uso</span>
                    : <button className="btn" onClick={() => setCfg({ ...cfg, ...suggestion })}><Wand2 size={15} /> Usar sugerido</button>}
                </div>
              )}
              <div className="row between" style={{ marginTop: 6 }}>
                <span className="sub">{dirty ? "Alterações por guardar." : "O serviço lê as regras a cada verificação."}</span>
                <button className="btn primary" disabled={!dirty || busy !== null} onClick={() => act("cfg", () => backend.memorySetConfig(cfg), "Regras guardadas")}>
                  {busy === "cfg" && <Loader2 size={15} className="spin" />} Guardar
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </>
  );
}

/** One stacked bar for the whole RAM instead of two unrelated dials. */
function Composition({ s }: { s: MemoryStatus | null }) {
  const total = s?.totalMb ?? 0;
  const used = s ? Math.max(0, total - s.freeMb - s.standbyMb - s.modifiedMb) : 0;
  const parts = [
    { key: "used", label: "Em uso", mb: used, hint: "Programas e sistema" },
    { key: "modified", label: "Modificada", mb: s?.modifiedMb ?? 0, hint: "À espera de ir para disco" },
    { key: "standby", label: "Standby", mb: s?.standbyMb ?? 0, hint: "Cache que a purga liberta" },
    { key: "free", label: "Livre", mb: s?.freeMb ?? 0, hint: "Pronta a usar" },
  ];
  return (
    <div className="panel corners">
      <div className="mem-total">
        <div>
          <div className="label">Memória RAM</div>
          <div className="value">{s ? fmtMb(total) : "—"}</div>
        </div>
        <div className="sub" style={{ textAlign: "right" }}>
          {s ? <>{Math.round((used / total) * 100)}% em uso<br />{fmtMb(s.standbyMb + s.freeMb)} disponíveis</> : "A ler…"}
        </div>
      </div>
      <div className="mem-bar" role="img" aria-label="Composição da memória">
        {parts.map((p) => <i key={p.key} className={`seg-${p.key}`} style={{ flex: `0 0 ${total ? (p.mb / total) * 100 : 0}%` }} />)}
      </div>
      <div className="mem-legend">
        {parts.map((p) => (
          <div key={p.key} className="mem-stat">
            <i className={`seg-${p.key}`} />
            <div>
              <div className="label">{p.label}</div>
              <div className="value">{s ? fmtMb(p.mb) : "—"}</div>
              <div className="sub">{p.hint}</div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

/** Says in plain words whether the service will purge and, if not, which number is holding it back. */
function RuleState({ s }: { s: MemoryStatus }) {
  const c = s.config;
  const standbyOk = s.standbyMb >= c.standbyThresholdMb;
  const freeOk = s.freeMb < c.freeThresholdMb;
  // Plenty of free memory is the usual reason, so it is explained first.
  const verdict = !s.serviceRunning
    ? "O serviço não está a correr — instala-o ou inicia-o para purgar automaticamente."
    : !c.enabled
      ? "As regras estão desligadas: o serviço não purga."
      : standbyOk && freeOk
        ? "As duas condições estão cumpridas: o serviço purga na próxima verificação."
        : !freeOk
          ? `Não é preciso purgar: tens ${fmtMb(s.freeMb)} livres e a regra só atua abaixo de ${fmtMb(c.freeThresholdMb)}.`
          : `Não é preciso purgar: a standby tem ${fmtMb(s.standbyMb)} e a regra só atua a partir de ${fmtMb(c.standbyThresholdMb)}.`;
  return (
    <div className="panel">
      <h3>Estado da regra</h3>
      <div className="sub" style={{ marginBottom: 4 }}>{verdict}</div>
      <div className="sub faint" style={{ marginBottom: 10 }}>O serviço só purga quando as duas condições se cumprem ao mesmo tempo.</div>
      <div className="rule-line"><span>Standby de pelo menos {fmtMb(c.standbyThresholdMb)} · agora <b>{fmtMb(s.standbyMb)}</b></span><Met ok={standbyOk} /></div>
      <div className="rule-line"><span>Menos de {fmtMb(c.freeThresholdMb)} livres · agora <b>{fmtMb(s.freeMb)}</b></span><Met ok={freeOk} /></div>
      <div className="rule-line">
        <span>Purgas automáticas <b>{s.purgeCount}</b>{s.lastPurge ? ` · última às ${time(s.lastPurge)}` : ""}</span>
        <span className="mono faint">{s.lastCheck ? `verificado ${time(s.lastCheck)}` : ""}</span>
      </div>
    </div>
  );
}

const Met = ({ ok }: { ok: boolean }) => <span className={`chip ${ok ? "on" : ""}`}>{ok ? "cumprida" : "por cumprir"}</span>;

function Slider({ label, unit, min, max, step, value, onChange }: { label: string; unit: string; min: number; max: number; step: number; value: number; onChange: (v: number) => void }) {
  return (
    <div style={{ marginBottom: 14 }}>
      <div className="row between" style={{ marginBottom: 6 }}>
        <span className="sub">{label}</span>
        <span className="mono" style={{ color: "var(--accent)" }}>{value} {unit}</span>
      </div>
      <input type="range" aria-label={label} min={min} max={max} step={step} value={value} onChange={(e) => onChange(Number(e.target.value))} />
    </div>
  );
}
