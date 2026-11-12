## Vektor 1.1.0

### Novidades
- **Atualizações dentro da app**: o Vektor verifica no arranque (e de 6 em 6 horas) se há versão nova no GitHub e instala-a com um clique em Definições → Atualizações. O instalador é verificado por assinatura antes de correr.
- **Novo visual**: paleta verde-escura nova em toda a app, ícone novo e efeito de vidro nas barras e botões.
- **Memória**: página redesenhada com a composição da RAM (em uso, modificada, standby, livre) e o estado da regra em tempo real — mostra porque é que o serviço purgou ou não.
- **4 otimizações novas** para o Windows 11: atalhos de acessibilidade (Shift 5×) desligados, Edge sem execução em segundo plano, Widgets desligados e atualizações sem partilha P2P.

### Correções
- **VektorMemory**: o serviço deixava de purgar sem dar qualquer sinal quando uma leitura de memória ou a purga falhava. Agora ativa o privilégio necessário antes de ler, regista o erro, mostra-o na app e reinicia sozinho se falhar. O serviço também é parado e retomado durante as atualizações.
- **NVIDIA**: a opção "Filtragem de texturas: alto desempenho" desligava a otimização trilinear em vez de a manter ligada. A verificação depois de aplicar também dava erro quando o perfil tinha sido aplicado com sucesso; ambos corrigidos. "Baixa latência" passa a aparecer como ligada no Painel de Controlo NVIDIA.
- O Profile Inspector já não deixa a app presa se abrir uma janela de erro.

### Desempenho
- Navegação instantânea entre páginas e interface mais leve (sem animações contínuas nem desfoques em painéis).
- O Vektor deixa de consultar CPU/RAM quando está minimizado ou na bandeja, e cada leitura passou a ser muito mais leve — menos interferência durante o jogo.

### Notas
- Quem tem a 1.0.0 instala a 1.1.0 manualmente uma última vez; daí em diante as atualizações chegam pela app.
- O instalador não tem assinatura de código Windows: o SmartScreen mostra "Editor desconhecido".
- A app pede privilégios de administrador ao abrir.
