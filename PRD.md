# PRD — BrowserRouter (br)

> **Status:** Rascunho para geração de produto via IA
> **Versão do documento:** 1.0
> **Última atualização:** 2026-06-10
> **Linguagem-alvo:** Rust
> **Plataformas-alvo:** Windows, macOS, Linux

---

## 1. Visão geral do produto

**BrowserRouter (nome de código `br`)** É um **direcionador de links e protocolos** ("link/protocol router" ou "browser picker/launcher"): um utilitário leve, nativo e multiplataforma que se registra como o manipulador padrão de `http`/`https` (e opcionalmente outros esquemas/URIs e tipos de arquivo), intercepta cada link clicado fora de um navegador (ex.: em um app de chat, e-mail, IDE, terminal) e decide — automaticamente via regras, ou perguntando ao usuário através de um seletor (*picker*) — **em qual navegador, perfil de navegador, janela anônima ou aplicativo** o link deve ser aberto.

### 1.1 Problema a resolver

Usuários avançados (desenvolvedores, profissionais que separam contas pessoais/trabalho, testadores de QA, pesquisadores) frequentemente precisam:

- Abrir links de trabalho em um perfil/navegador diferente dos pessoais.
- Garantir que links vindos de apps específicos (Slack, Teams, e-mail, IDE) abram em navegadores específicos.
- Limpar URLs de parâmetros de rastreamento antes de abrir.
- Testar comportamento em múltiplos navegadores rapidamente.
- Evitar que o sistema operacional sempre abra tudo no "navegador padrão" único.

Hoje essas necessidades são atendidas por ferramentas **fragmentadas por sistema operacional** (macOS: Choosy, OpenIn, Browserino, Browserosaurus — descontinuado; Windows: Hurl, Browser Tamer, BrowserSelect, BrowserBarrier; Linux: Junction) ou por uma solução comercial cross-platform fechada (Linklever). Não existe uma solução **open-source, nativa, leve e verdadeiramente multiplataforma** com um formato de configuração unificado.

### 1.2 Proposta de valor

1. **Multiplataforma de verdade**: mesma engine de regras, mesmo formato de configuração, comportamento consistente em Windows, macOS e Linux.
2. **Leve e nativo**: escrito em Rust, sem runtime pesado (sem Electron/Chromium embutido), baixo consumo de memória e CPU, inicialização em milissegundos.
3. **Privacidade e localidade**: 100% offline, sem telemetria, sem conta, configuração armazenada localmente (com sincronização opcional via arquivo).
4. **Poderoso, mas opcional**: funciona "out of the box" como um simples seletor de navegador, mas permite regras avançadas (regex, contexto de app de origem, modificadores de teclado, perfis, scripts).
5. **Aberto e extensível**: configuração legível (TOML), CLI, API local via socket/named pipe, possibilidade de extensões de navegador.

---

## 2. Objetivos e métricas de sucesso

| Objetivo | Métrica |
|---|---|
| Tempo de resposta do *picker* | < 100 ms entre o clique no link e a janela do picker aparecer |
| Footprint de memória em repouso (daemon) | < 30 MB RSS |
| Tamanho do binário/instalador | < 15 MB por plataforma (sem runtime externo) |
| Cobertura de plataformas no MVP | Windows 10/11, macOS 12+, Linux (X11 + Wayland, principais distros) |
| Tempo para registrar como handler padrão | Fluxo guiado < 3 cliques em cada SO |
| Confiabilidade das regras | 0 falsos positivos em testes de regressão (suite de regras) |
| Privacidade | Nenhuma chamada de rede feita pelo binário principal (auditável) |

---

## 3. Público-alvo / Personas

1. **Dev/Power user multi-perfil** — separa Chrome "Trabalho" (Workspace Google) de Chrome "Pessoal", e quer que links do Gmail trabalho abram sempre no perfil correto.
2. **QA/Tester** — precisa abrir a mesma URL rapidamente em Firefox, Edge, Chrome e Safari para checar compatibilidade.
3. **Usuário focado em privacidade** — quer que links de redes sociais sempre abram em uma instância isolada/anônima ou em um navegador secundário, e quer remover parâmetros de rastreamento (`utm_*`, `fbclid`, `gclid`, etc.).
4. **Usuário corporativo** — a empresa exige Edge para sites internos (SSO/Conditional Access) mas o usuário prefere outro navegador para uso pessoal.
5. **Usuário casual** — só quer um pequeno menu de seleção de navegador ao clicar em links, sem configurar nada.

---

## 4. Análise SWOT

### Forças (Strengths)
- **Rust**: performance, segurança de memória, binário único estático, baixo consumo de recursos, fácil distribuição (sem dependências de runtime como .NET/JVM/Electron).
- **Cobertura cross-platform desde o dia 1**: a maioria dos concorrentes diretos é exclusiva de um único SO (Browserosaurus/Browserino/Choosy/OpenIn = macOS; Hurl/Browser Tamer/BrowserSelect/BrowserBarrier = Windows; Junction = Linux).
- **Open source**: gera confiança, auditabilidade (importante para uma ferramenta que intercepta todos os links do usuário) e permite contribuições da comunidade.
- **Configuração unificada e portátil**: um único arquivo TOML/JSON pode ser versionado/sincronizado entre máquinas e SOs.
- **Sem telemetria / 100% local**: diferencial forte de privacidade frente a soluções comerciais.

### Fraquezas (Weaknesses)
- **Ecossistema de GUI em Rust ainda imaturo** comparado a Electron/.NET/SwiftUI: menos widgets nativos prontos, possíveis lacunas de acessibilidade, fontes/HiDPI exigem atenção manual.
- **Registro como handler padrão é altamente específico de cada SO** e, em alguns casos (macOS App Sandbox, Windows 11 Default Apps), exige fluxos de UX guiados pelo usuário (não 100% automatizável) — alto custo de engenharia por plataforma.
- **Equipe pequena (provavelmente solo/poucos devs)** vs. necessidade de manter qualidade em 3 SOs + atualizações frequentes desses SOs.
- **Ausência de marca/reputação** frente a ferramentas estabelecidas (Choosy desde 2007, Browser Tamer com base de usuários ativa).

