# Vektor 1.1.0

Otimizador de PC Windows e Counter-Strike 2 — Tauri 2 (Rust) + React.

## Funcionalidades
- **Otimizações Windows** (todas reversíveis): plano de energia Vektor, Modo de Jogo, Game DVR off, HAGS,
  prioridades MMCSS, rede sem limitação, Win32PrioritySeparation, aceleração do rato off,
  CS2 com prioridade alta / GPU de alto desempenho / sem otimizações de ecrã inteiro, telemetria off.
- **Mais otimizações**: power throttling off, otimizações para jogos em janela, efeitos visuais/transparência, apps em segundo plano, MPO off (correção de flicker).
- **Windows 11 (1.1.0)**: atalhos de acessibilidade off (Shift 5×), Edge sem Startup Boost/segundo plano, Widgets off, Otimização da Entrega sem P2P.
  Exigem terminar sessão, por isso nunca entram no "Otimizar agora".
- **Limpeza** (análise antes de apagar): temporários, INetCache, WER, dumps, reciclagem, miniaturas, prefetch,
  caches de shaders (drivers e Steam), cache web da Steam, Windows Update, Otimização da Entrega, instaladores de drivers, DNS.
  Apps em segundo plano (lista fixa; nunca fecha Steam/CS2).
- **Memória (estilo ISLC)**: serviço `VektorMemory` (o próprio `vektor.exe --memory-service`, LocalSystem, sem IPC)
  que purga a lista de standby quando standby ≥ X MB e livre < Y MB. Configuração em `%ProgramData%\Vektor\memory.json`;
  o serviço escreve contadores, um heartbeat e o último erro em `memory-status.json`, e reinicia sozinho se falhar.
- **NVIDIA**: otimiza o perfil global com o NVIDIA Profile Inspector 3.0.2.1 (MIT, incluído no instalador via
  `scripts/fetch-npi.sh`, hash verificado). Snapshot do perfil antes de alterar; "Repor original" reimporta-o em modo replace.
- **CS2**: deteção de Steam/jogo/contas, presets de config (`vektor.cfg` + `autoexec.cfg` + `cs2_video.txt`),
  opções de arranque recomendadas, arranque e benchmark de FPS (mapa Workshop 3240880604).
- **Modo Jogo**: quando o `cs2.exe` arranca aplica (e reverte quando fecha): fechar/reabrir apps, afinidade de CPU
  (automática por grupo L3 — X3D/2 CCDs — ou manual), resolução durante o jogo (ex. 4:3), digital vibrance com foco (AMD).
- **NVIDIA CS2**: perfil dedicado ao `cs2.exe` (baixa latência, desempenho máximo, V-Sync off, texturas) além do global.
- **Benchmarks**: histórico com comparação à primeira medição.
- **Atualizações**: verificação automática e instalação num clique a partir dos releases do GitHub (instalador assinado, ver abaixo).
- **App**: ícone na bandeja, fechar para a bandeja, arranque com o Windows (tarefa agendada elevada), exportar diagnóstico (.zip sem dados de conta Steam).
- **Restaurar**: journal em `%ProgramData%\Vektor\journal.json` com o valor original de cada alteração e backups de ficheiros.

## Arquitetura
- Corre como administrador (manifesto `requireAdministrator`), sem serviço Windows permanente nem pipes.
- Tudo local: sem servidor, sem contas, sem scripts remotos.
- `src/api.ts` é o contrato entre UI e backend; `src-tauri/src/commands.rs` expõe os comandos.
- Módulos: `system`, `win/*` (registo, energia, serviços, ficheiros), `tweaks/*`, `cs2/*`, `journal`, `undo`.

## Desenvolvimento
```sh
bun install
./scripts/fetch-npi.sh   # NVIDIA Profile Inspector para src-tauri/resources/npi
bun run dev          # UI no browser com dados de demonstração (src/mock.ts)
bunx tauri dev       # app completa (Windows)
bunx tauri build     # instalador NSIS (Windows)
```
No macOS: `cd src-tauri && cargo test --lib && cargo clippy --target x86_64-pc-windows-msvc`
(precisa de `rustup target add x86_64-pc-windows-msvc` e `llvm-rc` do Homebrew `llvm` no PATH).

O workflow `.github/workflows/build-windows.yml` compila o instalador num runner Windows.

## Interface
- Paleta em `src/styles.css` (`--ink`, `--pine`, `--teal`, `--mint`, `--frost`); todas as outras cores derivam destas cinco.
- Vidro (blur) só em barras e botões; os painéis são opacos de propósito, para o scroll e os repaints ficarem baratos.
- Sem biblioteca de animação. O polling pára quando a janela está minimizada ou na bandeja (`src/visibility.ts`).

## Atualizações e releases
A app usa o updater do Tauri: lê `latest.json` do último release e só instala se a assinatura do instalador
corresponder à chave pública em `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`).

Para lançar uma versão:
1. Subir a versão em `package.json`, `src-tauri/Cargo.toml` e `src-tauri/tauri.conf.json` (o CI falha se não coincidirem com a tag).
2. Escrever `RELEASE_NOTES.md` (aparece no release e dentro da app).
3. `git tag vX.Y.Z && git push --tags` — o CI compila, assina, gera `latest.json` e publica o release.

Requisitos:
- Secret `TAURI_SIGNING_PRIVATE_KEY` no repositório (a chave privada gerada com `bunx tauri signer generate`;
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` só se a chave tiver password). **Perder a chave privada impede atualizar as instalações existentes.**
- Os ficheiros do release têm de ser descarregáveis sem autenticação: o repositório (ou um repositório só de releases,
  mudando `plugins.updater.endpoints` e o URL no workflow) tem de ser público.
- O serviço VektorMemory corre a partir do `vektor.exe`; o instalador pára-o antes de substituir o ficheiro e retoma-o depois (`src-tauri/windows/hooks.nsh`).

## Por validar num PC Windows real
- `powercfg /duplicatescheme` e parsing da saída localizada; serviços via SCM como administrador.
- VRAM quando `qwMemorySize` é REG_BINARY; HAGS não verifica suporte da GPU.
- CS2: `map_workshop` via `+exec`, formato `[VProf] FPS: Avg=…, P1=…`, valores de vídeo HDR/FSR, `cl_crosshair_t`, `cl_radar_scale`.
- Serviço VektorMemory: instalar/arrancar/parar via SCM, privilégio SeProfileSingleProcessPrivilege e purga reais;
  ações de recuperação (reinício após falha).
- NVIDIA: `-exportCustomized`/`-silentImport` com driver real e nome do perfil global ("Base Profile"). As flags e o formato
  `.nip` foram conferidos com o código do Profile Inspector 3.0.2.1; falta confirmar com um driver real que valores iguais
  ao predefinido ficam fora da exportação (a verificação aceita os dois casos).
- Atualizações: fluxo completo (verificar, transferir, instalador passivo, reinício) entre duas versões reais;
  paragem/retoma do serviço pelo instalador.
- Vidro (backdrop-filter) e pausa do polling na bandeja no WebView2.
- Limpeza: ficheiros bloqueados do Windows Update/Otimização da Entrega; Reciclagem com a app elevada.
- Modo jogo: SetProcessAffinityMask no cs2.exe, reabertura via explorer.exe sem elevação, heurística X3D.
- Vibrance AMD (ADL) e troca de resolução; schtasks; Ambiente de Trabalho redirecionado para OneDrive.
