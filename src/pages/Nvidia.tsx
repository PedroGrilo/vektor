import { useState } from "react";
import { Crosshair, Cpu, Loader2, RotateCcw, Sparkles } from "lucide-react";
import type { NvCs2Options, NvGlobalOptions, NvidiaStatus } from "../api";
import { backend } from "../mock";
import { PageHead, Toggle, useLoad, useToast } from "../ui";

const OPTIONS: { key: keyof NvGlobalOptions; title: string; desc: string }[] = [
  { key: "lowLatency", title: "Modo de baixa latência", desc: "Máximo de 1 frame pré-renderizado. Menos input lag em jogos limitados pela GPU." },
  { key: "maxPerformancePower", title: "Desempenho máximo da GPU", desc: "Evita quedas de clock durante o jogo. A GPU gasta mais energia em repouso." },
  { key: "textureHighPerformance", title: "Filtragem de texturas: alto desempenho", desc: "Inclui otimizações trilinear e anisotrópica. Ganho pequeno, ligeira perda de qualidade." },
  { key: "unlimitedShaderCache", title: "Cache de shaders ilimitada", desc: "Menos recompilação de shaders e menos stutters entre sessões." },
];

export default function Nvidia() {
  const toast = useToast();
  const st = useLoad(() => backend.nvidiaStatus());
  const [opts, setOpts] = useState<NvGlobalOptions>({ lowLatency: true, maxPerformancePower: true, textureHighPerformance: true, unlimitedShaderCache: true });
  const [busy, setBusy] = useState<"apply" | "revert" | null>(null);

  const run = async (kind: "apply" | "revert") => {
    setBusy(kind);
    try {
      const s = kind === "apply" ? await backend.nvidiaApplyGlobal(opts) : await backend.nvidiaRevertGlobal();
      st.setData(s);
      toast("ok", kind === "apply" ? "Perfil global NVIDIA otimizado" : "Perfil global NVIDIA restaurado");
    } catch (e) {
      toast("err", String(e));
    } finally {
      setBusy(null);
    }
  };

  const s = st.data;
  const blocked = !s?.nvidiaGpu || !s?.inspectorAvailable;
  return (
    <>
      <PageHead eyebrow="// nvidia" title="Perfil NVIDIA"
        desc="Otimiza o perfil global da driver NVIDIA (todos os jogos) com o NVIDIA Profile Inspector. O perfil original é guardado e pode ser reposto."
        right={s?.applied && <span className="chip on"><span className="dot" />perfil Vektor ativo</span>} />

      {s && !s.nvidiaGpu && <div className="callout" style={{ marginBottom: 16 }}>Não foi detetada nenhuma placa gráfica NVIDIA neste PC.</div>}
      {s && s.nvidiaGpu && !s.inspectorAvailable && <div className="callout" style={{ marginBottom: 16 }}>O NVIDIA Profile Inspector não está incluído nesta instalação.</div>}

      <div className="stack" style={{ gap: 10 }}>
        {OPTIONS.map((o) => (
          <div key={o.key} className={`tweak ${opts[o.key] ? "applied" : ""}`}>
            <div>
              <h4>{o.title}</h4>
              <p>{o.desc}</p>
            </div>
            <Toggle label={o.title} on={opts[o.key]} disabled={blocked || busy !== null} onChange={(v) => setOpts({ ...opts, [o.key]: v })} />
          </div>
        ))}
      </div>

      <div className="panel row wrap" style={{ marginTop: 16, gap: 16 }}>
        <Cpu size={22} color="var(--lime)" />
        <div style={{ flex: 1, minWidth: 260 }}>
          <h3>Aplicar ao perfil global</h3>
          <div className="sub">
            V-Sync, G-SYNC e limitador de FPS ficam como estão. Perfis de jogos específicos continuam a ter prioridade.
            {s?.inspectorVersion && <> Profile Inspector <span className="mono">{s.inspectorVersion}</span> (MIT).</>}
          </div>
        </div>
        {s?.applied && (
          <button className="btn danger" disabled={busy !== null} onClick={() => run("revert")}>
            {busy === "revert" ? <Loader2 size={15} className="spin" /> : <RotateCcw size={15} />} Repor original
          </button>
        )}
        <button className="btn primary" disabled={blocked || busy !== null || !Object.values(opts).some(Boolean)} onClick={() => run("apply")}>
          {busy === "apply" ? <Loader2 size={15} className="spin" /> : <Sparkles size={15} />} {s?.applied ? "Reaplicar" : "Aplicar"}
        </button>
      </div>

      {s && <Cs2Profile status={s} blocked={blocked} onChange={st.setData} />}
    </>
  );
}

const CS2_OPTIONS: { key: keyof NvCs2Options; title: string }[] = [
  { key: "lowLatency", title: "Baixa latência" },
  { key: "maxPerformancePower", title: "Desempenho máximo" },
  { key: "vsyncOff", title: "V-Sync forçado desligado" },
  { key: "textureHighPerformance", title: "Texturas: alto desempenho" },
];

function Cs2Profile({ status, blocked, onChange }: { status: NvidiaStatus; blocked: boolean; onChange: (s: NvidiaStatus) => void }) {
  const toast = useToast();
  const [opts, setOpts] = useState<NvCs2Options>({ lowLatency: true, maxPerformancePower: true, vsyncOff: true, textureHighPerformance: true });
  const [busy, setBusy] = useState<"apply" | "revert" | null>(null);
  const run = async (kind: "apply" | "revert") => {
    setBusy(kind);
    try {
      onChange(kind === "apply" ? await backend.nvidiaApplyCs2(opts) : await backend.nvidiaRevertCs2());
      toast("ok", kind === "apply" ? "Perfil NVIDIA do CS2 otimizado" : "Perfil NVIDIA do CS2 restaurado");
    } catch (e) {
      toast("err", String(e));
    } finally {
      setBusy(null);
    }
  };
  return (
    <div className="panel corners" style={{ marginTop: 16 }}>
      <div className="row between wrap" style={{ gap: 12 }}>
        <div className="row">
          <Crosshair size={20} color="var(--cyan)" />
          <div>
            <h3>Perfil dedicado ao CS2</h3>
            <div className="sub">Só afeta o cs2.exe e tem prioridade sobre o perfil global.</div>
          </div>
        </div>
        {status.cs2Applied && <span className="chip on"><span className="dot" />ativo</span>}
      </div>
      <div className="grid-2" style={{ margin: "14px 0", gap: 10 }}>
        {CS2_OPTIONS.map((o) => (
          <div key={o.key} className="row between" style={{ padding: "8px 4px" }}>
            <span>{o.title}</span>
            <Toggle label={o.title} on={opts[o.key]} disabled={blocked || busy !== null} onChange={(v) => setOpts({ ...opts, [o.key]: v })} />
          </div>
        ))}
      </div>
      <div className="row" style={{ justifyContent: "flex-end", gap: 8 }}>
        {status.cs2Applied && (
          <button className="btn danger" disabled={busy !== null} onClick={() => run("revert")}>
            {busy === "revert" ? <Loader2 size={15} className="spin" /> : <RotateCcw size={15} />} Repor original
          </button>
        )}
        <button className="btn primary" disabled={blocked || busy !== null || !Object.values(opts).some(Boolean)} onClick={() => run("apply")}>
          {busy === "apply" ? <Loader2 size={15} className="spin" /> : <Sparkles size={15} />} {status.cs2Applied ? "Reaplicar" : "Aplicar ao CS2"}
        </button>
      </div>
    </div>
  );
}