### Oportunidades (Opportunities)
- **Vácuo no macOS**: Browserosaurus foi arquivado (ago/2025), deixando espaço para uma alternativa open-source.
- **Nenhuma solução open-source cross-platform dominante** — Linklever é cross-platform mas comercial/fechada.
- **Tendência de multi-perfil de navegador** (trabalho remoto, Google Workspace/Microsoft 365 múltiplas contas) aumenta a demanda.
- **Funcionalidades de "URL hygiene"** (remoção de tracking, upgrade HTTPS) — vistas como diferencial em Linklever — são triviais de implementar e muito valorizadas.
- **Possibilidade de ecossistema**: extensões de navegador, integração com gerenciadores de automação (Raycast, Alfred, Rofi, PowerToys Run), API CLI para scripts/automação.
- **Comunidade Rust e "single static binary"**: facilita distribuição via gerenciadores de pacote (winget, Homebrew, AUR/Flatpak/apt).

### Ameaças (Threats)
- **Restrições de plataforma**: Apple aumenta sandboxing (apps de App Store exigem "Helper" não-sandboxed para registrar handler — caso do OpenIn); Microsoft dificulta troca de navegador padrão a cada versão do Windows (exige interação manual do usuário em Configurações).
- **Falsos positivos de antivírus/SmartScreen** para binários não assinados que alteram handlers de protocolo — assinatura de código tem custo (especialmente notarização Apple).
- **Concorrência pode reagir rapidamente**: Linklever já é cross-platform e comercial; soluções gratuitas por SO continuam ativas (Browser Tamer, Junction).
- **Mudanças de API do SO** (Windows Default Apps, macOS LaunchServices, portais XDG no Linux/Wayland) podem quebrar a integração e exigir manutenção contínua.
- **Fadiga de manutenção**: suportar Wayland + X11 + múltiplos desktop environments no Linux é historicamente uma fonte grande de bugs.

### Tabela comparativa de concorrentes

| Produto | SO | Open Source | Regras | Perfis de navegador | Filtros de URL (tracking) | Scripts/Plugins | Extensão de navegador |
|---|---|---|---|---|---|---|---|
| Browserosaurus (arquivado) | macOS | Sim | Não | Não | Não | Não | Não |
| Browserino | macOS | Sim | Básico | Não | Não | Não | Não |
| Choosy | macOS | Não ($10) | Sim | Sim | Não | Não | Sim |
| OpenIn 4 | macOS | Não | Sim (avançado) | Sim | Regex rewrite | zsh/JS | Sim |
| Hurl | Windows | Sim | Sim | Parcial | Não | Não | Experimental |
| Browser Tamer (bt) | Windows | Sim | Sim | Sim | Não | Sim (scripting) | Não |
| BrowserSelect | Windows | Sim (?) | Sim (URL pattern) | Sim (Chrome) | Não | Não | Não |
| BrowserBarrier | Windows | Sim | Não | Não | Não | Não | Não |
| Junction | Linux | Sim | Básico | Não | Não | Scripts via .desktop | Bookmarklet |
| Linklever | Win/macOS/Linux | Não (comercial) | Sim | Sim | **Sim** | Não | Sim |
| **BrowserRouter (br)** | **Win/macOS/Linux** | **Sim** | **Sim (avançado)** | **Sim** | **Sim** | **Sim** | **Sim (futuro)** |

---

## 5. Escopo de funcionalidades

### 5.1 MVP (v0.1 – v1.0)

1. **Registro como handler padrão de `http`/`https`** com fluxo de onboarding guiado por SO.
2. **Detecção automática de navegadores instalados** (Chrome, Firefox, Edge, Brave, Vivaldi, Opera, Safari [macOS], Arc, navegadores baseados em Chromium/Firefox genéricos).
3. **Detecção automática de perfis de navegador** (perfis do Chrome/Edge/Brave via `Local State`/`Preferences`; perfis do Firefox via `profiles.ini`).
4. **Picker (seletor) visual**: janela pequena, sempre no topo, mostrando ícones + nomes dos navegadores/perfis disponíveis, com:
   - Navegação por teclado (setas, números, primeira letra).
   - Atalhos com modificadores (Shift = janela anônima/privada, Ctrl/Cmd = abrir em segundo plano, etc.).
   - Botão/atalho "Sempre abrir `dominio.com` neste app" para criar regra rapidamente.
   - Timeout configurável (auto-fechar/usar padrão após N segundos) ou nunca.
   - Opção "fechar e cancelar" (Esc).
5. **Engine de regras** (avaliação ordenada por prioridade):
   - Match por padrão de URL (glob/regex): domínio, path, query string.
   - Match por aplicativo de origem (processo que originou a chamada, quando detectável pelo SO).
   - Match por modificador de teclado pressionado no momento do clique (quando suportado).
   - Ação: abrir em navegador/perfil específico, em modo anônimo/privado, ou exibir o picker.
   - Regra "catch-all" / fallback configurável.
6. **Filtros de URL** (aplicados antes do roteamento):
   - Remoção de parâmetros de rastreamento (lista padrão configurável: `utm_*`, `gclid`, `fbclid`, `mc_eid`, `igshid`, etc.).
   - Upgrade automático `http://` → `https://` (com lista de exceções).
   - Reescrita de URL via regex (capturas e substituições).
7. **Aplicativo de configurações (Settings UI)**:
   - Lista de navegadores/perfis detectados + adicionar manualmente (caminho do executável, argumentos, ícone).
   - Editor de regras (lista ordenável, drag-and-drop de prioridade, formulário + modo "avançado" editando TOML diretamente).
   - Editor de filtros.
   - Configurações gerais (comportamento padrão, tema claro/escuro/sistema, idioma).
   - Tela de status: "BrowserRouter está definido como navegador padrão? [Sim/Não] [Configurar]".
