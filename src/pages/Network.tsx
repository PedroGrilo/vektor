import { useState } from "react";
import { Activity, Check, Loader2, RotateCcw, Wifi } from "lucide-react";
import type { DnsResult, TweakInfo } from "../api";
import { backend } from "../mock";
import { PageHead, RiskChip, Toggle, useLoad, useToast } from "../ui";

export default function Network() {
  const toast = useToast();
  const status = useLoad(() => backend.dnsStatus());
  const tweaks = useLoad(() => backend.listTweaks());
  const [results, setResults] = useState<DnsResult[] | null>(null);
  const [testing, setTesting] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);

  const test = async () => {
    setTesting(true);
    try {
      setResults(await backend.dnsTest());
    } catch (e) {
      toast("err", String(e));
    } finally {
      setTesting(false);
    }
  };

  const apply = async (r: DnsResult) => {
    setBusy(r.id);
    try {
      status.setData(await backend.dnsApply(r.id));
      toast("ok", `DNS ${r.name} aplicado. Tem efeito total depois de religar a rede ou reiniciar.`);
    } catch (e) {
      toast("err", String(e));
    } finally {
      setBusy(null);
    }
  };

  const revert = async () => {
    setBusy("revert");
    try {
      status.setData(await backend.dnsRevert());
      toast("ok", "DNS original reposto");
    } catch (e) {
      toast("err", String(e));
    } finally {
      setBusy(null);
    }
  };

  const toggleTweak = async (t: TweakInfo, on: boolean) => {
    setBusy(t.id);
    try {
      const u = on ? await backend.applyTweak(t.id) : await backend.revertTweak(t.id);
      tweaks.setData((l) => (l ?? []).map((x) => (x.id === t.id ? u : x)));
      toast("ok", `${t.title}: ${on ? "aplicado" : "revertido"}${on && t.requiresReboot ? " (requer reinício)" : ""}`);
    } catch (e) {
      toast("err", `${t.title}: ${e}`);
    } finally {
      setBusy(null);
    }
  };

  const current = results?.find((r) => r.id === "current") ?? null;
  const best = results?.find((r) => r.latencyMs !== null && r.id !== "current") ?? null;
  // Only worth switching when clearly faster than what the PC uses now.
  const worth = best && (!current?.latencyMs || best.latencyMs! < current.latencyMs - 5);
  const maxMs = Math.max(1, ...(results ?? []).map((r) => r.latencyMs ?? 0));
  const netTweaks = (tweaks.data ?? []).filter((t) => t.category === "network");

  return (
    <>
      <PageHead eyebrow="// rede" title="Rede"
        desc="Testa qual DNS responde mais depressa a partir da tua casa e afina a placa de rede. O DNS não muda o ping dentro do jogo; acelera a ligação a servidores, lojas e lobbies." />

      <div className="panel" style={{ marginBottom: 16 }}>
        <div className="row between">
          <div>
            <h3 className="row"><Activity size={16} color="var(--accent)" /> Teste de DNS</h3>
            <div className="sub">
              DNS atual: <span className="mono">{status.data?.current.join(", ") || "—"}</span>
              {status.data?.applied && <> · aplicado pelo Vektor: <b>{status.data.applied}</b></>}
            </div>
          </div>
          <div className="row">
            {status.data?.applied && (
              <button className="btn" disabled={busy !== null} onClick={revert}>
                {busy === "revert" ? <Loader2 size={15} className="spin" /> : <RotateCcw size={15} />} Repor original
              </button>
            )}
            <button className="btn primary" disabled={testing} onClick={test}>
              {testing ? <Loader2 size={15} className="spin" /> : <Activity size={15} />} {results ? "Testar de novo" : "Testar"}
            </button>
          </div>
        </div>

        {results && (
          <div className="stack" style={{ gap: 8, marginTop: 14 }}>
            {results.map((r) => (
              <div key={r.id} className="row" style={{ gap: 12 }}>
                <div style={{ width: 150 }}>
                  <b>{r.name}</b>
                  <div className="mono faint" style={{ fontSize: 11 }}>{r.servers[0]}</div>
                </div>
                <div className="bar" style={{ flex: 1 }}><i style={{ width: `${((r.latencyMs ?? maxMs) / maxMs) * 100}%`, opacity: r.latencyMs === null ? 0.25 : 1 }} /></div>
                <div className="mono" style={{ width: 92, textAlign: "right" }}>
                  {r.latencyMs === null ? "sem resposta" : `${r.latencyMs.toFixed(1)} ms`}
                  {r.success < 1 && r.latencyMs !== null && <div className="faint" style={{ fontSize: 10.5 }}>{Math.round(r.success * 100)}% respostas</div>}
                </div>
                <div style={{ width: 110, textAlign: "right" }}>
                  {r.id === "current" ? <span className="chip">em uso</span>
                    : status.data?.applied === r.name ? <span className="chip on"><Check size={11} /> aplicado</span>
                    : <button className="btn ghost" disabled={busy !== null || r.latencyMs === null} onClick={() => apply(r)}>
                        {busy === r.id ? <Loader2 size={14} className="spin" /> : null} Usar
                      </button>}
                </div>
              </div>
            ))}
            <div className="sub" style={{ marginTop: 6 }}>
              {worth ? <>Recomendado: <b style={{ color: "var(--accent)" }}>{best!.name}</b>, mais rápido que o teu DNS atual.</>
                : "O teu DNS atual já está entre os mais rápidos. Não vale a pena mudar."}
              {" "}Aplica-se às ligações ativas (IPv4 e IPv6) e fica em Restaurar.
            </div>
          </div>
        )}
      </div>

      <div className="panel">
        <h3 className="row"><Wifi size={16} color="var(--accent)" /> Placa de rede</h3>
        <div className="sub" style={{ marginBottom: 12 }}>Em Wi-Fi o que mais ajuda é ficar perto do router ou usar cabo; estes ajustes evitam quebras de energia na placa.</div>
        <div className="stack" style={{ gap: 10 }}>
          {netTweaks.map((t) => (
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
              <Toggle label={t.title} on={t.applied} busy={busy === t.id} disabled={!t.available || (busy !== null && busy !== t.id)} onChange={(v) => toggleTweak(t, v)} />
            </div>
          ))}
        </div>
      </div>
    </>
  );
}
