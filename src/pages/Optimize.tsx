import { useState } from "react";
import { Loader2, Sparkles, Trash2, XCircle } from "lucide-react";
import type { ProcInfo, TweakCategory, TweakInfo } from "../api";
import { backend } from "../mock";
import { PageHead, RiskChip, Toggle, fmtBytes, useLoad, useToast } from "../ui";

const CATS: { id: TweakCategory | "all"; label: string }[] = [
  { id: "all", label: "Todas" },
  { id: "power", label: "Energia" },
  { id: "system", label: "Sistema" },
  { id: "gpu", label: "GPU" },
  { id: "game", label: "Jogo" },
  { id: "input", label: "Rato" },
  { id: "network", label: "Rede" },
  { id: "privacy", label: "Privacidade" },
];

export default function Optimize() {
  const toast = useToast();
  const tw = useLoad(() => backend.listTweaks());
  const [cat, setCat] = useState<TweakCategory | "all">("all");
  const [busy, setBusy] = useState<string | null>(null);

  const toggle = async (t: TweakInfo, on: boolean) => {
    setBusy(t.id);
    try {
      const updated = on ? await backend.applyTweak(t.id) : await backend.revertTweak(t.id);
      tw.setData((list) => (list ?? []).map((x) => (x.id === t.id ? updated : x)));
      toast("ok", `${t.title}: ${on ? "aplicado" : "revertido"}${on && t.requiresReboot ? " (requer reinício)" : ""}`);
    } catch (e) {
      toast("err", `${t.title}: ${e}`);
    } finally {
      setBusy(null);
    }
  };

  const list = (tw.data ?? []).filter((t) => cat === "all" || t.category === cat);
  const counts = (id: TweakCategory | "all") => (tw.data ?? []).filter((t) => id === "all" || t.category === id).length;

  return (
    <>
      <PageHead eyebrow="// afinação do windows" title="Otimizações"
        desc="Cada otimização guarda o estado anterior antes de alterar o sistema e pode ser revertida a qualquer momento." />

      <div className="tabs" style={{ marginBottom: 18 }}>
        {CATS.filter((c) => counts(c.id) > 0).map((c) => (
          <button key={c.id} className={cat === c.id ? "active" : ""} onClick={() => setCat(c.id)}>
            {c.label} <span className="faint mono">{counts(c.id)}</span>
          </button>
        ))}
      </div>

      {tw.error && <div className="callout">{tw.error}</div>}
      <div className="stack" style={{ gap: 10 }}>
        {tw.loading && !tw.data && <div className="empty"><Loader2 className="spin" /></div>}
        {list.map((t) => (
          <div key={t.id} className={`tweak ${t.applied ? "applied" : ""}`} style={{ opacity: t.available ? 1 : 0.55 }}>
            <div>
              <h4>{t.title}</h4>
              <p>{t.description}</p>
              <div className="meta">
                <RiskChip risk={t.risk} />
                {t.requiresReboot && <span className="chip">reinício</span>}
                {t.applied && <span className="chip on"><span className="dot" />ativo</span>}
                {!t.available && t.reason && <span className="chip">{t.reason}</span>}
              </div>
            </div>
            <Toggle label={t.title} on={t.applied} busy={busy === t.id} disabled={!t.available || (busy !== null && busy !== t.id)}
              onChange={(v) => toggle(t, v)} />
          </div>
        ))}
      </div>

      <div className="grid-2" style={{ marginTop: 22 }}>
        <ShaderCache />
        <BackgroundApps />
      </div>
    </>
  );
}

function ShaderCache() {
  const toast = useToast();
  const [busy, setBusy] = useState(false);
  const [last, setLast] = useState<string | null>(null);
  const run = async () => {
    setBusy(true);
    try {
      const r = await backend.cleanShaderCache();
      setLast(`${fmtBytes(r.freedBytes)} libertados · ${r.filesRemoved} ficheiros`);
      toast("ok", `Cache de shaders limpa: ${fmtBytes(r.freedBytes)}`);
    } catch (e) {
      toast("err", String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="panel">
      <div className="row between">
        <div>
          <h3>Cache de shaders</h3>
          <div className="sub">Limpa as caches do DirectX, das drivers e do CS2. O jogo recompila os shaders no próximo arranque — útil depois de atualizar drivers ou quando há stutters.</div>
        </div>
      </div>
      <div className="row between" style={{ marginTop: 16 }}>
        <span className="mono faint" style={{ fontSize: 12 }}>{last ?? "—"}</span>
        <button className="btn" onClick={run} disabled={busy}>
          {busy ? <Loader2 size={15} className="spin" /> : <Trash2 size={15} />} Limpar
        </button>
      </div>
    </div>
  );
}

function BackgroundApps() {
  const toast = useToast();
  const apps = useLoad(() => backend.listBackgroundApps());
  const [sel, setSel] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const flip = (p: ProcInfo) =>
    setSel((s) => {
      const n = new Set(s);
      if (n.has(p.name)) n.delete(p.name);
      else n.add(p.name);
      return n;
    });
  const close = async () => {
    setBusy(true);
    try {
      const n = await backend.killBackgroundApps([...sel]);
      toast("ok", `${n} processo(s) fechado(s)`);
      setSel(new Set());
      await apps.reload();
    } catch (e) {
      toast("err", String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="panel">
      <h3>Apps em segundo plano</h3>
      <div className="sub" style={{ marginBottom: 12 }}>Fecha apps que consomem CPU e RAM durante o jogo. A Steam e o CS2 nunca são fechados.</div>
      {apps.data?.length === 0 && <div className="sub">Nenhuma app conhecida a correr.</div>}
      <div className="stack" style={{ gap: 6 }}>
        {(apps.data ?? []).map((p) => (
          <label key={p.pid} className="row between" style={{ cursor: "pointer", padding: "6px 4px" }}>
            <span className="row"><input type="checkbox" checked={sel.has(p.name)} onChange={() => flip(p)} />{p.label}</span>
            <span className="mono faint" style={{ fontSize: 12 }}>{p.memoryMb.toFixed(0)} MB</span>
          </label>
        ))}
      </div>
      <div className="row" style={{ justifyContent: "flex-end", marginTop: 12, gap: 8 }}>
        <button className="btn ghost" onClick={() => apps.reload()}><Sparkles size={14} /> Atualizar</button>
        <button className="btn danger" disabled={!sel.size || busy} onClick={close}><XCircle size={15} /> Fechar {sel.size || ""}</button>
      </div>
    </div>
  );
}