8. **Persistência de configuração**: arquivo único `config.toml` em diretório padrão de configuração do usuário (`%APPDATA%`, `~/Library/Application Support`, `~/.config`).
9. **CLI**:
   - `br open <url>` — roteia uma URL manualmente (para uso em scripts/automação).
   - `br doctor` — diagnostica status de registro como handler padrão, navegadores detectados, problemas de configuração.
   - `br config validate|export|import`.
   - `br rules list|add|rm|test <url>` (testar qual regra seria aplicada, sem abrir nada).
10. **Logs locais** (rotativos, opcionais, nível configurável) para depuração — nunca enviados externamente.
11. **Auto-start no login do sistema** (opcional, configurável).
12. **Atualização**: verificação manual de nova versão (sem auto-update obrigatório no MVP) com link para download; estrutura preparada para auto-update futuro (ex.: via crate `self_update` ou integração com gerenciadores de pacote).

### 5.2 v1.x (pós-MVP, "feature parity com líderes de mercado")

13. **Suporte a contexto de aplicativo de origem aprofundado**: detectar nome/processo do app que originou o clique (ex.: Slack, Outlook, VS Code) para regras como "links vindos do Slack → Chrome Work".
14. **Modo "abrir em múltiplos navegadores simultaneamente"** (ex.: Ctrl+clique abre em todos os navegadores selecionados — útil para QA), inspirado no Junction.
15. **Suporte a outros esquemas de URI**: `mailto:`, `tel:`, `ftp:`, esquemas customizados (`x-br://`) para integração com automação (Raycast, Alfred, Rofi, atalhos do SO).
16. **Suporte a tipos de arquivo / "abrir com"** (opcional, fora do escopo central, mas valioso): permitir regras para extensões específicas (ex.: `.pdf` → leitor específico).
17. **"Focus modes" / Perfis de configuração** (workspace "Trabalho" vs "Pessoal") que trocam o conjunto de regras ativo, possivelmente vinculados a:
    - Horário do dia.
    - Rede Wi-Fi conectada.
    - Modo de foco do SO (macOS Focus, Windows Focus Assist).
18. **Scripting de regras**: permitir que uma regra execute um script externo (shell/PowerShell/zsh) ou expressão simples para decidir o destino dinamicamente.
19. **Importar/exportar regras e configuração** em JSON/TOML, com versionamento e migração de schema.
20. **Sincronização de configuração** entre dispositivos via arquivo (Syncthing/Dropbox/iCloud Drive/OneDrive — apenas apontando o caminho do config para uma pasta sincronizada; sem backend próprio).
21. **API local (named pipe / unix socket / HTTP local em loopback)** para automações de terceiros consultarem/alterarem regras e disparar roteamento.
22. **Extensão de navegador (Chrome/Firefox/Edge/Safari)**:
    - Enviar a aba atual para outro navegador/perfil com um clique.
    - Bookmarklet/menu de contexto "Abrir com BrowserRouter".
23. **Tema visual customizável** (cores, tamanho do picker, posição na tela, modo compacto vs. detalhado).
24. **Internacionalização (i18n)**: PT-BR, EN, ES no mínimo.

### 5.3 v2+ (visão de longo prazo)

25. **Plugin system** (ex.: WASM) para extensões de terceiros (novas fontes de regra, integrações).
26. **Estatísticas locais opcionais** (quantos links foram roteados, para qual navegador, sem sair da máquina) — dashboard simples.
27. **Suporte a "Handoff"-like**: continuar navegação entre dispositivos (avançado, baixa prioridade).
28. **Modo "kiosk"/managed**: políticas configuráveis por administrador de TI (deploy corporativo via MDM/GPO), com configuração somente leitura.
29. **Acessibilidade avançada**: suporte completo a leitores de tela (NVDA, VoiceOver, Orca), navegação 100% por teclado, alto contraste.
30. **Empacotamento e distribuição automatizada**: Homebrew Cask, winget, Scoop, AUR, Flatpak (Flathub), .deb/.rpm, com pipelines de release automatizados (CI cross-compile + assinatura).

---

## 6. Requisitos funcionais detalhados

### 6.1 Interceptação e roteamento de links

- **RF-01**: O sistema DEVE se registrar como manipulador padrão dos esquemas `http` e `https` no SO.
- **RF-02**: Ao receber uma URL, o sistema DEVE:
  1. Aplicar **filtros** (limpeza de tracking params, upgrade HTTPS, regex rewrite), na ordem configurada.
  2. Avaliar **regras** em ordem de prioridade (maior prioridade primeiro); a primeira regra cuja condição (`match`) seja satisfeita determina a ação.
  3. Se nenhuma regra corresponder (ou a regra correspondente especificar `action = "ask"`), exibir o **picker**.
  4. Executar a ação: lançar o navegador/perfil/app de destino com a URL (processada pelos filtros), aplicando flags como modo anônimo/privado quando solicitado.
- **RF-03**: O picker DEVE aparecer em **menos de 100ms** após o clique, na tela onde está o cursor/foco, e DEVE ficar sempre no topo (always-on-top), sem roubar foco de forma agressiva além do necessário.
- **RF-04**: O usuário DEVE poder, a partir do picker, criar uma regra "sempre abrir este domínio/padrão neste destino" com 1 clique/atalho.
- **RF-05**: O sistema DEVE suportar abrir o mesmo link em **múltiplos destinos simultaneamente** (ação "duplicar"), configurável por modificador de teclado ou botão no picker.

### 6.2 Detecção de navegadores e perfis

