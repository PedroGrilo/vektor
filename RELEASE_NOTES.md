## Vektor 1.0.0

Otimizador de PC Windows e Counter-Strike 2. Tudo local, sem contas nem servidor; todas as alterações são reversíveis em **Restaurar**.

### Funcionalidades
- **Otimizações Windows** (18): plano de energia, Modo de Jogo, Game DVR, HAGS, MMCSS, rede, prioridades, rato, power throttling, jogos em janela, efeitos visuais, MPO, CS2 (prioridade, GPU, ecrã inteiro), telemetria.
- **Limpeza** com análise prévia: temporários, INetCache, WER, dumps, reciclagem, miniaturas, prefetch, caches de shaders (drivers e Steam), Windows Update, Otimização da Entrega, DNS.
- **Memória (estilo ISLC)**: serviço VektorMemory que purga a lista de standby.
- **NVIDIA**: perfil global e perfil dedicado ao CS2 via NVIDIA Profile Inspector (MIT), com cópia do perfil original.
- **Modo Jogo**: fecha/reabre apps, afinidade de CPU, resolução 4:3, vibrance (AMD) — só enquanto o CS2 corre.
- **CS2**: presets de config, opções de arranque, benchmark de FPS com histórico, aviso de atualização pendente.
- Bandeja, arranque com o Windows, exportar diagnóstico.

### Notas
- O instalador não está assinado: o Windows SmartScreen mostra "Editor desconhecido".
- A app pede privilégios de administrador ao abrir.
- Digital vibrance só em AMD nesta versão.
