import { useState } from "react";
import { ArrowUpCircle, CircleCheck, FileArchive, Info, Loader2, RefreshCw } from "lucide-react";
import { AUTHOR_URL, PROJECT_URL, openUrl } from "../api";
import type { AppSettings } from "../api";
import { backend } from "../mock";
import { PageHead, Toggle, useLoad, useToast } from "../ui";
import { checkForUpdate, installUpdate, useUpdate } from "../updates";

const ROWS: { key: keyof AppSettings; title: string; desc: string }[] = [
  { key: "autostart", title: "Arrancar com o Windows", desc: "Tarefa agendada com privilégios elevados — sem pedido de UAC no arranque." },
  { key: "startMinimized", title: "Arrancar minimizado", desc: "No arranque automático, fica só na bandeja." },
  { key: "closeToTray", title: "Fechar para a bandeja", desc: "Fechar a janela mantém o Vektor ativo — necessário para o modo jogo automático." },
  { key: "autoUpdate", title: "Procurar atualizações automaticamente", desc: "Verifica no arranque e de 6 em 6 horas se há uma versão nova no GitHub. Nada é instalado sem carregares em Atualizar." },
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

  const link = (url: string) => openUrl(url).catch((e) => toast("err", `Não foi possível abrir o link: ${e}`));

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
      <PageHead eyebrow="// definições" title="Definições" desc="Comportamento da aplicação, atualizações e suporte." />

      <Updates />

      <div className="stack" style={{ gap: 10, marginTop: 16 }}>
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
          <h3 className="row"><FileArchive size={16} color="var(--accent)" /> Exportar diagnóstico</h3>
          <div className="sub" style={{ margin: "6px 0 14px" }}>
            Cria um .zip no Ambiente de Trabalho com as especificações do PC, otimizações aplicadas e registos — para enviar ao suporte. Não inclui nomes nem IDs de contas Steam.
          </div>
          <button className="btn" disabled={busy !== null} onClick={exportDiag}>
            {busy === "diag" ? <Loader2 size={15} className="spin" /> : <FileArchive size={15} />} Exportar
          </button>
        </div>
        <div className="panel">
          <h3 className="row"><Info size={16} color="var(--accent)" /> Sobre</h3>
          <dl className="kv" style={{ marginTop: 10 }}>
            <dt>versão</dt><dd className="mono">{__APP_VERSION__}</dd>
            <dt>autor</dt><dd>By Kicas aka Pedro Grilo</dd>
            <dt>github</dt><dd><a onClick={() => link(AUTHOR_URL)}>github.com/PedroGrilo</a></dd>
            <dt>projeto</dt><dd><a onClick={() => link(PROJECT_URL)}>github.com/PedroGrilo/vektor</a></dd>
            <dt>dados</dt><dd className="mono">%ProgramData%\Vektor</dd>
            <dt>terceiros</dt><dd>NVIDIA Profile Inspector (MIT)</dd>
          </dl>
        </div>
      </div>
    </>
  );
}

function Updates() {
  const toast = useToast();
  const up = useUpdate();
  const busy = up.phase === "checking" || up.phase === "installing";

  const check = async () => {
    const r = await checkForUpdate();
    if (r.phase === "error") toast("err", `Não foi possível verificar atualizações: ${r.error}`);
    else if (r.phase === "up-to-date") toast("ok", "Já tens a versão mais recente.");
  };
  const install = () => installUpdate().catch((e) => toast("err", `A atualização falhou: ${e}`));

  const pct = up.progress === null ? null : Math.round(up.progress * 100);
  const status =
    up.phase === "installing" ? (pct === null ? "A transferir…" : pct >= 100 ? "A instalar — o Vektor reinicia sozinho." : `A transferir… ${pct}%`)
    : up.phase === "available" ? `Versão ${up.info?.version} disponível (tens a ${up.info?.currentVersion}).`
    : up.phase === "up-to-date" ? "Estás na versão mais recente."
    : up.phase === "checking" ? "A verificar…"
    : up.phase === "error" ? "Não foi possível verificar agora."
    : "Ainda não foi verificado nesta sessão.";

  return (
    <div className="panel corners">
      <div className="row between wrap" style={{ gap: 14 }}>
        <div className="row" style={{ alignItems: "flex-start" }}>
          {up.phase === "up-to-date" ? <CircleCheck size={20} color="var(--ok)" /> : <ArrowUpCircle size={20} color="var(--accent)" />}
          <div>
            <h3>Atualizações</h3>
            <div className="sub">
              {status}
              {up.checkedAt && up.phase !== "checking" && <> Última verificação às {new Date(up.checkedAt).toLocaleTimeString("pt-PT")}.</>}
            </div>
          </div>
        </div>
        <div className="row" style={{ gap: 8 }}>
          <button className="btn" disabled={busy} onClick={check}>
            {up.phase === "checking" ? <Loader2 size={15} className="spin" /> : <RefreshCw size={15} />} Verificar
          </button>
          {(up.phase === "available" || up.phase === "installing") && (
            <button className="btn primary" disabled={busy} onClick={install}>
              {up.phase === "installing" ? <Loader2 size={15} className="spin" /> : <ArrowUpCircle size={15} />} Atualizar e reiniciar
            </button>
          )}
        </div>
      </div>
      {up.phase === "installing" && <div className="bar" style={{ marginTop: 14 }}><i style={{ width: `${pct ?? 100}%` }} /></div>}
      {up.phase === "available" && up.info?.notes && (
        <pre className="code" style={{ marginTop: 14, whiteSpace: "pre-wrap", maxHeight: 180, overflowY: "auto", color: "var(--text-dim)" }}>{up.info.notes}</pre>
      )}
    </div>
  );
}
