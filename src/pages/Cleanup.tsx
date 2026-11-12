import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Loader2, RefreshCw, Trash2 } from "lucide-react";
import { backend } from "../mock";
import { PageHead, fmtBytes, useLoad, useToast } from "../ui";

export default function Cleanup() {
  const toast = useToast();
  const scan = useLoad(() => backend.cleanScan());
  const [sel, setSel] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);

  // Default selection = recommended targets that have something to clean.
  useEffect(() => {
    if (scan.data) setSel(new Set(scan.data.filter((t) => t.recommended && t.available).map((t) => t.id)));
  }, [scan.data]);

  const total = useMemo(
    () => (scan.data ?? []).filter((t) => sel.has(t.id)).reduce((a, t) => a + t.sizeBytes, 0),
    [scan.data, sel],
  );

  const flip = (id: string) =>
    setSel((s) => {
      const n = new Set(s);
      if (n.has(id)) n.delete(id);
      else n.add(id);
      return n;
    });

  const run = async () => {
    setBusy(true);
    try {
      const r = await backend.cleanRun([...sel]);
      toast("ok", `${fmtBytes(r.freedBytes)} libertados (${r.filesRemoved} ficheiros)`);
      await scan.reload();
    } catch (e) {
      toast("err", String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <PageHead eyebrow="// limpeza" title="Limpeza"
        desc="Liberta espaço em disco. Os ficheiros em uso são ignorados. A limpeza não aumenta FPS por si só — a cache de shaders ajuda contra stutters depois de atualizar drivers."
        right={
          <div className="row">
            <button className="btn ghost" onClick={() => scan.reload()} disabled={scan.loading || busy}>
              <RefreshCw size={15} className={scan.loading ? "spin" : ""} /> Analisar
            </button>
            <button className="btn primary" onClick={run} disabled={!sel.size || busy || scan.loading}>
              {busy ? <Loader2 size={16} className="spin" /> : <Trash2 size={16} />} Limpar {fmtBytes(total)}
            </button>
          </div>
        } />

      {scan.error && <div className="callout" style={{ marginBottom: 16 }}>{scan.error}</div>}
      {scan.loading && !scan.data && <div className="empty"><Loader2 className="spin" /> A analisar o disco…</div>}

      <div className="grid-2">
        {(scan.data ?? []).map((t) => (
          <label key={t.id} className={`tweak ${sel.has(t.id) ? "applied" : ""}`} style={{ cursor: t.available ? "pointer" : "default", opacity: t.available ? 1 : 0.5 }}>
            <div>
              <h4 className="row" style={{ gap: 10 }}>
                <input type="checkbox" checked={sel.has(t.id)} disabled={!t.available} onChange={() => flip(t.id)} />
                {t.title}
              </h4>
              <p>{t.description}</p>
              <div className="meta">
                {!t.recommended && <span className="chip medium">opcional</span>}
                {t.files > 0 && <span className="chip">{t.files.toLocaleString("pt-PT")} ficheiros</span>}
              </div>
              {t.warning && (
                <p className="row" style={{ marginTop: 8, color: "var(--warn)", gap: 6, alignItems: "flex-start" }}>
                  <AlertTriangle size={14} style={{ flexShrink: 0, marginTop: 2 }} /> {t.warning}
                </p>
              )}
            </div>
            <div className="value" style={{ fontSize: 16, textAlign: "right" }}>{t.sizeBytes ? fmtBytes(t.sizeBytes) : "—"}</div>
          </label>
        ))}
      </div>
    </>
  );
}
