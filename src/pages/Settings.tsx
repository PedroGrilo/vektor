import { useState } from "react";
import { FileArchive, Info, Loader2 } from "lucide-react";
import type { AppSettings } from "../api";
import { backend } from "../mock";
import { PageHead, Toggle, useLoad, useToast } from "../ui";

const ROWS: { key: keyof AppSettings; title: string; desc: string }[] = [
  { key: "autostart", title: "Arrancar com o Windows", desc: "Tarefa agendada com privilégios elevados — sem pedido de UAC no arranque." },
  { key: "startMinimized", title: "Arrancar minimizado", desc: "No arranque automático, fica só na bandeja." },
  { key: "closeToTray", title: "Fechar para a bandeja", desc: "Fechar a janela mantém o Vektor ativo — necessário para o modo jogo automático." },
];

export default function Settings() {
  const toast = useToast();
  const st = useLoad(() => backend.appSettings());
  const [busy, setBusy] = useState<string | null>(null);

  const set = async (key: keyof AppSettings, v: boolean) => {
    if (!st.data) return;
    setBusy(key);
    try {
      st.setData(await backend.appSetSettings({ ...st.data, [key]: v }));
    } catch (e) {
      toast("err", String(e));
    } finally {
      setBusy(null);
    }
  };

  const exportDiag = async () => {
    setBusy("diag");
    try {
      const path = await backend.exportDiagnostics();
      toast("ok", `Diagnóstico guardado em ${path}`);
    } catch (e) {
      toast("err", String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <>
      <PageHead eyebrow="// definições" title="Definições" desc="Comportamento da aplicação e suporte." />
      <div className="stack" style={{ gap: 10 }}>
        {ROWS.map((r) => (
          <div key={r.key} className={`tweak ${st.data?.[r.key] ? "applied" : ""}`}>
            <div>
              <h4>{r.title}</h4>
              <p>{r.desc}</p>
            </div>
            <Toggle label={r.title} on={!!st.data?.[r.key]} busy={busy === r.key} disabled={!st.data} onChange={(v) => set(r.key, v)} />
          </div>
        ))}
      </div>

      <div className="grid-2" style={{ marginTop: 16 }}>
        <div className="panel">
          <h3 className="row"><FileArchive size={16} color="var(--cyan)" /> Exportar diagnóstico</h3>
          <div className="sub" style={{ margin: "6px 0 14px" }}>
            Cria um .zip no Ambiente de Trabalho com as especificações do PC, otimizações aplicadas e registos — para enviar ao suporte. Não inclui nomes nem IDs de contas Steam.
          </div>
          <button className="btn" disabled={busy !== null} onClick={exportDiag}>
            {busy === "diag" ? <Loader2 size={15} className="spin" /> : <FileArchive size={15} />} Exportar
          </button>
        </div>
        <div className="panel">
          <h3 className="row"><Info size={16} color="var(--cyan)" /> Sobre</h3>
          <dl className="kv" style={{ marginTop: 10 }}>
            <dt>versão</dt><dd className="mono">1.0.0</dd>
            <dt>dados</dt><dd className="mono">%ProgramData%\Vektor</dd>
            <dt>terceiros</dt><dd>NVIDIA Profile Inspector (MIT)</dd>
          </dl>
        </div>
      </div>
    </>
  );
}