- **RF-06**: O sistema DEVE detectar automaticamente navegadores instalados em caminhos padrão de cada SO (registro do Windows / `Applications` no macOS via LaunchServices / `.desktop` files e `$PATH` no Linux).
- **RF-07**: Para navegadores baseados em Chromium (Chrome, Edge, Brave, Vivaldi, Opera, Arc), o sistema DEVE ler o arquivo `Local State` para listar perfis (nome + diretório do perfil) e permitir abrir uma URL diretamente em um perfil específico via `--profile-directory=<dir>`.
- **RF-08**: Para Firefox (e derivados como LibreWolf, Zen), o sistema DEVE ler `profiles.ini` para listar perfis e usar `-P <nome>` ou `--profile <caminho>`.
- **RF-09**: O sistema DEVE permitir adicionar manualmente aplicativos/navegadores não detectados automaticamente (caminho do executável, argumentos extras, ícone, nome de exibição).
- **RF-10**: O sistema DEVE permitir ocultar navegadores/perfis detectados da lista do picker sem desinstalá-los.

### 6.3 Engine de regras

- **RF-11**: Cada regra possui: `id`, `name`, `enabled`, `priority` (inteiro, maior = avaliado primeiro), `match` (condições) e `action`.
- **RF-12**: Condições de `match` suportadas (combináveis com AND implícito; suporte a OR via lista de padrões):
  - `url_pattern`: glob (`*.exemplo.com/*`) e/ou regex (prefixo `regex:`).
  - `host`, `path`, `query_param` (presença/valor de parâmetro específico).
  - `source_app`: nome do processo/app de origem (quando disponível pelo SO).
  - `modifier_keys`: combinação de teclas pressionadas no momento da ação (Shift, Ctrl/Cmd, Alt).
  - `time_range` / `weekday` (para perfis "Trabalho" vs "Pessoal" por horário) — v1.x.
- **RF-13**: Ações de `action` suportadas:
  - `open_with`: id do navegador/perfil de destino.
  - `open_with_all`: lista de destinos (abrir em todos).
  - `private`: booleano — abrir em janela anônima/privada.
  - `ask`: forçar exibição do picker mesmo que outras regras combinassem.
  - `block`: opcional — não abrir nada (ex.: bloquear domínios específicos).
- **RF-14**: O sistema DEVE fornecer um comando `br rules test <url> [--source-app NOME]` que mostra **qual regra seria aplicada e por quê**, sem efetivamente abrir nada (dry-run), essencial para depuração.
- **RF-15**: A ordem de prioridade DEVE ser editável via drag-and-drop na UI e via edição direta do TOML.

### 6.4 Filtros de URL

- **RF-16**: Lista de parâmetros de tracking removíveis DEVE vir com um conjunto padrão (atualizável) e permitir customização (adicionar/remover padrões, incluindo wildcards `utm_*`).
- **RF-17**: Upgrade HTTP→HTTPS DEVE ser configurável globalmente, com lista de domínios em exceção (ex.: hosts locais, `*.local`, IPs privados).
- **RF-18**: Reescrita via regex DEVE suportar grupos de captura nomeados e templates de substituição (`${1}`, `${nome}`).
- **RF-19**: Filtros DEVEM ser aplicados **antes** da engine de regras, para que as regras operem sobre a URL já normalizada.

### 6.5 Configuração

- **RF-20**: Toda a configuração (navegadores, perfis, regras, filtros, preferências gerais) DEVE residir em um único arquivo legível por humanos (TOML), com schema versionado (`config_version`).
- **RF-21**: O sistema DEVE validar o arquivo de configuração ao carregar e, em caso de erro, **não travar**: deve operar com a última configuração válida conhecida (ou um fallback seguro: sempre mostrar o picker) e notificar o usuário do erro.
- **RF-22**: Importar/exportar configuração completa ou parcial (somente regras, somente filtros).
- **RF-23**: Hot-reload: alterações no arquivo de configuração (feitas manualmente ou pela UI) DEVEM ser detectadas e aplicadas sem reiniciar o processo principal.

### 6.6 Aplicativo de configurações (UI)

- **RF-24**: Tela inicial/Status: mostra se `br` é o handler padrão de `http`/`https` no SO atual, com botão de ação para abrir as configurações nativas do SO (ex.: `ms-settings:defaultapps` no Windows, painel de "Navegador padrão" no macOS, `xdg-settings` no Linux) e instruções passo a passo (com capturas de tela/ilustrações).
- **RF-25**: Tela de Navegadores/Perfis: lista, adicionar, editar, remover, reordenar, ocultar.
- **RF-26**: Tela de Regras: lista ordenável, criação/edição via formulário guiado e via editor de texto (TOML) com validação em tempo real.
- **RF-27**: Tela de Filtros: gerenciamento de listas de tracking params, upgrade HTTPS, regras de rewrite.
- **RF-28**: Tela de Preferências Gerais: tema (claro/escuro/sistema), idioma, comportamento padrão (perguntar sempre / usar regra / abrir direto), timeout do picker, iniciar com o sistema, posição do picker na tela.
- **RF-29**: Tela "Sobre": versão, licença, link do repositório, changelog.
- **RF-30**: A UI DEVE ser responsiva a DPI (suporte a telas HiDPI/Retina) e suportar redimensionamento.

### 6.7 CLI

- **RF-31**: `br open <url> [--app <id>] [--private]` — abre uma URL respeitando (ou ignorando, via flags) as regras.
- **RF-32**: `br doctor` — relatório de diagnóstico (handler padrão, navegadores detectados, caminho da config, erros de validação, versão do SO).
- **RF-33**: `br config show|validate|export <arquivo>|import <arquivo>`.
- **RF-34**: `br rules list|add|rm|enable|disable|test`.
- **RF-35**: `br register` / `br unregister` — tenta registrar/des-registrar como handler padrão (com instruções manuais quando o SO não permitir automação total).
- **RF-36**: Saída da CLI DEVE suportar formato humano e `--json` para uso em scripts.

### 6.8 Integração por sistema operacional

- **RF-37 (Windows)**:
  - Registrar `br` em `HKEY_CURRENT_USER\Software\Classes` + `RegisteredApplications` / `Capabilities` conforme o esquema de Default Programs do Windows.
  - Como Windows 10/11 exige confirmação manual do usuário em **Configurações > Aplicativos > Aplicativos padrão**, o app DEVE abrir essa tela diretamente (`ms-settings:defaultapps?registeredAppUser=...` ou equivalente) e exibir instruções visuais.
  - Detectar navegadores via `HKEY_LOCAL_MACHINE\SOFTWARE\Clients\StartMenuInternet` e caminhos comuns.
