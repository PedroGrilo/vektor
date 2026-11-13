import { useState } from "react";
import { History, LifeBuoy, Loader2, RotateCcw } from "lucide-react";
import { backend } from "../mock";
import { PageHead, useLoad, useToast } from "../ui";

export default function Restore() {
  const toast = useToast();
  const changes = useLoad(() => backend.listChanges());
  const [busy, setBusy] = useState<string | null>(null);

  const revert = async (id: string, title: string) => {
    setBusy(id);
    try {
      await backend.revertChange(id);
      toast("ok", `${title} revertido`);
      await changes.reload();
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
      await changes.reload();
      setBusy(null);
    }
  };

  const list = changes.data ?? [];
  return (
    <>
      <PageHead eyebrow="// segurança" title="Restaurar"
        desc="Tudo o que o Vektor altera fica registado com o valor original. Reverte uma alteração ou volta ao estado inicial de uma vez."
        right={
          <button className="btn danger" disabled={busy !== null || list.length === 0} onClick={revertAll}>
            {busy === "*" ? <Loader2 size={15} className="spin" /> : <RotateCcw size={15} />} Reverter tudo
          </button>
        } />

      <RestorePointPanel />

      <div className="callout info" style={{ marginBottom: 18 }}>
        <History size={18} />
        <span>Os registos ficam em <span className="mono">%ProgramData%\Vektor\journal.json</span> e os backups de ficheiros em <span className="mono">backups\</span>. Sobrevivem a falhas e reinícios.</span>
      </div>

      {list.length === 0 && !changes.loading && (
        <div className="panel empty">
          <History size={28} color="var(--text-faint)" />
          <div>Nenhuma alteração ativa. O sistema está no estado original.</div>
        </div>
      )}
      <div className="stack" style={{ gap: 10 }}>
        {list.map((c) => (
          <div key={c.id} className="tweak applied">
            <div>
              <h4>{c.title}</h4>
              <div className="meta"><span className="chip mono">{c.id}</span></div>
            </div>
            <button className="btn" disabled={busy !== null} onClick={() => revert(c.id, c.title)}>
              {busy === c.id ? <Loader2 size={15} className="spin" /> : <RotateCcw size={15} />} Reverter
            </button>
          </div>
        ))}
      </div>
    </>
  );
}

function RestorePointPanel() {
  const toast = useToast();
  const last = useLoad(() => backend.restorePointLast());
  const [busy, setBusy] = useState(false);
  const [needsProtection, setNeedsProtection] = useState(false);

  const create = async (enableProtection: boolean) => {
    setBusy(true);
    try {
      const r = await backend.restorePointCreate(enableProtection);
      last.setData(r.last);
      setNeedsProtection(!r.created && r.message.includes("Proteção do Sistema"));
      toast(r.created ? "ok" : "err", r.message);
    } catch (e) {
      toast("err", String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="panel row between" style={{ marginBottom: 16, gap: 16 }}>
      <div className="row" style={{ gap: 14 }}>
        <LifeBuoy size={22} color="var(--accent)" />
        <div>
          <h3>Ponto de restauro do Windows</h3>
          <div className="sub">
            Rede de segurança extra: repõe o Windows inteiro mesmo que algo corra mal fora do Vektor.
            O Vektor cria um sozinho antes da primeira alteração de risco médio ou alto.
            {" "}Último: {last.loading ? "…" : last.data ? <b>{new Date(last.data.date).toLocaleString("pt-PT")} ({last.data.description})</b> : "nenhum"}
          </div>
        </div>
      </div>
      <div className="row" style={{ flexShrink: 0 }}>
        {needsProtection && (
          <button className="btn" disabled={busy} onClick={() => create(true)}>Ativar proteção e criar</button>
        )}
        <button className="btn primary" disabled={busy} onClick={() => create(false)}>
          {busy ? <Loader2 size={15} className="spin" /> : <LifeBuoy size={15} />} Criar agora
        </button>
      </div>
    </div>
  );
}
