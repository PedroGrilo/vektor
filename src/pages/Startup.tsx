import { useState } from "react";
import { Loader2, Lock, Power, RefreshCw } from "lucide-react";
import type { StartupEntry } from "../api";
import { backend } from "../mock";
import { PageHead, Toggle, useLoad, useToast } from "../ui";

export default function Startup() {
  const toast = useToast();
  const list = useLoad(() => backend.startupList());
  const [busy, setBusy] = useState<string | null>(null);

  const flip = async (e: StartupEntry, on: boolean) => {
    setBusy(e.id);
    try {
      list.setData(await backend.startupSetEnabled(e.id, on));
      toast("ok", `${e.name}: ${on ? "volta a arrancar com o Windows" : "já não arranca com o Windows"}`);
    } catch (err) {
      toast("err", `${e.name}: ${err}`);
    } finally {
      setBusy(null);
    }
  };

  const entries = list.data ?? [];
  const on = entries.filter((e) => e.enabled).length;

  return (
    <>
      <PageHead eyebrow="// arranque" title="Arranque do Windows"
        desc="Programas que abrem sozinhos quando ligas o PC. Desligar não os desinstala: só deixam de abrir no arranque. É o mesmo interruptor do separador Arranque do Gestor de Tarefas."
        right={<button className="btn" onClick={() => list.reload()}><RefreshCw size={15} /> Atualizar</button>} />

      {list.data && (
        <div className="callout info" style={{ marginBottom: 16 }}>
          <Power size={18} />
          <span><b>{on}</b> de {entries.length} programas arrancam com o Windows. Cada mudança fica em Restaurar e pode ser desfeita.</span>
        </div>
      )}
      {list.error && <div className="callout">{list.error}</div>}
      {list.loading && !list.data && <div className="empty"><Loader2 className="spin" /></div>}

      <div className="stack" style={{ gap: 10 }}>
        {entries.map((e) => (
          <div key={e.id} className={`tweak ${e.enabled ? "" : "applied"}`}>
            <div style={{ minWidth: 0 }}>
              <h4>{e.name}</h4>
              <p className="mono" style={{ fontSize: 11.5, overflowWrap: "anywhere" }}>{e.command}</p>
              {e.hint && <p>{e.hint}</p>}
              <div className="meta">
                <span className="chip">{e.location}</span>
                {!e.enabled && <span className="chip on"><span className="dot" />desligado</span>}
                {e.changedByVektor && <span className="chip">alterado pelo Vektor</span>}
                {e.protected && <span className="chip"><Lock size={11} /> protegido</span>}
              </div>
            </div>
            <Toggle label={`${e.name} no arranque`} on={e.enabled} busy={busy === e.id}
              disabled={(e.protected && e.enabled) || (busy !== null && busy !== e.id)}
              onChange={(v) => flip(e, v)} />
          </div>
        ))}
      </div>

      <div className="sub" style={{ marginTop: 16 }}>
        O interruptor ligado quer dizer que o programa arranca com o Windows. Tarefas agendadas e serviços não aparecem aqui.
      </div>
    </>
  );
}