- **RF-38 (macOS)**:
  - Declarar `CFBundleURLTypes` no `Info.plist` para `http`/`https` e usar `LSSetDefaultHandlerForURLScheme` (via `Launch Services`), respeitando que o usuário precisa confirmar (prompt do sistema) ou ajustar manualmente em **Ajustes do Sistema > Apps Padrão**.
  - Detectar navegadores instalados via `/Applications` e `~/Applications`, lendo `Info.plist` de cada `.app` para nome/ícone/bundle id.
  - Lidar com **assinatura de código e notarização** (Apple Developer ID) como requisito de distribuição fora da App Store.
- **RF-39 (Linux)**:
  - Fornecer um arquivo `.desktop` (`br.desktop`) declarando `MimeType=x-scheme-handler/http;x-scheme-handler/https;` e usar `xdg-mime default br.desktop x-scheme-handler/http x-scheme-handler/https` (e `xdg-settings set default-web-browser`).
  - Detectar navegadores via `.desktop` files em `/usr/share/applications`, `~/.local/share/applications` e `update-alternatives` (onde aplicável).
  - Suportar tanto **X11** quanto **Wayland** para a janela do picker (always-on-top, posicionamento), considerando limitações de Wayland (ex.: necessidade de protocolos `wlr-layer-shell` em compositores compatíveis, fallback gracioso quando não suportado).

### 6.9 Logs e diagnóstico

- **RF-40**: Logs locais em arquivo rotativo (ex.: `~/.local/share/br/logs/`), níveis configuráveis (error/warn/info/debug/trace), nunca habilitados em "debug" por padrão.
- **RF-41**: `br doctor` deve incluir: versão do `br`, SO/versão, status de handler padrão, lista de navegadores/perfis detectados, caminho e validade da configuração, últimas N entradas de log relevantes.

---

## 7. Requisitos não funcionais

| Categoria | Requisito |
|---|---|
| **Performance** | Picker deve renderizar em < 100ms (cold) e < 30ms (warm/daemon já rodando). Processo em repouso < 30MB RAM, ~0% CPU. |
| **Tamanho** | Binário principal < 10MB; instalador completo < 15MB por plataforma, sem dependências externas obrigatórias (sem .NET/Java/Node). |
| **Privacidade** | Nenhuma chamada de rede no binário principal por padrão. Verificação de atualização (se habilitada) deve ser opt-in e claramente documentada. |
| **Segurança** | Validação rigorosa de URLs e argumentos passados a processos externos (prevenção de injeção de comando — nunca usar shell para concatenar URLs; usar `std::process::Command` com argumentos separados). Scripts custom (RF-18, item 18 do escopo) executados de forma explícita e com aviso de segurança. |
| **Confiabilidade** | Se o `br` travar ou a configuração estiver corrompida, o comportamento padrão deve ser **mostrar o picker com a lista padrão de navegadores** (fail-safe), nunca falhar silenciosamente em abrir nada. |
| **Compatibilidade** | Windows 10 (1903+) e 11; macOS 12 (Monterey)+; principais distros Linux com glibc recente (Ubuntu 22.04+, Fedora, Arch) — X11 e Wayland. |
| **Acessibilidade** | Navegação completa por teclado no picker e na UI de configurações; contraste adequado (WCAG AA); compatível com leitores de tela nativos quando o framework de UI permitir. |
| **Internacionalização** | Arquitetura de strings preparada para i18n desde o início (mesmo que MVP só tenha PT-BR/EN). |
| **Observabilidade local** | Logs estruturados (ex.: `tracing` crate) com rotação e nível configurável. |
| **Manutenibilidade** | Código organizado em crates separados (core, platform, ui, cli) — ver Arquitetura. Cobertura de testes automatizados para a engine de regras e filtros (unitários) e testes de integração por plataforma (quando viável em CI). |
| **Distribuição** | Builds automatizados via CI (GitHub Actions) para os 3 SOs; assinatura de código (Windows Authenticode, macOS Developer ID + notarização) antes do release estável. |

---

## 8. Arquitetura técnica proposta

### 8.1 Visão geral de módulos (workspace Cargo)

```
br/
├── Cargo.toml                  # workspace
├── crates/
│   ├── br-core/                # engine de regras, filtros, modelos de dados, config (sem deps de UI/SO)
│   ├── br-platform/             # abstrações + implementações por SO (windows/, macos/, linux/)
│   │   ├── windows.rs          # registro de protocolo, detecção de navegadores, registry
│   │   ├── macos.rs             # LaunchServices, Info.plist, /Applications scan
│   │   └── linux.rs             # xdg-mime, .desktop parsing, X11/Wayland
│   ├── br-ui-picker/            # janela leve do seletor (overlay always-on-top)
│   ├── br-ui-settings/          # aplicativo de configurações (janela principal)
│   ├── br-cli/                  # binário `br` (CLI + ponto de entrada)
│   └── br-daemon/               # processo de fundo opcional (hot-reload, IPC, autostart)
└── PRD.md
```

- **br-core**: tipos `Rule`, `Filter`, `BrowserTarget`, `Config`; função pura `route(url, context, &Config) -> RoutingDecision`. 100% testável sem SO.
- **br-platform**: trait `PlatformIntegration` com métodos `register_as_default_handler()`, `is_default_handler() -> bool`, `discover_browsers() -> Vec<BrowserTarget>`, `launch(target, url, opts)`, `get_foreground_app_name() -> Option<String>`. Implementações concretas por `cfg(target_os)`.
- **br-ui-picker**: janela minimalista, sempre no topo, renderização imediata, foco em latência baixíssima.
- **br-ui-settings**: aplicativo completo de configurações (pode compartilhar componentes visuais com o picker).
- **br-cli**: parsing de argumentos (`clap`), comandos `open/doctor/config/rules/register`.
- **br-daemon**: opcional — processo residente para reduzir latência (evita custo de cold start a cada link), hot-reload de config, IPC local.

