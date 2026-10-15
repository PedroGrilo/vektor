# Vektor

Otimizador de PC Windows e Counter-Strike 2 — Tauri 2 (Rust) + React.

## Funcionalidades
- **Otimizações Windows** (todas reversíveis): plano de energia Vektor, Modo de Jogo, Game DVR off, HAGS,
  prioridades MMCSS, rede sem limitação, Win32PrioritySeparation, aceleração do rato off,
  CS2 com prioridade alta / GPU de alto desempenho / sem otimizações de ecrã inteiro, telemetria off.
- **Limpeza**: caches de shaders (DirectX, drivers, CS2) e apps em segundo plano (lista fixa; nunca fecha Steam/CS2).
- **CS2**: deteção de Steam/jogo/contas, presets de config (`vektor.cfg` + `autoexec.cfg` + `cs2_video.txt`),
  opções de arranque recomendadas, arranque e benchmark de FPS (mapa Workshop 3240880604).
- **Restaurar**: journal em `%ProgramData%\Vektor\journal.json` com o valor original de cada alteração e backups de ficheiros.

## Arquitetura
- Corre como administrador (manifesto `requireAdministrator`), sem serviço Windows permanente nem pipes.
- Tudo local: sem servidor, sem contas, sem scripts remotos.
- `src/api.ts` é o contrato entre UI e backend; `src-tauri/src/commands.rs` expõe os comandos.
- Módulos: `system`, `win/*` (registo, energia, serviços, ficheiros), `tweaks/*`, `cs2/*`, `journal`, `undo`.
- `gpu/` e `ryzen/` são apenas esqueletos (devolvem "não suportado") e não aparecem na interface.

## Desenvolvimento
```sh
bun install
bun run dev          # UI no browser com dados de demonstração (src/mock.ts)
bunx tauri dev       # app completa (Windows)
bunx tauri build     # instalador NSIS (Windows)
```
No macOS: `cd src-tauri && cargo test --lib && cargo clippy --target x86_64-pc-windows-msvc`
(precisa de `rustup target add x86_64-pc-windows-msvc` e `llvm-rc` do Homebrew `llvm` no PATH).

O workflow `.github/workflows/build-windows.yml` compila o instalador num runner Windows.

## Por validar num PC Windows real
- `powercfg /duplicatescheme` e parsing da saída localizada; serviços via SCM como administrador.
- VRAM quando `qwMemorySize` é REG_BINARY; HAGS não verifica suporte da GPU.
- CS2: `map_workshop` via `+exec`, formato `[VProf] FPS: Avg=…, P1=…`, valores de vídeo HDR/FSR, `cl_crosshair_t`, `cl_radar_scale`.
