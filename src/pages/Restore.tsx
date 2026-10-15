import { useState } from "react";
import { History, Loader2, RotateCcw } from "lucide-react";
import { backend } from "../mock";
import { PageHead, RiskChip, useLoad, useToast } from "../ui";

export default function Restore() {
  const toast = useToast();
  const tw = useLoad(() => backend.listTweaks());
  const [busy, setBusy] = useState<string | null>(null);
  const applied = (tw.data ?? []).filter((t) => t.applied);

  const revert = async (id: string, title: string) => {
    setBusy(id);
    try {
      await backend.revertTweak(id);
      toast("ok", `${title} revertido`);
      await tw.reload();
    } catch (e) {
      toast("err", `${title}: ${e}`);
    } finally {
      setBusy(null);
    }
  };

  const revertAll = async () => {
    setBusy("*");
    try {
      await backend.revertAll();
      toast("ok", "Sistema restaurado ao estado anterior ao Vektor.");
    } catch (e) {
      toast("err", String(e));
    } finally {
      await tw.reload();
      setBusy(null);
    }
  };

  return (
    <>
      <PageHead eyebrow="// segurança" title="Restaurar"
        desc="Tudo o que o Vektor altera fica registado com o valor original. Reverte uma otimização ou volta ao estado inicial de uma vez."
        right={
          <button className="btn danger" disabled={busy !== null} onClick={revertAll}>
            {busy === "*" ? <Loader2 size={15} className="spin" /> : <RotateCcw size={15} />} Reverter tudo
          </button>
        } />

      <div className="callout info" style={{ marginBottom: 18 }}>
        <History size={18} />
        <span>Os registos ficam em <span className="mono">%ProgramData%\Vektor\journal.json</span> e os backups de ficheiros em <span className="mono">backups\</span>. Sobrevivem a falhas e reinícios. As configs do CS2 aparecem como <span className="mono">cs2-config</span>.</span>
      </div>

      {applied.length === 0 && !tw.loading && (
        <div className="panel empty">
          <History size={28} color="var(--text-faint)" />
          <div>Nenhuma alteração ativa. O sistema está no estado original.</div>
        </div>
      )}
      <div className="stack" style={{ gap: 10 }}>
        {applied.map((t) => (
          <div key={t.id} className="tweak applied">
            <div>
              <h4>{t.title}</h4>
              <div className="meta"><RiskChip risk={t.risk} /><span className="chip mono">{t.id}</span></div>
            </div>
            <button className="btn" disabled={busy !== null} onClick={() => revert(t.id, t.title)}>
              {busy === t.id ? <Loader2 size={15} className="spin" /> : <RotateCcw size={15} />} Reverter
            </button>
          </div>
        ))}
      </div>
    </>
  );
}