### 8.2 Escolha de biblioteca de UI multiplataforma

Critérios: leveza, performance, suporte nativo a Windows/macOS/Linux (X11+Wayland), sem necessidade de runtime web pesado, boa renderização de janelas "overlay always-on-top" para o picker.

| Framework | Tipo | Tamanho/peso | Always-on-top/overlay | Maturidade | Observações |
|---|---|---|---|---|---|
| **egui** (`eframe`) | Immediate mode, GPU (wgpu/glow) | Muito leve | Sim, fácil | Alta | Ótimo para o picker (latência baixa, redraw simples). Visual menos "nativo" por padrão, mas customizável. |
| **Slint** | Declarativo, compilado, GPU/software | Leve | Sim | Média/Alta | Visual mais polido/nativo, boa para UI de configurações; licença a verificar (GPL/comercial para certos usos). |
| **iced** | Elm-architecture, GPU (wgpu) | Leve/médio | Possível, com mais esforço | Média | Boa ergonomia, comunidade ativa. |
| **Tauri** | WebView do SO + Rust backend | Médio (depende do WebView) | Possível | Alta | Visual fácil (HTML/CSS), mas adiciona dependência de WebView do sistema (WebView2 no Windows) — contraria parcialmente "leve". |
| GTK4 / libadwaita (`gtk4-rs`) | Bindings nativos | Médio (dep. GTK) | Sim | Alta no Linux | Nativo no Linux, mas pesa mais em Windows/macOS (precisa empacotar runtime GTK). |

**Recomendação**: usar **egui/eframe** para o **picker** (latência mínima, renderização simples, fácil overlay always-on-top em todos os SOs) e **Slint ou egui** para o **app de configurações** — preferencialmente **a mesma stack (egui)** para reduzir complexidade de build/manutenção e tamanho final, a menos que se priorize um visual 100% nativo (caso em que Slint é a segunda opção recomendada). Tauri é desencorajado pelo requisito explícito de leveza (evitar dependência de WebView).

### 8.3 Fluxo de execução (alto nível)

1. SO chama `br open "https://exemplo.com/?utm_source=x"` (porque `br` está registrado como handler).
2. `br-cli`/`br-daemon` carrega `Config` (cache em memória se daemon ativo).
3. `br-platform` tenta identificar o app de origem (`get_foreground_app_name`/PID pai, quando possível).
4. `br-core::route(url, context, &config)`:
   - Aplica filtros → URL normalizada.
   - Avalia regras em ordem de prioridade → `RoutingDecision` (`OpenWith(target)`, `OpenWithAll(targets)`, `AskUser`, `Block`).
5. Se `AskUser`, `br-ui-picker` exibe a janela; usuário escolhe (ou timeout aplica padrão).
6. `br-platform::launch(target, url_normalizada, opts)` invoca o processo do navegador com argumentos apropriados (perfil, modo privado).
7. (Opcional) Log da decisão é gravado.

### 8.4 Modelo de dados de configuração (TOML — exemplo)

```toml
config_version = 1

[general]
default_action = "ask"          # "ask" | "open_with:<id>"
picker_timeout_ms = 0             # 0 = sem timeout
picker_position = "cursor"        # "cursor" | "center" | "top-right" | ...
theme = "system"                  # "system" | "light" | "dark"
language = "pt-BR"
start_on_login = true
log_level = "warn"

[[browsers]]
id = "chrome-default"
name = "Google Chrome"
kind = "chromium"                 # "chromium" | "firefox" | "generic"
executable = "auto"               # "auto" = detectar pelo SO
icon = "auto"

[[browsers]]
id = "chrome-work"
name = "Chrome (Trabalho)"
kind = "chromium"
executable = "auto"
profile_dir = "Profile 1"
icon = "auto"

[[browsers]]
id = "firefox-personal"
name = "Firefox (Pessoal)"
kind = "firefox"
executable = "auto"
profile_name = "default-release"

[[filters]]
id = "strip-tracking"
enabled = true
strip_query_params = ["utm_*", "gclid", "fbclid", "mc_eid", "igshid"]

[[filters]]
id = "https-upgrade"
enabled = true
upgrade_http_to_https = true
exceptions = ["*.local", "127.0.0.1", "localhost"]

[[rules]]
id = "work-mail-links"
name = "Links do Gmail Trabalho -> Chrome Work"
enabled = true
priority = 100
match = { url_pattern = ["*://mail.google.com/*", "*://*.google.com/url?*"] , source_app = ["Outlook", "Slack"] }
action = { open_with = "chrome-work" }

[[rules]]
id = "social-private"
name = "Redes sociais -> Firefox Pessoal em modo privado"
enabled = true
priority = 50
match = { url_pattern = ["*://*.instagram.com/*", "*://*.x.com/*", "*://*.facebook.com/*"] }
action = { open_with = "firefox-personal", private = true }

[[rules]]
id = "fallback"
name = "Padrão: perguntar"
enabled = true
priority = 0
match = { url_pattern = ["*"] }
action = { ask = true }
```

---

## 9. UX — Fluxos principais

### 9.1 Onboarding (primeira execução)

1. Tela de boas-vindas explicando o que o `br` faz (com aviso claro de privacidade: "100% local, sem telemetria").
2. Detecção automática de navegadores instalados (mostrar lista com ícones, permitir desmarcar os que não devem aparecer).
3. Passo "Definir como navegador padrão": botão que abre as configurações nativas do SO + instruções ilustradas específicas (Windows/macOS/Linux).
4. Passo opcional: criar 1-2 regras de exemplo guiadas (ex.: "Quer que links de redes sociais sempre abram no navegador X?").
5. Tela final: resumo + botão "Concluir" → app vai para a bandeja/system tray.

### 9.2 Uso do picker

