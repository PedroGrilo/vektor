import { useMemo, useState } from "react";
import { Gamepad2, Loader2, Plus, RefreshCw, Rocket, RotateCcw, Search, Trash2 } from "lucide-react";
import type { GameInfo, GameSource, GameTweakKind } from "../api";
import { backend } from "../mock";
import { PageHead, Toggle, useLoad, useToast } from "../ui";

const SOURCE: Record<GameSource, string> = {
  steam: "Steam",
  epic: "Epic Games",
  gog: "GOG",
  ubisoft: "Ubisoft",
  riot: "Riot",
  manual: "Adicionado",
};

const TWEAKS: { kind: GameTweakKind; label: string; hint: string }[] = [
  { kind: "priority", label: "Prioridade de CPU alta", hint: "O Windows arranca o jogo sempre com prioridade Alta." },
  { kind: "gpu", label: "GPU de alto desempenho", hint: "Força a gráfica dedicada (portáteis com integrada + dedicada)." },
  { kind: "fso", label: "Sem otimizações de ecrã inteiro", hint: "Ecrã inteiro exclusivo. Em alguns PCs baixa a latência; noutros não muda nada." },
];

const fileName = (p: string) => p.split(/[\\/]/).pop() ?? p;

export default function Games() {
  const toast = useToast();
  const [refresh, setRefresh] = useState(false);
  const list = useLoad(() => backend.gamesList(refresh), [refresh]);
  const [q, setQ] = useState("");
  const [adding, setAdding] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);

  const replace = (g: GameInfo) => list.setData((l) => (l ?? []).map((x) => (x.id === g.id ? g : x)));
  const run = async (key: string, f: () => Promise<void>) => {
    setBusy(key);
    try {
      await f();
    } catch (e) {
      toast("err", String(e));
    } finally {
      setBusy(null);
    }
  };

  const games = useMemo(
    () => (list.data ?? []).filter((g) => g.name.toLowerCase().includes(q.trim().toLowerCase())),
    [list.data, q],
  );
  const optimized = (list.data ?? []).filter((g) => g.tweaks.priority && g.tweaks.gpu && g.tweaks.fso).length;

  return (
    <>
      <PageHead eyebrow="// biblioteca" title="Jogos"
        desc="Todos os jogos encontrados na Steam, Epic, GOG, Ubisoft e Riot, mais os que adicionares. Otimiza cada um e escolhe quais ativam o Modo Jogo."
        right={
          <div className="row">
            <button className="btn" disabled={list.loading} onClick={() => (refresh ? list.reload() : setRefresh(true))}>
              {list.loading ? <Loader2 size={15} className="spin" /> : <RefreshCw size={15} />} Procurar de novo
            </button>
            <button className="btn primary" onClick={() => setAdding((v) => !v)}><Plus size={15} /> Adicionar jogo</button>
          </div>
        } />

      {adding && <AddGame onAdded={(g) => { list.setData((l) => [...(l ?? []), g]); setAdding(false); toast("ok", `${g.name} adicionado`); }} />}

      <div className="row between" style={{ marginBottom: 14, gap: 12 }}>
        <div className="row" style={{ flex: 1, maxWidth: 360 }}>
          <Search size={16} color="var(--text-faint)" />
          <input className="input" placeholder="Procurar jogo" value={q} onChange={(e) => setQ(e.target.value)} />
        </div>
        {list.data && <span className="sub">{list.data.length} jogos · {optimized} otimizados</span>}
      </div>

      {list.error && <div className="callout">{list.error}</div>}
      {list.loading && !list.data && <div className="empty"><Loader2 className="spin" /><div className="sub">A procurar jogos instalados…</div></div>}
      {list.data?.length === 0 && (
        <div className="panel empty">
          <Gamepad2 size={28} color="var(--text-faint)" />
          <div>Nenhum jogo encontrado. Usa “Adicionar jogo” para escolher o executável.</div>
        </div>
      )}

      <div className="stack" style={{ gap: 12 }}>
        {games.map((g) => {
          const all = g.tweaks.priority && g.tweaks.gpu && g.tweaks.fso;
          const any = g.tweaks.priority || g.tweaks.gpu || g.tweaks.fso;
          return (
            <div key={g.id} className={`panel ${all ? "corners" : ""}`}>
              <div className="row between" style={{ alignItems: "flex-start", gap: 12 }}>
                <div style={{ minWidth: 0 }}>
                  <h3 className="row" style={{ gap: 8 }}>
                    {g.name}
                    <span className="chip">{SOURCE[g.source]}</span>
                    {g.running && <span className="chip on"><span className="dot" />a correr</span>}
                  </h3>
                  {g.exe ? (
                    g.candidates.length > 1 && !any ? (
                      <select className="input" style={{ marginTop: 8, maxWidth: 460 }} value={g.exe}
                        onChange={(e) => run(g.id, async () => replace(await backend.gamesSetExe(g.id, e.target.value)))}>
                        {g.candidates.map((c) => <option key={c} value={c}>{fileName(c)}</option>)}
                      </select>
                    ) : (
                      <div className="sub mono" style={{ marginTop: 6, fontSize: 12, overflowWrap: "anywhere" }} title={g.exe}>{fileName(g.exe)}</div>
                    )
                  ) : (
                    <div className="sub" style={{ marginTop: 6, color: "var(--warn)" }}>Executável não encontrado. Remove e adiciona-o à mão.</div>
                  )}
                </div>
                <div className="row" style={{ flexShrink: 0 }}>
                  <span className="label">modo jogo</span>
                  <Toggle label={`Modo Jogo para ${g.name}`} on={g.inSession} disabled={!g.exe || busy !== null}
                    onChange={(v) => run(g.id, async () => replace(await backend.gamesSetSession(g.id, v)))} />
                </div>
              </div>

              <div className="grid-3" style={{ marginTop: 14, gap: 10 }}>
                {TWEAKS.map((t) => (
                  <div key={t.kind} className={`tweak ${g.tweaks[t.kind] ? "applied" : ""}`} style={{ padding: "10px 12px" }}>
                    <div>
                      <h4 style={{ fontSize: 13.5 }}>{t.label}</h4>
                      <p style={{ fontSize: 12 }}>{t.hint}</p>
                    </div>
                    <Toggle label={`${t.label} em ${g.name}`} on={g.tweaks[t.kind]} busy={busy === `${g.id}:${t.kind}`}
                      disabled={!g.exe || (busy !== null && busy !== `${g.id}:${t.kind}`)}
                      onChange={(v) => run(`${g.id}:${t.kind}`, async () => replace(await backend.gamesSetTweak(g.id, t.kind, v)))} />
                  </div>
                ))}
              </div>

              <div className="row" style={{ justifyContent: "flex-end", gap: 8, marginTop: 12 }}>
                <button className="btn ghost" disabled={busy !== null || any}
                  title={any ? "Reverte as otimizações antes de remover" : undefined}
                  onClick={() => run(g.id, async () => { await backend.gamesRemove(g.id); list.setData((l) => (l ?? []).filter((x) => x.id !== g.id)); })}>
                  <Trash2 size={14} /> Remover da lista
                </button>
                {any && (
                  <button className="btn" disabled={busy !== null} onClick={() => run(g.id, async () => { replace(await backend.gamesOptimize(g.id, false)); toast("ok", `${g.name}: revertido`); })}>
                    <RotateCcw size={14} /> Reverter
                  </button>
                )}
                {!all && (
                  <button className="btn primary" disabled={!g.exe || busy !== null} onClick={() => run(g.id, async () => { replace(await backend.gamesOptimize(g.id, true)); toast("ok", `${g.name} otimizado`); })}>
                    {busy === g.id ? <Loader2 size={14} className="spin" /> : <Rocket size={14} />} Otimizar
                  </button>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {list.data && (
        <div className="sub" style={{ marginTop: 16 }}>
          Removeste um jogo por engano? <a onClick={() => run("unhide", async () => { await backend.gamesUnhideAll(); await list.reload(); })}>Mostrar os jogos escondidos</a>.
        </div>
      )}
    </>
  );
}

function AddGame({ onAdded }: { onAdded: (g: GameInfo) => void }) {
  const toast = useToast();
  const procs = useLoad(() => backend.gamesRunningProcesses());
  const [exe, setExe] = useState("");
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);

  const add = async (path: string, n: string | null) => {
    setBusy(true);
    try {
      onAdded(await backend.gamesAdd(path, n));
    } catch (e) {
      toast("err", String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="panel" style={{ marginBottom: 16 }}>
      <h3>Adicionar jogo</h3>
      <div className="sub" style={{ marginBottom: 12 }}>
        O mais fácil: abre o jogo e escolhe-o na lista. Também podes colar o caminho do <span className="mono">.exe</span>.
      </div>
      <div className="stack" style={{ gap: 4, marginBottom: 12 }}>
        {procs.loading && <Loader2 size={16} className="spin" />}
        {(procs.data ?? []).map((p) => (
          <div key={p.exe} className="row between" style={{ padding: "4px 2px" }}>
            <span className="row" style={{ minWidth: 0 }}>
              <b>{p.name}</b>
              <span className="mono faint" style={{ fontSize: 11.5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }} title={p.exe}>{p.exe}</span>
            </span>
            <button className="btn ghost" disabled={busy} onClick={() => add(p.exe, null)}><Plus size={14} /> Adicionar</button>
          </div>
        ))}
        {procs.data?.length === 0 && <div className="sub">Nenhum programa grande a correr agora.</div>}
      </div>
      <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
        <input className="input" style={{ flex: 2, minWidth: 240 }} placeholder="C:\Jogos\Jogo\jogo.exe" value={exe} onChange={(e) => setExe(e.target.value)} />
        <input className="input" style={{ flex: 1, minWidth: 140 }} placeholder="Nome (opcional)" value={name} onChange={(e) => setName(e.target.value)} />
        <button className="btn primary" disabled={!exe.trim() || busy} onClick={() => add(exe.trim(), name.trim() || null)}>
          {busy ? <Loader2 size={14} className="spin" /> : <Plus size={14} />} Adicionar
        </button>
      </div>
    </div>
  );
}
