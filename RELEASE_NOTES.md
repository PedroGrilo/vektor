## Vektor 1.4.0

### Novidades
- **Vektor Lite agora instala-se** (`Vektor-Lite_1.4.0_x64-setup.exe`) e traz a **limpeza de memória sempre ligada**: um serviço do Windows (VektorLiteMemory) purga a memória em espera sempre que a memória livre fica baixa, mesmo com a app fechada.
- As regras do Lite são pensadas para PCs com pouca RAM: purga a partir de 512 MB em espera, quando a memória livre fica abaixo de um quarto da RAM, verificando a cada 3 segundos. Podem ser ajustadas na nova página Memória do Lite.

### Notas
- A versão portátil (`Vektor-Lite_1.4.0_portable.exe`) continua disponível; pode purgar a memória à mão, mas o serviço só existe na versão instalada.
- Desinstalar o Vektor Lite remove o serviço.
- A versão completa chega pela própria app: Definições → Atualizações → Atualizar e reiniciar.