1. Usuário clica em link em um app não-navegador.
2. SO invoca `br`.
3. Janela pequena aparece próxima ao cursor (ou posição configurada), mostrando: URL (truncada/elidida), e uma grade/lista de ícones de navegadores/perfis.
4. Usuário:
   - Clica em um ícone → abre nesse destino.
   - Pressiona número/letra → abre no destino correspondente.
   - Segura Shift ao clicar → abre em modo privado.
   - Clica em "Sempre" próximo a um destino → cria regra para o domínio atual e fecha.
   - Pressiona Esc → cancela (não abre nada).
   - Não faz nada por X segundos (se configurado) → abre no destino padrão.

### 9.3 Edição de regras (modo avançado vs. simples)

- **Modo simples**: formulário com dropdowns ("Quando a URL contiver... e vier de... então abrir em...").
- **Modo avançado**: editor de texto TOML com syntax highlight e validação inline (erros destacados por linha).
- Botão "Testar regra" → executa `br rules test <url-de-exemplo>` e mostra o resultado (qual regra/ação seria aplicada).

---

## 10. Roadmap de releases sugerido

| Versão | Conteúdo | Critério de saída |
|---|---|---|
| **v0.1 (alpha interna)** | br-core (engine de regras + filtros) com testes unitários completos; CLI `open`/`rules test`; sem UI gráfica (picker via terminal/stub) | Engine de regras 100% coberta por testes; roteamento funcional via CLI nas 3 plataformas |
| **v0.2 (alpha)** | br-platform para 1 SO (priorizar Windows ou macOS conforme uso pessoal do mantenedor); detecção de navegadores/perfis; registro como handler padrão (manual/guiado) | Handler funcional ponta a ponta em 1 SO |
| **v0.3** | Picker visual (egui) + app de configurações básico | Onboarding completo em 1 SO |
| **v0.4** | Suporte ao 2º SO | Paridade funcional entre 2 SOs |
| **v0.5** | Suporte ao 3º SO (Linux X11+Wayland) | Paridade nas 3 plataformas |
| **v1.0 (MVP completo)** | Todos os itens da seção 5.1, instaladores assinados, `br doctor`, documentação de usuário | Critérios da seção 2 atingidos |
| **v1.x** | Itens da seção 5.2 (perfis de workspace, scripting, extensão de navegador, sync) | — |
| **v2+** | Itens da seção 5.3 (plugins, MDM, estatísticas locais) | — |

---

## 11. Critérios de aceite (resumo para QA)

- [ ] `br` pode ser definido como navegador padrão em Windows 11, macOS 14+ e ao menos 2 distros Linux (uma com GNOME/Wayland, uma com KDE/X11).
- [ ] Clicar em um link em um app não-navegador (ex.: Slack, e-mail) abre o picker em < 100ms (com daemon ativo).
- [ ] Regras com `url_pattern` glob e `regex:` funcionam corretamente, incluindo precedência por `priority`.
- [ ] Filtros removem corretamente parâmetros `utm_*`, `gclid`, `fbclid` de uma URL de teste, preservando demais query params.
- [ ] Upgrade HTTP→HTTPS funciona e respeita exceções configuradas.
- [ ] Abrir URL em perfil específico do Chrome/Edge/Brave e do Firefox funciona (validar que o perfil correto é carregado).
- [ ] Modo privado/anônimo funciona para Chrome, Edge, Firefox, Brave, Safari (quando aplicável).
- [ ] `br rules test <url>` retorna a decisão correta sem efeitos colaterais (não abre nada).
- [ ] Configuração corrompida não impede o roteamento (fallback para picker com navegadores padrão).
- [ ] Hot-reload: editar `config.toml` externamente reflete no comportamento sem reiniciar.
- [ ] `br doctor` reporta corretamente status de handler padrão e navegadores detectados nas 3 plataformas.
- [ ] Nenhuma chamada de rede é feita pelo binário principal durante uso normal (validável via monitor de rede/sandbox).
- [ ] App de configurações é navegável 100% por teclado.

---

## 12. Riscos técnicos e mitigação

| Risco | Impacto | Mitigação |
|---|---|---|
| Windows não permite registrar handler padrão programaticamente sem interação do usuário | Alto | Fluxo guiado que abre diretamente a tela de Configurações correta + instruções visuais passo a passo |
| macOS exige assinatura/notarização para distribuição confiável | Médio | Orçar custo de Apple Developer Program; CI com assinatura automatizada; build não assinado documentado como "use por sua conta e risco" para early adopters |
| Wayland limita janelas always-on-top/posicionamento | Alto (Linux) | Detectar compositor e usar protocolos disponíveis (`wlr-layer-shell` quando suportado); fallback para janela normal centralizada quando não suportado |
| Detecção de "app de origem" não é confiável em todos os SOs | Médio | Tornar regras baseadas em `source_app` "best effort"; documentar limitações por SO; não bloquear demais funcionalidades nessa detecção |
| Parsing de `Local State`/`profiles.ini` pode quebrar com updates dos navegadores | Médio | Parsing tolerante a falhas (campos ausentes não quebram), testes com fixtures de múltiplas versões, fallback para "sem perfis detectados" sem crash |
| Performance do picker (cold start) | Médio | Processo daemon residente opcional (br-daemon) para eliminar cold start na maioria dos casos |

---

## 13. Fora de escopo (explicitamente)

- BrowserRouter **não é** um navegador (não renderiza páginas web).
- Não inclui sincronização em nuvem própria (apenas suporte a sincronizar o arquivo de config via ferramentas de terceiros escolhidas pelo usuário).
- Não inclui telemetria, analytics ou contas de usuário.
- Funcionalidades de teste cross-browser tipo MultiBrowser (emuladores mobile, gravação de vídeo, automação de testes) **não fazem parte do escopo** — `br` é um roteador de links, não uma ferramenta de QA/automação de browser.

---

## 14. Boas práticas de desenvolvimento open source (Rust)

