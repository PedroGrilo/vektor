import { useEffect, useState } from "react";
import { AlertTriangle, Check, Copy, Gauge as GaugeIcon, Loader2, Play, Sparkles, Target, Trash2 } from "lucide-react";
import type { BenchRecord, BenchStatus, ConfigPreset } from "../api";
import { backend } from "../mock";
import { PageHead, useLoad, useToast } from "../ui";

const TAG_LABEL: Record<ConfigPreset["tag"], string> = {
  fps: "FPS", competitive: "competitivo", visibility: "visibilidade", lowend: "PC modesto",
};

export default function Cs2() {
  const toast = useToast();
  const inst = useLoad(() => backend.cs2Detect());
  const presets = useLoad(() => backend.cs2Presets());
  const launchOpts = useLoad(() => backend.cs2RecommendedLaunchOptions());
  const [account, setAccount] = useState<number | null>(null);
  const [chosen, setChosen] = useState<string | null>(null);
  const [reloadKey, setReloadKey] = useState("F5");
  const [applying, setApplying] = useState(false);
  const [copied, setCopied] = useState(false);

  const users = inst.data?.users ?? [];
  useEffect(() => {
    if (account === null && users.length) setAccount((users.find((u) => u.mostRecent) ?? users[0]).accountId);
  }, [users, account]);

  const apply = async () => {
    const p = presets.data?.find((x) => x.id === chosen);
    if (!p || account === null) return;
    setApplying(true);
    try {
      await backend.cs2ApplyConfig(account, { ...p.config, reloadKey: reloadKey.trim() || null });
      toast("ok", `Preset "${p.name}" aplicado. No jogo, prime ${reloadKey || "exec vektor"} para recarregar.`);
    } catch (e) {
      toast("err", String(e));
    } finally {
      setApplying(false);
    }
  };

  const copy = async () => {
    if (!launchOpts.data) return;
    await navigator.clipboard.writeText(launchOpts.data);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const i = inst.data;
  return (
    <>
      <PageHead eyebrow="// counter-strike 2" title="CS2"
        desc="Configs otimizadas, opções de arranque e benchmark de FPS. Só ficheiros de configuração oficiais — nada toca na memória do jogo."
        right={
          <button className="btn primary" disabled={!i?.installed || i.running} onClick={() => backend.cs2Launch([]).catch((e) => toast("err", String(e)))}>
            <Play size={16} /> {i?.running ? "Em execução" : "Jogar"}
          </button>
        } />

      {inst.error && <div className="callout" style={{ marginBottom: 16 }}>{inst.error}</div>}
      {i?.updateRequired && (
        <div className="callout" style={{ marginBottom: 16 }}>
          <AlertTriangle size={18} /> <span>A Steam indica que o CS2 tem uma atualização pendente. Atualiza antes de jogar ou correr o benchmark.</span>
        </div>
      )}

      <div className="grid-3">
        <div className="panel corners" style={{ gridColumn: "span 2" }}>
          <div className="label" style={{ marginBottom: 10 }}>Instalação</div>
          <dl className="kv">
            <dt>estado</dt>
            <dd>{i ? (i.installed ? <span className="chip on"><span className="dot" />instalado</span> : <span className="chip high">não encontrado</span>) : "…"}</dd>
            <dt>jogo</dt><dd className="mono" title={i?.gamePath ?? ""}>{i?.gamePath ?? "—"}</dd>
            <dt>steam</dt><dd className="mono">{i?.steamPath ?? "—"}</dd>
            <dt>arranque</dt><dd className="mono">{i?.launchOptions || "(sem opções)"}</dd>
            <dt>build</dt><dd className="mono">{i?.buildId ?? "—"}{i && !i.updateRequired && i.installed ? " · atualizado" : ""}</dd>
          </dl>
        </div>
        <div className="panel">
          <div className="label" style={{ marginBottom: 10 }}>Conta Steam</div>
          {users.length === 0 && <div className="sub">Nenhuma conta encontrada.</div>}
          <select className="input" value={account ?? ""} onChange={(e) => setAccount(Number(e.target.value))} disabled={!users.length}>
            {users.map((u) => <option key={u.accountId} value={u.accountId}>{u.personaName}{u.mostRecent ? " · recente" : ""}</option>)}
          </select>
          <div className="sub mono" style={{ marginTop: 8, fontSize: 11 }}>{users.find((u) => u.accountId === account)?.steamId64 ?? ""}</div>
        </div>
      </div>

      <h3 style={{ margin: "26px 0 12px", fontSize: 13 }} className="label">Presets de configuração</h3>
      <div className="grid-2">
        {(presets.data ?? []).map((p) => (
          <button key={p.id} className={`tweak ${chosen === p.id ? "applied" : ""}`} style={{ textAlign: "left", cursor: "pointer", color: "inherit" }} onClick={() => setChosen(p.id)}>
            <div>
              <h4 className="row" style={{ gap: 8 }}><Target size={15} color="var(--accent)" />{p.name}</h4>
              <p>{p.description}</p>
              <div className="meta">
                <span className="chip">{TAG_LABEL[p.tag]}</span>
                <span className="chip">{Object.keys(p.config.convars).length} convars</span>
                <span className="chip">{Object.keys(p.config.video.values).length} vídeo</span>
              </div>
            </div>
            {chosen === p.id ? <Check color="var(--accent)" /> : <span />}
          </button>
        ))}
      </div>
      <div className="panel row wrap" style={{ marginTop: 12, gap: 16 }}>
        <div style={{ flex: 1, minWidth: 260 }}>
          <h3>Aplicar à conta selecionada</h3>
          <div className="sub">Escreve <span className="mono">vektor.cfg</span>, liga-o ao <span className="mono">autoexec.cfg</span> e ajusta o vídeo. Fecha o CS2 antes. Backup automático.</div>
        </div>
        <label className="row" style={{ gap: 8 }}>
          <span className="label">tecla recarregar</span>
          <input className="input" style={{ width: 80 }} value={reloadKey} onChange={(e) => setReloadKey(e.target.value)} />
        </label>
        <button className="btn primary" disabled={!chosen || account === null || applying || i?.running} onClick={apply}>
          {applying ? <Loader2 size={15} className="spin" /> : <Sparkles size={15} />} Aplicar preset
        </button>
      </div>

      <div className="grid-2" style={{ marginTop: 16 }}>
        <div className="panel">
          <h3>Opções de arranque recomendadas</h3>
          <div className="sub" style={{ marginBottom: 12 }}>Steam → Biblioteca → CS2 → Propriedades → Opções de arranque.</div>
          <div className="row">
            <div className="code" style={{ flex: 1 }}>{launchOpts.data ?? "…"}</div>
            <button className="btn" onClick={copy} aria-label="Copiar">{copied ? <Check size={15} /> : <Copy size={15} />}</button>
          </div>
        </div>
        <Benchmark disabled={!i?.installed || !!i?.running} />
      </div>
      <BenchHistory />
    </>
  );
}

function Benchmark({ disabled }: { disabled: boolean }) {
  const toast = useToast();
  const [st, setSt] = useState<BenchStatus | null>(null);
  const active = st?.state === "running" || st?.state === "waiting-for-game";

  useEffect(() => {
    if (!active) return;
    const id = setInterval(async () => {
      try {
        setSt(await backend.cs2BenchmarkPoll());
      } catch {
        /* keep polling */
      }
    }, 2000);
    return () => clearInterval(id);
  }, [active]);

  const start = async () => {
    try {
      await backend.cs2BenchmarkStart();
      setSt(await backend.cs2BenchmarkPoll());
    } catch (e) {
      toast("err", String(e));
    }
  };

  return (
    <div className="panel corners">
      <div className="row between">
        <div>
          <h3>Benchmark de FPS</h3>
          <div className="sub">Corre o mapa de benchmark da Workshop e mede o FPS médio e o 1% low.</div>
        </div>
        <GaugeIcon color="var(--accent-2)" />
      </div>
      <div className="row" style={{ gap: 28, margin: "18px 0" }}>
        <div><div className="label">média</div><div className="value">{st?.avgFps?.toFixed(0) ?? "—"}</div></div>
        <div><div className="label">1% low</div><div className="value" style={{ color: "var(--accent-2)" }}>{st?.p1Fps?.toFixed(0) ?? "—"}</div></div>
        <div className="dim" style={{ flex: 1, fontSize: 12.5 }}>{st?.message ?? (st?.state === "done" ? "Concluído." : "")}</div>
      </div>
      <button className="btn" disabled={disabled || active} onClick={start}>
        {active ? <Loader2 size={15} className="spin" /> : <Play size={15} />} {active ? "A correr…" : "Iniciar benchmark"}
      </button>
    </div>
  );
}

function BenchHistory() {
  const toast = useToast();
  const hist = useLoad(() => backend.benchHistory());
  const list = hist.data ?? [];
  const baseline = list.length ? list[list.length - 1] : null;
  const best = Math.max(1, ...list.map((b) => b.avgFps));
  const delta = (b: BenchRecord) => (baseline && b.id !== baseline.id ? ((b.avgFps - baseline.avgFps) / baseline.avgFps) * 100 : null);

  const act = async (fn: () => Promise<BenchRecord[]>) => {
    try {
      hist.setData(await fn());
    } catch (e) {
      toast("err", String(e));
    }
  };

  return (
    <div className="panel" style={{ marginTop: 16 }}>
      <div className="row between" style={{ marginBottom: 12 }}>
        <div>
          <h3>Histórico de benchmarks</h3>
          <div className="sub">Compara com a primeira medição{baseline ? ` (${new Date(baseline.date).toLocaleDateString("pt-PT")})` : ""}. Corre um antes e outro depois de otimizar.</div>
        </div>
        <button className="btn ghost" onClick={() => hist.reload()}>Atualizar</button>
      </div>
      {list.length === 0 && <div className="sub">Ainda não há resultados.</div>}
      <div className="stack" style={{ gap: 8 }}>
        {list.map((b) => {
          const d = delta(b);
          return (
            <div key={b.id} className="row" style={{ gap: 14, padding: "8px 4px", borderBottom: "1px solid var(--line)" }}>
              <span className="mono faint" style={{ width: 128, fontSize: 12 }}>{new Date(b.date).toLocaleString("pt-PT", { dateStyle: "short", timeStyle: "short" })}</span>
              <div style={{ flex: 1 }}>
                <div className="bar"><i style={{ width: `${(b.avgFps / best) * 100}%` }} /></div>
              </div>
              <span className="mono" style={{ width: 70, textAlign: "right" }}>{b.avgFps.toFixed(0)}</span>
              <span className="mono" style={{ width: 70, textAlign: "right", color: "var(--accent-2)" }}>{b.p1Fps.toFixed(0)}</span>
              <span className="mono" style={{ width: 64, textAlign: "right", color: d === null ? "var(--text-faint)" : d >= 0 ? "var(--ok)" : "var(--danger)" }}>
                {d === null ? "base" : `${d >= 0 ? "+" : ""}${d.toFixed(1)}%`}
              </span>
              <input className="input" style={{ width: 170 }} placeholder="nota" defaultValue={b.note ?? ""}
                onBlur={(e) => e.target.value !== (b.note ?? "") && act(() => backend.benchSetNote(b.id, e.target.value))} />
              <button className="btn ghost" aria-label="Apagar" onClick={() => act(() => backend.benchDelete(b.id))}><Trash2 size={14} /></button>
            </div>
          );
        })}
      </div>
      {list.length > 0 && <div className="sub mono" style={{ marginTop: 8, fontSize: 11 }}>colunas: média · 1% low · variação da média</div>}
    </div>
  );
}
