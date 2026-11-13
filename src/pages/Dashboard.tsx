import { useState } from "react";
import { ArrowRight, Cpu, Gamepad2, MemoryStick, MonitorCog, Rocket, ShieldCheck } from "lucide-react";
import type { PageId } from "../App";
import { backend } from "../mock";
import { Gauge, PageHead, useLiveStats, useLoad, useToast } from "../ui";

export default function Dashboard({ go }: { go: (p: PageId) => void }) {
  const toast = useToast();
  const sys = useLoad(() => backend.systemInfo());
  const tw = useLoad(() => backend.listTweaks());
  const stats = useLiveStats();
  const [boosting, setBoosting] = useState(false);

  const available = (tw.data ?? []).filter((t) => t.available);
  const applied = available.filter((t) => t.applied);
  const score = available.length ? Math.round((applied.length / available.length) * 100) : 0;
  // One-click boost only touches low-risk tweaks that need no reboot and ask nothing.
  const quick = available.filter((t) => !t.applied && t.risk === "low" && !t.requiresReboot && !t.question);

  const boost = async () => {
    setBoosting(true);
    let ok = 0;
    const errors: string[] = [];
    for (const t of quick) {
      try {
        await backend.applyTweak(t.id);
        ok++;
      } catch (e) {
        errors.push(`${t.title}: ${e}`);
      }
    }
    await tw.reload();
    setBoosting(false);
    if (errors.length) toast("err", `${ok} aplicadas, ${errors.length} falharam. ${errors[0]}`);
    else toast("ok", ok ? `${ok} otimizações aplicadas. Tudo pode ser revertido em Restaurar.` : "Já está tudo otimizado.");
  };

  const s = sys.data;
  return (
    <>
      <PageHead eyebrow="// painel de controlo" title="Desempenho" desc="Estado do sistema e otimização num clique. Cada alteração fica registada e pode ser desfeita." />

      <div className="hero">
        <div className="panel corners boost-core">
          <div className={`boost-ring ${boosting ? "active" : ""}`}>
            <div className="r r3" />
            <div className="r" />
            <div className="r r2" />
            <div className="arc" />
            <div className="core">
              <div>
                <div className="value">{score}</div>
                <div className="label">índice</div>
              </div>
            </div>
          </div>
          <button className="btn primary big" disabled={boosting || tw.loading} onClick={boost}>
            <Rocket size={18} />
            {boosting ? "A otimizar…" : tw.loading && !tw.data ? "A analisar…" : quick.length ? `Otimizar agora (${quick.length})` : "Sistema otimizado"}
          </button>
          <p className="sub" style={{ marginTop: 14, maxWidth: 360 }}>
            Aplica só otimizações de <b style={{ color: "var(--ok)" }}>risco baixo</b> que não exigem reinício.
            As restantes estão em <a onClick={() => go("optimize")}>Otimizações</a>.
          </p>
        </div>

        <div className="stack">
          <div className="panel">
            <div className="row" style={{ justifyContent: "space-around" }}>
              <Gauge value={stats?.cpuUsage ?? 0} label="CPU" />
              <Gauge value={stats ? (stats.ramUsedGb / stats.ramTotalGb) * 100 : 0} label="RAM" color="var(--accent-2)" />
            </div>
          </div>
          <div className="panel">
            <div className="row between">
              <div>
                <div className="label">Otimizações ativas</div>
                <div className="value">{applied.length}<span className="faint" style={{ fontSize: 15 }}> / {available.length}</span></div>
              </div>
              <button className="btn" onClick={() => go("optimize")}>Ver todas <ArrowRight size={15} /></button>
            </div>
            <div className="bar" style={{ marginTop: 14 }}><i style={{ width: `${score}%` }} /></div>
          </div>
        </div>
      </div>

      <div className="grid-3" style={{ marginTop: 16 }}>
        <Spec icon={<Cpu size={18} />} label="Processador" value={s?.cpu.name ?? "—"} extra={s ? `${s.cpu.cores} núcleos · ${s.cpu.threads} threads${s.cpu.zenGen ? ` · Zen ${s.cpu.zenGen}` : ""}` : ""} />
        <Spec icon={<MonitorCog size={18} />} label="Gráfica" value={s?.gpus[0]?.name ?? "—"} extra={s?.gpus[0] ? `${s.gpus[0].vramMb ? `${Math.round(s.gpus[0].vramMb / 1024)} GB · ` : ""}driver ${s.gpus[0].driverVersion ?? "?"}` : ""} />
        <Spec icon={<MemoryStick size={18} />} label="Sistema" value={s ? `${s.osName}` : "—"} extra={s ? `build ${s.osBuild} · ${s.ramTotalGb} GB RAM` : ""} />
      </div>

      <div className="grid-2" style={{ marginTop: 16 }}>
        <div className="panel row" style={{ gap: 16 }}>
          <Gamepad2 size={22} color="var(--accent)" />
          <div style={{ flex: 1 }}>
            <h3>Os teus jogos</h3>
            <div className="sub">Otimiza cada jogo da Steam, Epic, GOG, Ubisoft e Riot, e escolhe quais ativam o Modo Jogo.</div>
          </div>
          <button className="btn" onClick={() => go("games")}>Abrir <ArrowRight size={15} /></button>
        </div>
        <div className="panel row" style={{ gap: 16 }}>
          <ShieldCheck size={22} color="var(--ok)" />
          <div style={{ flex: 1 }}>
            <h3>Tudo reversível</h3>
            <div className="sub">Backup antes de cada alteração. {s && !s.isAdmin && <span style={{ color: "var(--warn)" }}>Sem privilégios de administrador.</span>}</div>
          </div>
          <button className="btn" onClick={() => go("restore")}>Restaurar <ArrowRight size={15} /></button>
        </div>
      </div>
    </>
  );
}

function Spec({ icon, label, value, extra }: { icon: React.ReactNode; label: string; value: string; extra: string }) {
  return (
    <div className="panel">
      <div className="row" style={{ color: "var(--accent)", marginBottom: 10 }}>{icon}<span className="label">{label}</span></div>
      <div style={{ fontWeight: 600, fontSize: 15, marginBottom: 4 }}>{value}</div>
      <div className="sub mono" style={{ fontSize: 11.5 }}>{extra}</div>
    </div>
  );
}
