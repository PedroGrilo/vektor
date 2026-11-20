import { useState } from "react";
import { Loader2, TerminalSquare, TriangleAlert } from "lucide-react";
import { backend } from "./mock";
import { useToast } from "./ui";

/** Card that opens Chris Titus Tech's WinUtil (`irm https://christitus.com/win | iex`) after one confirmation. */
export default function WinUtilCard() {
  const toast = useToast();
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);

  const open = async () => {
    setBusy(true);
    try {
      await backend.openWinUtil();
      toast("ok", "WinUtil a abrir numa janela do PowerShell.");
      setConfirming(false);
    } catch (e) {
      toast("err", `Não foi possível abrir o WinUtil: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="panel row" style={{ gap: 16, alignItems: "flex-start" }}>
      <TerminalSquare size={22} color="var(--accent)" />
      <div style={{ flex: 1 }}>
        <h3>Chris Titus WinUtil</h3>
        {confirming ? (
          <>
            <div className="sub" style={{ color: "var(--warn)" }}>
              <TriangleAlert size={13} style={{ verticalAlign: -2 }} /> Descarrega e corre o script de christitus.com como administrador.
              As alterações feitas lá não ficam em Restaurar.
            </div>
            <div className="row" style={{ gap: 8, marginTop: 12 }}>
              <button className="btn primary" disabled={busy} onClick={open}>
                {busy ? <Loader2 size={15} className="spin" /> : <TerminalSquare size={15} />} Abrir WinUtil
              </button>
              <button className="btn ghost" disabled={busy} onClick={() => setConfirming(false)}>Cancelar</button>
            </div>
          </>
        ) : (
          <div className="sub">Instalar programas, atualizações e afinações extra do Windows. Abre numa janela própria do PowerShell.</div>
        )}
      </div>
      {!confirming && <button className="btn" onClick={() => setConfirming(true)}>Abrir</button>}
    </div>
  );
}