Esta seção define o conjunto mínimo de práticas, ferramentas e convenções a adotar desde o primeiro commit, para que o projeto seja saudável, auditável e amigável a contribuidores externos.

### 14.1 Estrutura e arquivos essenciais do repositório

- **`README.md`**: descrição do projeto, badges (CI, licença, versão crates.io), instalação, uso básico, link para documentação completa.
- **`LICENSE` (dual licensing)**: adotar **MIT OR Apache-2.0**, padrão de facto no ecossistema Rust — maximiza compatibilidade com outros crates e empresas.
- **`CONTRIBUTING.md`**: como configurar o ambiente de dev, rodar testes/lints, convenções de commit, processo de PR/review.
- **`CODE_OF_CONDUCT.md`**: adotar o [Contributor Covenant](https://www.contributor-covenant.org/).
- **`SECURITY.md`**: política de divulgação responsável de vulnerabilidades (canal de contato privado, prazo de resposta esperado).
- **`CHANGELOG.md`**: seguindo o formato [Keep a Changelog](https://keepachangelog.com/), atualizado a cada release.
- **`.github/`**: templates de issue (bug report, feature request) e de pull request; `dependabot.yml`/`renovate.json` para atualização automática de dependências; workflows de CI/CD.
- **`rust-toolchain.toml`**: fixa a versão/toolchain do Rust usada no CI e recomendada para contribuidores.

### 14.2 Estilo de código e qualidade estática

- **`rustfmt`**: formatação obrigatória, com `rustfmt.toml` no repositório definindo as convenções do projeto; `cargo fmt --check` no CI bloqueia PRs não formatados.
- **`clippy`**: rodar `cargo clippy --workspace --all-targets -- -D warnings` no CI; lints adicionais (`pedantic`/`nursery`) podem ser habilitados seletivamente por crate.
- **MSRV (Minimum Supported Rust Version)**: declarar explicitamente (`rust-version` no `Cargo.toml` de cada crate) e validar no CI com uma matriz incluindo o MSRV e o `stable` mais recente.
- **Edition**: usar a edition mais recente estável do Rust suportada pelo MSRV escolhido.
- **Pre-commit hooks** (opcional, recomendado): via `cargo-husky` ou hook git simples rodando `cargo fmt` e `cargo clippy` antes do commit, para feedback rápido local.

### 14.3 Testes e cobertura

- **Testes unitários** colocados junto ao código (`#[cfg(test)] mod tests`), priorizando `br-core` (engine de regras e filtros) com cobertura próxima de 100%.
- **Testes de integração** em `tests/` por crate, incluindo fixtures (ex.: arquivos `Local State`/`profiles.ini` de exemplo para `br-platform`).
- **Property-based testing** (`proptest` ou `quickcheck`) para a engine de regras e parsers de URL/regex, garantindo robustez contra entradas inesperadas.
- **Cobertura de código**: medir com `cargo-llvm-cov`, publicar no Codecov (ou similar) e exibir badge no README; sem gate obrigatório de percentual no MVP, mas monitorado.
- CI deve rodar `cargo test --workspace` em todas as plataformas-alvo (Windows, macOS, Linux) via matriz do GitHub Actions.

### 14.4 CI/CD e releases

- **CI (GitHub Actions)**: matriz `os: [windows-latest, macos-latest, ubuntu-latest]` executando `fmt --check`, `clippy`, `test`, e build de release; cache de dependências via `Swatinem/rust-cache`.
- **Auditoria de dependências**: `cargo audit` (vulnerabilidades conhecidas via RustSec) e `cargo deny` (licenças incompatíveis, crates banidos, duplicação de versões) rodando no CI.
- **Versionamento semântico (SemVer)**: aplicado a todos os crates publicados; mudanças que quebram compatibilidade exigem bump de major.
- **Conventional Commits**: usar prefixos (`feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `ci:`) para permitir geração automática de changelog e bump de versão.
- **Automação de release**: avaliar `release-plz` ou `cargo-release` para automatizar bump de versão, geração de changelog e tagging, integrados ao pipeline de CI descrito na seção 7 (Distribuição).
- **Builds reprodutíveis**: `Cargo.lock` versionado (mesmo para crates binários), garantindo builds determinísticos no CI e em releases assinados.

### 14.5 Documentação

- **Rustdoc**: todo item público (`pub`) deve ter doc comments (`///`); publicar documentação da API em [docs.rs] automaticamente ao publicar no crates.io (para crates de biblioteca, ex. `br-core`).
- **Documentação de usuário**: considerar `mdBook` para um guia de usuário (instalação, configuração, exemplos de regras TOML), hospedado via GitHub Pages.
- **Exemplos**: diretório `examples/` com arquivos `config.toml` comentados demonstrando casos de uso comuns (seção 9 do PRD).

### 14.6 Governança e comunidade

- **Issue/PR templates**: incluir checklist de contribuição (testes adicionados, `cargo fmt`/`clippy` passou, changelog atualizado).
- **Labels padronizadas**: `good first issue`, `help wanted`, `bug`, `enhancement`, `platform:windows/macos/linux`.
- **Processo de review**: exigir ao menos 1 aprovação antes do merge em `main`; CI verde obrigatório (branch protection).
- **Canal de comunicação**: GitHub Discussions para dúvidas/propostas, evitando fragmentação em múltiplas plataformas.
- **Transparência de roadmap**: manter o roadmap (seção 10) refletido em GitHub Projects/Milestones públicos.

---

## 15. Glossário

- **Picker**: janela pequena exibida ao usuário para escolher o destino de um link.
- **Handler padrão**: aplicativo registrado no SO para abrir URLs `http`/`https` por padrão.
- **Regra (rule)**: condição + ação que determina automaticamente o destino de uma URL.
- **Filtro (filter)**: transformação aplicada à URL antes do roteamento (limpeza, rewrite, upgrade HTTPS).
- **Target/Destino**: combinação de navegador + perfil (+ flags como modo privado) para onde uma URL pode ser enviada.
- **Source app**: aplicativo de onde o clique no link se originou.
