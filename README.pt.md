# TMC — The Multipurpose Companion

<p align="center">
  <a href="README.md"><strong>English</strong></a> ·
  <a href="README.es.md"><strong>Español</strong></a> ·
  <a href="README.fr.md"><strong>Français</strong></a> ·
  <a href="README.de.md"><strong>Deutsch</strong></a> ·
  <a href="README.pt.md"><strong>Português</strong></a>
</p>

[![Versão](https://img.shields.io/badge/vers%C3%A3o-1.0.0-orange.svg)](https://github.com/Danikuso/TheMultipurposeCompanion/releases)
[![Licença: PolyForm Noncommercial 1.0.0](https://img.shields.io/badge/licen%C3%A7a-PolyForm%20Noncommercial%201.0.0-blue.svg)](LICENSE)
[![Plataforma](https://img.shields.io/badge/plataforma-Windows%2010%2F11%20%7C%20Linux%20%2F%20Steam%20Deck-green.svg)]()
[![Inara Whitelist](https://img.shields.io/badge/Inara%20InAPI%20v1-Whitelist%20Oficial-success.svg)]()

**TMC (*The Multipurpose Companion*)** é um assistente de cabine gratuito, de código aberto e em tempo real para ***Elite Dangerous*** (*Live 4.0 / Odyssey / Horizons* e *Legacy 3.8*), construído com **Rust**, **Tauri 2** e **Svelte 5**.

O seu objetivo é servir de copiloto integral tanto para pilotos novatos quanto para comandantes veteranos: reúne em um único aplicativo ultraleve **10 layouts de cabine combinados**, **16 módulos especializados**, **planejamento de rotas**, **guia passo a passo para desbloqueio de engenheiros**, **copiloto auditivo e consultas por voz**, além de uma **tela sensível ao toque auxiliar para tablet ou celular**.

---

## Índice deste Documento

- **Recursos para Jogadores:** [Manual de Instruções e Tutoriais (PDF)](website/Manual_TMC_Comandante.pdf) · [Página Web de Apresentação (`website/`)](website/index.html)
- **[Parte I — Guia para Jogadores e Usuários](#parte-i--guia-para-jogadores-e-usuários)**
  1. [Filosofia e Qualidades Principais](#1-filosofia-e-qualidades-principais)
  2. [Instalação e Primeiros Passos (Windows e Linux / Steam Deck)](#2-instalação-e-primeiros-passos)
  3. [Como Funciona a Interface (`TopBar`, `Auto-companion` e os 10 Layouts)](#3-como-funciona-a-interface)
  4. [Todas as Funcionalidades: Os 16 Módulos em Detalhes](#4-todas-as-funcionalidades-os-16-módulos-em-detalhes)
  5. [Copiloto de Voz, Consultas por Microfone, VoiceAttack e Tablet / Celular (QR)](#5-copiloto-de-voz-consultas-por-microfone-voiceattack-e-tablet--celular-qr)
  6. [Personalização em Ajustes e Suporte / Feedback](#6-personalização-em-ajustes-e-suporte--feedback)
- **[Parte II — Documentação Técnica para Desenvolvedores](#parte-ii--documentação-técnica-para-desenvolvedores)**
  7. [Arquitetura do Workspace (12 Crates + 2 Apps + Frontend)](#7-arquitetura-do-workspace)
  8. [Ingestão do Diário, Armazenamento, Segurança e Desempenho](#8-ingestão-do-diário-armazenamento-segurança-e-desempenho)
  9. [Caminhos Locais, Argumentos de Linha de Comando e Variáveis de Ambiente](#9-caminhos-locais-argumentos-de-linha-de-comando-e-variáveis-de-ambiente)
  10. [Ferramentas de Desenvolvimento, Reprodução e Auditoria Científica](#10-ferramentas-de-desenvolvimento-reprodução-e-auditoria-científica)
  11. [Compilação, Testes Automatizados e Empacotamento NSIS](#11-compilação-testes-automatizados-e-empacotamento-nsis)
  12. [Contribuição, Privacidade e Licença](#12-contribuição-privacidade-e-licença)

---

# Parte I — Guia para Jogadores e Usuários

## 1. Filosofia e Qualidades Principais

- **Um guia que nunca deixa você bloqueado:** se deseja aprimorar um módulo da sua nave mas ainda não descobriu ou não desbloqueou o engenheiro correspondente, o TMC não exibe um aviso em branco: ele mostra primeiro **passo a passo como descobri-lo, como obter o convite e qual tarefa ou doação a sua oficina exige** (lendo o progresso real do seu diário de jogo), e logo abaixo calcula os materiais e trocas necessários.
- **Clareza visual («TMC Glass HUD») e cores sincronizadas com a sua nave:** cada painel apresenta no topo as informações essenciais com medidores gráficos, semáforos de status e ações de um clique, mantendo tabelas extensas recolhidas em **`Ver informações adicionais`**. Além disso, o tema **`HUD automático`** detecta a matriz de cores personalizada da cabine do seu jogo para adotar exatamente a mesma paleta visual.
- **Leve e silencioso em segundo plano:** projetado para não comprometer a taxa de quadros (FPS) nem os recursos do seu computador. Ao fechar a janela (`✕`), o TMC continua em execução na bandeja do sistema (junto ao relógio do Windows), liberando sua memória gráfica enquanto mantém alertas de voz e a cópia automática de saltos funcionando.
- **100% respeitoso com seus dados e com a galáxia Legacy:** funciona de forma totalmente local desde o primeiro segundo, sem exigir nenhuma conta obrigatória. Se detectar que você está jogando na galáxia **Legacy 3.8**, bloqueia automaticamente qualquer envio para redes comunitárias externas (EDDN, EDSM, Inara) para evitar misturar os dados.
- **5 idiomas completos:** interface, dados de engenharia e avisos de voz disponíveis em **Português**, **Inglês**, **Espanhol**, **Francês** e **Alemão**.

---

## 2. Instalação e Primeiros Passos

### Windows 10 / 11 (64 bits)

1. Baixe o instalador **`TMC_0.1.0_x64-setup.exe`** na seção [Releases](https://github.com/Danikuso/TheMultipurposeCompanion/releases) (assistente disponível em todos os 5 idiomas).
2. Execute o instalador e conclua as etapas na tela.

> [!NOTE]
> **Aviso preventivo do Windows Defender SmartScreen:**  
> Por ser um projeto de código aberto sem fins lucrativos e sem certificado comercial corporativo pago, o Windows pode exibir a tela azul *«O Windows protegeu o seu computador»* na primeira execução. Clique em **«Mais informações»** e em seguida em **«Executar assim mesmo»**.

3. Abra o **TMC** antes ou depois de iniciar o *Elite Dangerous*. Ele detectará automaticamente a sua pasta de jogos salvos, importará o histórico em segundo plano (`Importando histórico: X de Y`) e memorizará o tamanho e a posição da janela entre as sessões.
4. **Bandeja do sistema:** ao clicar em `✕`, o TMC permanece ativo no ícone da bandeja do sistema. Clique com o botão esquerdo no ícone (ou clique com o botão direito → **`Abrir TMC`**) para restaurar a janela, ou selecione **`Sair`** para fechá-lo totalmente.

### Linux e SteamOS (Steam Deck)

- O TMC detecta automaticamente a pasta de diários do *Elite Dangerous* nos prefixos **Proton** da Steam (tanto em instalações nativas quanto em Flatpak, AppID `359320`).
- Você pode executar o aplicativo com interface gráfica (`tmc-desktop`) ou iniciar o servidor leve sem janela (`tmc-headless`) para abrir o painel pelo navegador ou por um tablet na mesma rede Wi-Fi.

---

## 3. Como Funciona a Interface

### 3.1 Barra Superior (`TopBar`)
Sempre visível no topo da tela:
- **Status e Galáxia:** indicador luminoso de conexão em tempo real, insígnia **`LIVE 4.0`** ou **`LEGACY`** e avisos de contexto (`O jogo não está em execução` ou `Galáxia Legacy: os dados permanecem locais`).
- **6 Métricas Instantâneas de Voo:** **`Sistema`** atual, **`MODO`** (entre 10 estados: *Atracado, Pousado, Espaço normal, Supercruzeiro, Hiperespaço, A pé, No SRV, No caça, No táxi, Tripulação múltipla*), **`Combustível`** (com aviso amarelo abaixo de 25%), **`Casco`** (com alerta vermelho abaixo de 30%), **`Carga`** (`usada / capacidade`) e **`Créditos`**.
- **Controles Rápidos à Direita:** botão de bloqueio tático **`🔓` / `🔒`** (com atalho **`Alt + L`** para congelar a tela de imediato), botão de microfone **`🎙️`** e interruptor **`Auto-companion`**.

### 3.2 Auto-Companion Pro: Inteligência Preditiva, Trava Tática e Prioridades
- **Alternância Inteligente e Contínua:** com o **`Auto-companion` ativado**, o TMC muda automaticamente para o layout de cabine mais relevante com base nas suas ações no jogo (combate, prospecção de asteroides, atracação em estação, engenharia, salto de Fleet Carrier, colonização ou viagem no hiperespaço).
- **Trava Tática de 1 Clique (`Alt + L` / Cadeado):** em meio a um combate intenso, ao estudar uma rota ou ao revisar uma receita de engenharia, clique no cadeado na barra superior ou pressione **`Alt + L`** para congelar a tela atual. O cadeado brilha em tom âmbar no estilo *Glass HUD*, impedindo qualquer alteração automática de tela até que você decida destravá-lo.
- **Histerese e Retenção sem Saltos Bruscos (`linger_seconds`):** após o término de um combate, de uma prospecção mineral ou de uma venda no mercado, a tela não muda abruptamente. Ela permanece no cockpit ativo durante o período configurado (padrão de 20 segundos, ajustável de 0 a 60 s em Ajustes) para que você avalie danos, recompensas e lucros com tranquilidade.
- **Transições Preditivas em Voo:** o TMC antecipa os acontecimentos antes que eles ocorram:
  - *Interdição iminente:* ao sofrer ou iniciar uma interdição em supercruzeiro, a cabine muda imediatamente para *Combate* durante a disputa de fuga.
  - *Aproximação de anéis:* ao descer em direção aos anéis ou cinturões de um planeta portando equipamentos de mineração, muda preventivamente para *Mineração*.
  - *Autorização de atracação concedida:* ao receber permissão de pouso em uma estação ou entreposto, muda para *Comércio*, preparando o mercado e os serviços locais.
- **Ordem de Prioridades Personalizada:** em `Ajustes`, você pode definir a hierarquia dos 9 papéis usando simples setas `▲` e `▼`, ou clicar em `Restaurar ordem padrão` para voltar à configuração tática recomendada.
- **Feedback Sensorial de Cabine:** ative o bipe sonoro suave (*Glass HUD Chime*, sintetizado nativamente via Web Audio API de 880 Hz a 1320 Hz sem arquivos pesados) ou avisos falados (TTS) do copiloto para saber quando a tela mudou sem precisar tirar os olhos dos instrumentos de voo.
- No layout de **`Exploração`**, o **`Painel de contexto`** alterna automaticamente entre **`Corpos do sistema`** e **`Exobiologia`** ao pousar na superfície de um mundo com sinais biológicos.
- Se você selecionar manualmente outro layout enquanto o `Auto-companion` estiver ativo (sem o cadeado travado), o TMC respeita a sua escolha até que a atividade no jogo mude. Desativar o `Auto-companion` mantém a tela fixa onde você a deixar.

### 3.3 Os 10 Layouts Combinados vs. Os 16 Módulos
O menu lateral esquerdo oferece duas formas complementares de navegação (além do botão fixo **`⚙️ Ajustes`** na parte inferior):

| Layout Combinado | Painéis reunidos em uma única tela |
|---|---|
| **1. `Exploração`** | Faixa de `Rota`, `Painel de contexto` (seletor `Corpos` / `Exobiologia`), `Planejador de rotas` e `Alcance de salto`. |
| **2. `Nave`** | `Nave` (equipamentos e engenharia), `Alcance de salto`, `Carga` e `Busca de Estaleiro e Módulos`. |
| **3. `Combate`** | `Alvo`, `Sessão de caça`, `Minha nave` e painéis automáticos de `Zona de conflito` e `Anti-Xeno (AX)` quando ativos. |
| **4. `Mineração`** | `Localizador de mineração` (circuito triangular e melhor venda), `Prospector`, `Sessão de mineração` e `Onde vender`. |
| **5. `Comércio`** | `Mercado atual`, `Carga`, `Sessão de comércio`, `Rota comercial` e `Localizar mercadoria`. |
| **6. `Missões`** | `Missões ativas`, `Avisos`, `Recompensas da sessão` e painéis automáticos de `Pilha de massacres`, `Passageiros`, `Turismo` e `Objetivos comunitários`. |
| **7. `Engenharia`** | `Materiais`, `Engenheiros`, `Avisos de engenharia`, `Nave modificada`, `Engenheirar módulo`, `Lista de compras`, `Trocas` e `Onde encontrar`. |
| **8. `Fleet Carrier`** | `Status do Carrier`, `Sequência de salto` e `Finanças e manutenção`. |
| **9. `Colonização`** | `Depósito de construção` e `Suprimento e compras`. |
| **10. `Powerplay 2.0`** | `Status de Powerplay` e `Facções do sistema` (BGS). |

Abaixo dos layouts, você encontrará a lista dos **16 Módulos individuais** (com um ponto luminoso indicando atividade em tempo real) para abrir qualquer ferramenta em largura total.

---

## 4. Todas as Funcionalidades: Os 16 Módulos em Detalhes

### 1. `Painel Geral` (7 painéis)
- **`Comandante`:** nome, saldo de créditos e barras de progresso com porcentagem nos 6 rankings da *Federação de Pilotos* (*Combate, Comércio, Exploração, Mercenário, Exobiologista, CQC*) e nos 2 das *Superpotências* (*Federação, Império*).
- **`Nave e Localização`:** resumo da nave ativa, sistema estelar, corpo celeste mais próximo, estação e modo atual.
- **`Status ao vivo`:** 7 indicadores luminosos (*Escudos, Armas acionadas, Trem de pouso, Bloqueio de massa, Navegação silenciosa, Coletor de carga, Sobrecarga de supercruzeiro SCO*) e distribuição de energia do distribuidor `SYS`, `ENG` e `WEP` (0 a 4 pips).
- **`Rota e Sessão`:** próximo sistema com botão **`Copiar próximo sistema`**, saltos restantes, duração da sessão, saltos concluídos, anos-luz percorridos e créditos obtidos.
- **`Alertas`:** notificações imediatas para *Combustível baixo, Superaquecimento, Interdição em andamento, Sob ataque, Dano no casco, Oxigênio baixo, Saúde baixa* ou falha de módulo.

### 2. `Nave` (2 painéis: Auditoria técnica e busca de Naves/Equipamentos)
- **Cabeçalho e Medidores (`Nave`):** modelo, nome de batismo, registro, massa vazia, custo de seguro (*rebuy*) e medidores gráficos para integridade do casco, escudos, tanques de combustível principal + reserva e carga.
- **3 Guias de checagem pré-voo:**
  - **`🧭 Exploração`:** alcance máximo de salto e lista de conferência (`✅` / `⚪`) para coletor de combustível, propulsor FSD Guardian, scanner de superfície DSS, hangar de SRV e unidade de manutenção AFMU.
  - **`⚔️ Combate`:** escudos, dissipação térmica, distribuição `SYS/ENG/WEP`, armas instaladas com munição `[pente / reserva]` e utilitários de defesa (*reforços de escudo, chaff, defesas de ponto e dissipadores de calor*).
  - **`📦 Carga e Mineração`:** capacidade total, compartimentos instalados, refinaria, controladores de drones e carga a bordo.
- **`Detalhamento módulo por módulo`:** filtrável por categoria (*Todos, Armas, Primários, Opcionais, Utilitários*) e por busca. Mostra classe e nota `[5A]`, integridade `%`, status de energia `ON/OFF`, prioridade `P1..P5` e insígnia dourada `G1..G5 ★` que, ao ser clicada, exibe a receita, o engenheiro, o efeito experimental e a **comparação exata de cada atributo (valor aprimorado vs. valor original)**.
- **`Estaleiro e Módulos` (Buscador de compra completa em parada única):**
  - **Cesta combinada a partir da sua posição:** parte automaticamente do seu sistema atual (mesmo em regiões profundas não indexadas via coordenadas 3D) e permite escolher **1 nave opcional** (48 modelos) e/ou **até 10 módulos** (153 famílias com filtros de categoria, classe `0..8`, nota `A..I`, tipo de armação *Fixa/Gimbal/Torre* e blindagens) para encontrar as estações mais próximas que vendem **toda a cesta de uma só vez**.
  - **Alternativa inteligente por item:** se nenhuma estação vender todos os itens simultaneamente, o sistema localiza a estação mais próxima para cada peça individualmente, sinalizando itens sem estoque conhecido.
  - **Travas de segurança e comodidades:** filtro **`−15% Li Yong-Rui`** (que reduz permanentemente o valor do seu seguro de recompra), botão dedicado **`Transportadores Fortaleza da sua potência`** para incluir os *Stronghold Carriers* da sua facção de Powerplay 2.0 (mantendo transportadores rivais e privados ocultos por padrão), proteção de sistemas que exigem autorização (`Shinrarta Dezhra`, `Sol`, `Achenar`, `Sirius`, `Alioth`…), **semáforo de segurança de recompra** (`Recompra OK` / `Ajustar recompra` / `Saldo insuficiente`), preços itemizados e botões rápidos **`Copiar`** e **`Traçar rota`**.

### 3. `Navegação` (3 painéis e planejador duplo Spansh)
- **Faixa de `Rota`:** exibe o plano Spansh ativo ou a rota traçada no mapa da galáxia do jogo. Ao desviar do trajeto, surge o aviso **`Fora da rota`** com botões **`Recalcular`** e **`Limpar plano`**. Detalha cada salto em 4 colunas: navegação (distância, classe estelar, indicador **`⛽ Reabastecimento possível`**, combustível residual estimado), exploração (descoberto/não descoberto, visitas anteriores, estrela de nêutrons/anã branca), política (aliança, governo, facção e **`⚠ Estado de perigo`** em guerras ou epidemias) e economia/segurança/Powerplay.
- **`Alcance de salto`:** alcance atual (com insígnia **`Sobrecarga ×N`** por nêutrons ou síntese), alcance com tanque cheio, nível de combustível e consumo máximo por salto.
- **`Planejador de rotas`:** preenchimento automático de sistemas, ponto de partida por coordenadas 3D e 2 motores de cálculo:
  - **`⚡ Rodovia de Nêutrons`:** cálculos rápidos através de estrelas de nêutrons com **`Eficiência (%)`** ajustável.
  - **`🗺️ Rota Exata (Exact Plotter)`:** lê peso, tanques, carga, motor FSD aprimorado e propulsor Guardian diretamente da sua nave sem enviar dados para a internet, com opções de **`Somente estrelas de reabastecimento`** e **`Usar injeções FSD`**. Ao clicar em **`Aceitar`**, grava a rota e copia automaticamente cada próximo salto para a área de transferência.

### 4. `Exploração` (`Corpos do sistema`)
- **Resumo superior:** status de descoberta do sistema, todas as classes estrelares presentes (destacando estrelas coletáveis *O, B, A, F, G, K, M* em verde), progresso do **`Scanner FSS`**, cartões comparativos **`Valor do sistema`** vs. **`Valor com mapeamentos recomendados`**, corpos de alto valor não mapeados e cartão **`Dados não vendidos`** com aviso em vermelho **`Dados em risco`** caso o casco tenha sofrido danos.
- **Tabela detalhada expansível:** 27 classificações estelares e planetárias, distância em segundos-luz, valor de escaneamento e mapeamento, sinais `Bio` e `Geo`, marcas (*Terraformável, Pousável, Materiais, Primeira descoberta, Já mapeado*) e coluna **`Recomendação`** (`★ Mapear` segundo o seu limite em créditos, `Pousar (exobiologia)` ou `Não compensa`, acompanhada da meta de sondas DSS).

### 5. `Exobiologia` (Assistente de amostragem na superfície Odyssey)
- **Cabeçalho planetário:** gravidade da superfície, temperatura, composição atmosférica, contagem de sinais biológicos e insígnia **`NÃO REGISTRADO · ×5 (estimado)`** quando nenhum comandante registrou o planeta anteriormente.
- **Rastreador de `Amostragem em andamento` (`1/3 → 2/3 → 3/3`):** barra e contador ao vivo de **`Distância até a amostra anterior`** comparada com a distância mínima clonal exigida pela espécie (`Faltam X m`), **indicação de direção relativa e bússola cardinal** com base para onde você está olhando (ex.: *«a amostra anterior está atrás à sua esquerda (SO)»*), aviso caso tente descartar uma espécie incompleta e projeção de lucros finais em **`Valor a bordo não vendido`**.
- **Tabela de `Espécies potenciais (ordenadas por valor)`:** cruza critérios astrofísicos, histórico de voo e dados da *Canonn* para exibir pagamento base, valor `Com bônus ×5`, distância mínima, probabilidade e sinais de superfície confirmados pelo scanner DSS.

### 6. `Combate` (5 painéis táticos e Anti-Xeno)
- **`Alvo`:** nave selecionada com emblema, piloto, ranking de combate, facção, faixa destacada **`LIMPO · NÃO DISPARAR`** (verde) ou **`PROCURADO · recompensa`** (vermelho), medidores de escudos e casco, e saúde em porcentagem do **`Subsistema`** selecionado (*Usina de força, Propulsores, FSD, Armas…*), retendo as informações por 10 segundos após a perda de mira.
- **`Minha nave`:** status de escudos, barra de casco em 10 segmentos, medidor térmico com cronômetro em superaquecimento, níveis do distribuidor (`SYS`, `ENG`, `WEP`) e alertas ativos.
- **`Sessão de caça`:** abates, créditos ganhos, rendimento por hora (`cr/h`), progresso do ranking de combate, botão **`Reiniciar sessão`**, lista de **`Títulos de recompensa pendentes`** por facção (com cálculo líquido a **`−25% no Interstellar Factors`**) e localização do contato mais próximo.
- **`Zona de conflito e Anti-Xeno (AX)` (Automático):** facção escolhida, abates e títulos de combate; alerta de presença Thargoid, títulos AX e contador de abates por variante (*Scout, Cyclops, Basilisk, Medusa, Hydra, Scythe, Glaive, Orthrus…*).

### 7. `Mineração` (4 painéis: Localizador, Prospector, Sessão e Onde vender)
- **`Localizador de alvos de mineração e venda ideal (Miner's Companion)`:**
  - **Circuito triangular mais curto:** calcula no cockpit a rota ideal completa: do seu sistema atual ➔ ao melhor anel ou cinturão ➔ até a estação mais próxima com o melhor preço de compra para os seus minérios.
  - **3 Perfis de mineração especializados & Atalhos rápidos:**
    - **Laser:** focado em Platina em anéis metálicos, Painita e Ósmio. Inclui combinações com locais de extração *Hazardous RES* (+100% de fragmentos por rocha) e *High RES* (+75% de fragmentos), além de sobreposições (*overlaps*) verificadas.
    - **Núcleo profundo (*Core*):** focado em Musgravita, Monazita, Alexandrita, Ópalas do Vácuo e Grandidierita em anéis rochosos e de gelo.
    - **Missões e Engenheiros:** Bromelita (para desbloquear o engenheiro Bill Turner em Alioth), Trítio (para abastecer *Fleet Carriers*) e Ósmio.
  - **Alerta de risco de taxa sobre demanda (*Demand Tax Risk*):** a mecânica de mercado do *Elite Dangerous* reduz drasticamente os pagamentos caso a carga transportada supere 25% da demanda total da estação. O TMC alerta preventivamente por semáforo e indica o limite seguro de toneladas.
  - **Otimizador Multi-Venda em 1 clique:** avalia os minérios e metais armazenados na sua nave e encontra a estação ideal que adquire toda a combinação com o maior lucro somado.
  - **Catálogo offline curado e sincronização atômica online:** vem acompanhado de mais de 40 locais de alta densidade confirmados pela comunidade (*EDTools/Spansh*), funcionando de pronto sem internet e atualizando-se silenciosamente em segundo plano com conexão ativa.
  - **Ações imediatas de cabine:** botões **`📋 Copiar sistema`** e **`🚀 Traçar rota`** em cada cartão e na tabela dinâmica com barra de rolagem horizontal fixa.
- **`Prospector`:** teor mineral (*Baixo/Médio/Alto*), faixa destacada **`◆ Núcleo: Minério`** para detonação profunda, barra multicolorida de composição, lista com minerais favoritos destacados com `★`, parecer automático **`✔ Vale a pena`** ou **`✘ Descartar`** baseado no seu limite de créditos e gráfico comparativo dos **`Últimos 8 asteroides`**.
- **`Sessão de mineração`:** tempo decorrido, tonelagem refinada e ritmo em `t/h`, estimativa de ganhos e `cr/h`, drones **`Limpets`** restantes e disparados, ocupação do compartimento de carga, distribuição por minério e estado das cubas da **`Refinaria`**.
- **`Onde vender`:** estação recomendada para a totalidade da carga, além de opções expansíveis por minério com estação, sistema (com botão de cópia `⎘`), distância, preço pago, demanda, tamanho da plataforma (`S/M/L`) e data da cotação.

### 8. `Comércio` (5 painéis)
- **`Mercado atual`:** evidencia as melhores oportunidades das últimas 48 horas e lista o catálogo completo, marcando mercadorias **`RARA`** (com distância ideal de venda de 160 al), preços de compra, estoque, melhor comprador nas redondezas e lucro por tonelada.
- **`Minha carga`:** ocupação da soute e controle de **`Preço médio pago`**, etiquetas de carga roubada ou de missão, comparação entre vender no local ou no melhor mercado vizinho e instrução direta **`VENDER AQUI`** ou **`TRANSPORTAR`**.
- **`Rota comercial`:** otimizador multiparadas a partir do porto atual, discriminando o que comprar e vender em cada escala, saltos necessários, lucro total e cópia automática da próxima escala.
- **`Sessão de comércio e Localizar mercadoria`:** lucro líquido acumulado, `cr/h`, tonelagem transportada, melhor viagem, progresso do ranking comercial e ferramenta de busca com preenchimento automático para encontrar fornecedores próximos de qualquer item.

### 9. `Missões` (7 painéis: Contratos, Massacres, Passageiros, Turismo e CGs)
- **`Missões ativas e Avisos`:** contratos organizados por destino com marcação `▶ Próxima parada` e botão **`Adicionar à rota`**, tipo, facção, progresso, contagem regressiva de expiração (em vermelho se `< 1 h`) e alertas para rota incorreta ou falta de carga da missão a bordo.
- **`Pilha de massacres`:** agrupa missões de combate contra a mesma facção pirata em barras paralelas por facção contratante (onde cada abate computa simultaneamente para todas as facções distintas), calculando baixas restantes e valor total.
- **`Passageiros`:** acomodação por classe de cabine (*Econômica, Executiva, Primeira Classe, Luxo*), marcadores **`VIP`** / **`PROCURADO`** e aviso crítico em vermelho **`⚠ Passageiro procurado a bordo: risco de escaneamento em áreas de segurança`**.
- **`Turismo`:** reordena automaticamente as balizas turísticas no trajeto mais curto (**`Rota TSP Ideal`**) frente à ordem do painel de missões (mostrando anos-luz e saltos economizados), calcula `cr/al` e `cr/h` e fornece o veredito **`✔ LUCRATIVO`** ou **`✖ NÃO COMPENSA`**.
- **`Recompensas da sessão e Objetivos comunitários`:** resumo de créditos recebidos, materiais obtidos e variações de reputação/influência por facção, além do monitoramento em tempo real de *Community Goals* (*participação, estágio, percentil, tempo restante e recompensa estimada*).

### 10. `Engenharia` (8 painéis para nave e trajes/armas Odyssey)
- **`Materiais`:** 4 abas (*Matérias-primas, Manufaturados, Dados codificados*, e o *Armário* Odyssey dividido em *Itens, Componentes, Consumíveis e Dados*), busca instantânea, **`Histórico da sessão`** indicando a origem (*Missão, Escaneamento, Mineração, Troca, Engenharia…*) e alertas de **`CAPACIDADE MÁXIMA`**.
- **`Engenheiros e Minha nave modificada`:** catálogo com filtros (*Todos, Desbloqueados, Pendentes*) de **engenheiros de naves e engenheiros a pé Odyssey** com nível `★`, distância e progresso das condições de desbloqueio; seletor rápido para qualquer módulo instalado suscetível a modificações.
- **`Engenheirar módulo`:** escolha o tipo de aprimoramento, efeito experimental, graduação `G1..G5` e tentativas por grau. Se ainda não possuir acesso a um engenheiro qualificado para esse nível, o painel exibe o **`GUIA DE APOIO — DESBLOQUEIO DE ENGENHEIRO`** detalhando as 3 etapas (*1. Descoberta → 2. Convite → 3. Tarefa/Doação de desbloqueio*), sistema com cópia rápida e progresso atual, acompanhado dos custos exatos e semáforo de disponibilidade (`OK`, **`Atendido via troca`** ou `Faltando`).
- **`Lista de compras, Trocas, Onde encontrar e Avisos`:** administre múltiplos projetos, ordene-os por prioridade (**`▲ Subir` / `▼ Descer`**) para reservar insumos, calcule trocas matemáticas com excedentes não alocados (`Entregar X → Receber Y`) e localize o **Comerciante de Materiais** ou o **Corretor de Tecnologia** (*Humano* e *Guardian*) mais próximo com roteiros de coleta.

### 11. `Fleet Carrier` (3 painéis)
- **`Status do Carrier`:** indicativo de chamada, nome de batismo, marcador `A BORDO`, sistema atual, saltos viáveis (500 al/salto), permissões de doca, nível do combustível **`Trítio`** somando o tanque (`1.000 t`) + reserva no compartimento de carga, e ocupação de espaço livre vs. alocado.
- **`Sequência de salto`:** estado da operação e cronômetro em tempo real `MM:SS` para a contagem de salto (15 min) e resfriamento posterior (5 min), indicando o destino e botão **`Copiar destino`**.
- **`Finanças e manutenção`:** saúde financeira (*Saudável, Alerta, Crítico*), **`Semanas cobertas`**, saldo do transportador, custo semanal de manutenção, reserva mínima e data do próximo débito.

### 12. `Colonização` (2 painéis)
- **`Depósito de construção`:** porcentagem concluída da obra, tonelagem entregue vs. pendente, cálculo automático do **`Total de viagens estimadas`** com base na capacidade da sua nave em uso, e listagem recurso por recurso.
- **`Suprimento e compras`:** cartões de insumos ordenados por urgência com quantidades faltantes, viagens necessárias e botão **`Copiar`** para compras rápidas.

### 13. `Powerplay 2.0` e 14. `BGS`
- **`Status de Powerplay`:** brasão da potência e superpotência, semanas de fidelidade, ranking atual (`X / 100`), méritos reunidos e faltantes para o próximo patamar, méritos obtidos na sessão, **`Taxa horária de méritos`** e contagem regressiva para a virada semanal (*Quinta-feira 07:00 UTC*).
- **`Facções do sistema (BGS)`:** facção controladora, estados vigentes, acompanhamento do **`Tick galáctico`** diário (`Há X horas`, sinalizando dados atualizados ou pendentes de processamento) e relação de todas as facções com gráficos de influência, fidelidade e modelos de governo.

### 15. `Crime e Notoriedade` (5 cartões táticos)
- **`Status Legal e Notoriedade`:** escala de 0 a 10 com temporizador regressivo de expiração (2 horas de voo ativo por ponto) capaz de registrar e avisar: **`⏸️ Queda pausada (atracado em estação)`**.
- **`Guia para Limpeza de Ficha`:** informa se contatos neutros estão indisponíveis pela notoriedade ou, zerada esta, aponta o **`Interstellar Factors mais próximo`** com distância, tamanho da plataforma e botão **`Copiar sistema`**.
- **`Contrabando e Mercados Negros`:** relação de mercadorias ilegais a bordo, localização do **`Mercado Negro mais próximo`** e manual em 3 passos para **`Entrada Furtiva`** (alinhamento a 8–10 km, navegação silenciosa / dissipador de calor a `< 4 km` e aproximação veloz).
- **`Quadro de Multas e Segurança`:** penalidades e recompensas por autoridade + nível de patrulhamento local com estimativa de resposta policial.

### 16. `Contas` (Sincronização comunitária opcional)
- **`Elite Dangerous Data Network (EDDN)`:** compartilhe cotações de mercado, estaleiros e registros de saltos de modo anônimo com a rede comunitária, recebendo dados comerciais atualizados na área configurada.
- **`Elite Dangerous Star Map (EDSM)`:** salve sua chave de API para manter sincronizados o diário de bordo, patentes e finanças.
- **`Inara`:** integração homologada na **Whitelist oficial da Inara** sob o título **`The Multipurpose Companion (TMC)`**, operando em lotes inteligentes (máximo de 2 chamadas a cada 60 s) e com teste de conectividade.
- **`Frontier Companion API (CAPI)`:** acesso autenticado e seguro por OAuth2 na página oficial da Frontier e gerenciador interativo da sua **`Frota remota guardada`** com todas as suas naves estacionadas em outros sistemas, localização, patrimônio e distância em anos-luz.

---

## 5. Copiloto de Voz, Consultas por Microfone, VoiceAttack e Tablet / Celular (QR)

### 5.1 Copiloto Auditivo (Avisos falados / TTS)
Em `Ajustes` → **Copiloto de voz e alertas de cabina**, escolha entre:
- **`Nativo do sistema operacional (Ultraleve)`:** faz uso imediato das vozes instaladas no seu Windows ou Linux sem downloads adicionais.
- **`Motor neural KokoroTTS (Voz cinematográfica de IA)`:** 10 vozes neurais locais de alta fidelidade em vários idiomas com hibernação em segundo plano para economizar memória (e retorno automático à voz do sistema se indisponível).
- Oferece ajuste de **volume**, **velocidade de fala**, teste sonoro **`🔊 Testar voz do copiloto`** e **5 canais independentes** (*Combate e alertas críticos*, *Navegação e reabastecimento estelar*, *Exploração e exobiologia*, *Mineração e prospecção*, e *Fleet Carrier*).

### 5.2 Assistente por Microfone (9 consultas faladas em 5 idiomas)
Em `Ajustes` → **Assistência por voz e integração**, ative o reconhecimento de fala com **`Whisper AI Local`** (~39 MB, 100% offline e privativo, instalado com um clique) ou pelo motor do sistema, operando em **Escuta contínua** ou **Aperte para falar** (botão `🎙️` no topo). Faça perguntas em linguagem natural sobre:
1. **Estado geral da nave** (integridade do casco, escudos e combustível).
2. **Nível de combustível** (porcentagem e toneladas).
3. **Próximo salto estelar** (anuncia o próximo destino da rota).
4. **Copiar próximo sistema** (anuncia e **copia o sistema para a área de transferência**, pronto para colar no mapa da galáxia).
5. **Sinais biológicos no sistema** (contagem de biosinais identificados).
6. **Resumo de materiais fixados** (andamento da lista de compras da engenharia).
7. **Situação legal e multas** (notoriedade, multas ativas e recompensas).
8. **Interstellar Factors mais próximo** (anuncia estação, sistema, distância e **copia o sistema para a área de transferência**).
9. **Mercado negro mais próximo** (anuncia estação, sistema, distância e **copia o sistema para a área de transferência**).

### 5.3 Integração com o `VoiceAttack`
O TMC detecta de forma automática se o *VoiceAttack* está em execução, monta o perfil sob medida **`TMC-Copiloto.vap`** com os comandos já atribuídos e disponibiliza o botão **`📥 Baixar perfil (.vap)`** em Ajustes para importação instantânea.

### 5.4 Tela Sensível ao Toque em Tablet ou Celular (`Código QR`)
Ao voar com um monitor único ou em óculos de realidade virtual (VR), vá em `Ajustes`, habilite a opção **`Acesso por rede local`** e aponte a câmera do seu smartphone ou tablet (no mesmo Wi-Fi) para o **`Código QR de emparelhamento`**. Toda a cabine se abre no navegador móvel sem necessidade de instalar aplicativos; tocar em «Copiar sistema» no tablet envia o destino diretamente para a área de transferência do seu PC.

---

## 6. Personalização em Ajustes e Suporte / Feedback

- **Idiomas e Temas:** transição instantânea entre **5 idiomas** (`es`, `en`, `fr`, `de`, `pt`) e **3 temas visuais** (**`HUD automático`** sincronizado com os tons da cabine da sua nave, **`Laranja clássico`** e **`Alto contraste`**).
- **Auto-Companion Pro:** ordenação de prioridade operacional para os 9 papéis (`▲` / `▼`) com botão de redefinição, controle deslizante de retenção/histerese (0 a 60 s com medidor ao vivo), chave de transições preditivas e avisos sonoros/falados (*Glass HUD Chime* e voz TTS).
- **Preferências operacionais:** cópia automática do salto subsequente, chave mestra do `Auto-companion`, conectividade de rede local LAN, chave geral de privacidade **`Consultar serviços online (Spansh, EDSM, Canonn)`** (desligue para operação 100% desconectada da internet), **`Limite de valor para mapear (cr)`** e **`Eficiência padrão do planejador de rotas (%)`**.
- **Economia, Turismo e Engenharia:** rol configurável de até 30 **`Minerais de interesse`** (11 já marcados), **`Valor mínimo para mineração (cr)`**, **`Raio de busca para vendas comerciais (al)`**, **`Raio de leitura de dados EDDN (al)`**, **`Patamar de referência turística (cr/al)`** e **`Tentativas de engenharia por grau (G1–G5)`**.
- **Canal de Comunidade e Suporte:** em `Ajustes` → **Suporte, Feedback e Comunidade**, clique em **`🚀 Reportar problema ou sugestão no GitHub`** para registrar um ticket no [GitHub Issues](https://github.com/Danikuso/TheMultipurposeCompanion/issues) contendo a versão, idioma e sistema operacional já anotados (sem incluir nenhuma informação privada do piloto).

---

# Parte II — Documentação Técnica para Desenvolvedores

## 7. Arquitetura do Workspace

O projeto está organizado como um workspace modular em **Rust estável (edição 2024)**, com separação estrita de atribuições e livre de ciclos de dependência, apoiado por uma camada de apresentação em **Svelte 5 + TypeScript + Vite** e catálogos **Fluent** para internacionalização:

| Crate / Diretório | Responsabilidade técnica |
|---|---|
| `crates/tmc-model` | Estruturas de dados tipadas, eventos de diário de bordo, estado volátil em tempo real, tópicos WebSocket, eventos de diagnóstico restrito (`app.toggle_lock`, `app.set_auto_priorities`) e configurações (`auto_companion_priorities`, `auto_companion_linger_seconds`, etc.). |
| `crates/tmc-journal` | Resolução multi-SO de pastas (Win32 `FOLDERID_SavedGames`, Proton/Flatpak no Linux), rastreamento seguro por *byte offset* de `Journal.*.log` sem trancar arquivos em uso pelo jogo e leitura resiliente de `Status.json` + 9 arquivos satélites. |
| `crates/tmc-store` | Persistência em banco **SQLite (modo WAL)** via `rusqlite`, migrações de dados incrementais, histórico de exploração/comércio/compras, cofre local de senhas e auto-recuperação contra corrupção. |
| `crates/tmc-i18n` | Carregamento e interpolação de dicionários **Project Fluent** (`locales/{es,en,fr,de,pt}/*.ftl`), com baterias de testes garantindo paridade total de chaves em relação ao arquivo `es`. |
| `crates/tmc-net` | Clientes HTTP assíncronos equipados com limitação de taxa, memória temporária (cache) e operação offline: *Spansh* (Neutron Plotter, Exact Plotter `/api/generic/route`, busca de estaleiros/módulos `POST /api/stations/search`, anéis/corpos `/api/bodies/search` e busca por proximidade `/api/nearest`), *Ardent*, *Canonn*, *tick.edcd.io*, *EDSM*, *Inara InAPI v1* (requisições em lotes, máx. 2 por minuto) e *Frontier CAPI* (OAuth2 S256 PKCE). |
| `crates/tmc-eddn` | Transmissor HTTP POST despachando esquemas formais para a *EDDN* (`journal/1`, `commodity/3`, `outfitting/2`, `shipyard/2`) usando identificadores com salt criptográfico local, além de receptor ZeroMQ selecionando cotações no raio útil. |
| `crates/tmc-tts` | Fila inteligente de priorização com deduplicação e intervalos de espera (`TtsQueue`), suporte a voz do Windows (SAPI5 / OneCore `< 2 MB` RAM) e processo worker neural desacoplado (`Kokoro-82M` ONNX com suspensão por inatividade após 60 s). |
| `crates/tmc-modules` | Lógica de negócio para os 16 módulos do jogo (como `outfitting_catalog.json` em `ship` e o acervo `mining_hotspots.json` com atualização atômica em memória em `mining`), determinação de relevância com histerese (`linger_seconds`), gatilhos preditivos e transmissão para a UI. |
| `crates/tmc-core` | Núcleo de gerenciamento de estado, distribuidor de mensagens, gerador tático `VoiceAdvisor` e resolvedor de comandos de voz em múltiplos idiomas `VoiceIntentResolver` (9 intenções catalogadas). |
| `crates/tmc-server` | Servidor HTTP/REST e WebSocket (`/ws`) construído sobre **axum**, agrupamento de atualizações a cada 100 ms (máximo de 10 envios/s), criador do arquivo `.vap` para *VoiceAttack*, proteções contra DNS-rebinding/CSRF e chave de 128 bits para tráfego local (`HttpOnly; SameSite=Strict`). |
| `crates/tmc-app` | Montagem e orquestração de serviços, ciclo de inicialização, leitura histórica com controle de fluxo adaptativo e monitor de integridade. |
| `crates/tmc-replay` | Aplicativo CLI para execução simulada e acelerada de sessões gravadas (`play`) e higienização determinística de diários (`anonymize`) na criação de fixtures. |
| `apps/desktop` | Aplicação nativa empacotada em **Tauri 2** (`TMC.exe`), trava de instância única, salvamento da geometria de tela, operação na área de notificação e suspensão de memória do WebView2. |
| `apps/headless` | Instância autônoma de linha de comando para operação de retaguarda e avaliações matemáticas de precisão (`tmc-headless eval`). |
| `ui/` | Frontend dinâmico em **Svelte 5** (runes), TypeScript, identidade visual *TMC Glass HUD*, trava de tela instantânea (`Alt + L`), organizador de prioridades em tempo real, sintetizador Web Audio API, rolagem virtual de alta eficiência e reconhecimento de voz no navegador com WebAssembly (`Whisper` via `@huggingface/transformers`). |
| `locales/` | Coleções de tradução `.ftl` sincronizadas para `es` (base), `en`, `fr`, `de` e `pt`. |

---

## 8. Ingestão do Diário, Armazenamento, Segurança e Desempenho

- **Leitura não intrusiva orientada a deslocamento de bytes (*byte offset*):** gatilhos do sistema operacional são utilizados estritamente para apontar a criação de um arquivo `Journal.*.log` recente; a decodificação linha a linha evolui via ponteiro de bytes sem reter handles abertos do processo `EliteDangerous64.exe`. Documentos com alto dinamismo (`Status.json`, `Cargo.json`, `Market.json`, `Outfitting.json`, `Shipyard.json`, `ModulesInfo.json`, `NavRoute.json`, `Backpack.json`, `ShipLocker.json`, `FCMaterials.json`) aplicam até 10 tentativas com pausa de 50 ms para absorver gravações incompletas do jogo.
- **Tolerância e autocura:** trechos malformados em JSON ou novas tags que venham a surgir no diário jamais interrompem a aplicação. Na eventualidade de o `tmc.db` acusar danos na abertura, é criado um backup no formato `tmc.db.corrupt-YYYYMMDDHHMMSS`, uma base limpa é gerada e o histórico é reconstruído. Se o arquivo `config.toml` apresentar erros estruturais, ele é preservado em `config.toml.bak` e os parâmetros padrão de fábrica são restaurados.
- **Proteção do servidor de bordo:** opera por padrão exclusivamente amarrado a `127.0.0.1:47320` (testando a faixa `47320..=47340` em caso de conflito de portas), checando cabeçalhos `Host` e `Origin`. Ao habilitar `lan = true` (`0.0.0.0`), qualquer chamada externa a loopback obrigatoriamente exige o token de segurança aleatório de 128 bits presente no QR Code.
- **Teto rígido de consumo de hardware (Meta inegociável):** `< 150 MB` de memória RAM total com tela em primeiro plano (ficando habitualmente entre ~35 e 112 MB), `< 1%` de uso de CPU em repouso, ausência de loops contínuos de `requestAnimationFrame` e pico de 10 mensagens por segundo no WebSocket. Quando enviado para a bandeja no Windows, aciona `COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW` e `SetIsVisible(false)`.

---

## 9. Caminhos Locais, Argumentos de Linha de Comando e Variáveis de Ambiente

### Locais padrão do sistema
- **Diários de bordo:**
  - Windows: `%USERPROFILE%\Saved Games\Frontier Developments\Elite Dangerous\` (e esquema de cores do painel em `%LOCALAPPDATA%\Frontier Developments\Elite Dangerous\Options\Graphics\GraphicsConfigurationOverride.xml`).
  - Linux / SteamOS: prefixos Proton para o AppID `359320` em `~/.steam/steam/...`, `~/.local/share/Steam/...` ou `~/.var/app/com.valvesoftware.Steam/...`.
- **Arquivo de configurações:** `%APPDATA%\TMC\config.toml` (Windows) / `~/.config/tmc/config.toml` (Linux).
- **Banco SQLite e logs rotativos (retenção máxima de 7 dias):** `%LOCALAPPDATA%\TMC\tmc.db` e `tmc*.log` (Windows) / `~/.local/share/tmc/tmc.db` (Linux).

### Opções de inicialização via terminal (`tmc-desktop` e `tmc-headless`)
- `--journal-dir <caminho>`: força o diretório de leitura dos diários e arquivos de status.
- `--config <caminho>`: aponta para um `config.toml` alternativo.
- `--ui-dir <caminho>`: carrega os arquivos compilados da interface web de um endereço customizado.

### Variáveis de ambiente complementares
- `TMC_CAPI_CLIENT_ID`: código de aplicação OAuth2 registrado na área de desenvolvedores da Frontier para sobrepor o padrão embutido (nunca versione credenciais secretas no Git; utilize um arquivo `.env` local).
- `VOICEATTACK_PATH` / `VOICEATTACK_DIR`: endereço onde o *VoiceAttack* se encontra instalado, caso esteja fora dos diretórios habituais.

---

## 10. Ferramentas de Desenvolvimento, Reprodução e Auditoria Científica

```powershell
# 1. Iniciar servidor e regras de negócio sem janela gráfica (ótimo para servidores remotos ou trabalho de UI)
cargo run -p tmc-headless -- --journal-dir tests/fixtures/journal

# 2. Executar um diário gravado em velocidade acelerada (10x) gravando em pasta temporária
cargo run -p tmc-replay -- play --from tests/fixtures/journal/basic_session.log --to $env:TEMP\tmc-sim --speed 10

# 3. Remover dados sensíveis de um diário autêntico antes de torná-lo teste (limpa listas, censura chats e mascara CMDR/FID)
cargo run -p tmc-replay -- anonymize --from C:\Caminho\Journal.log --to tests\fixtures\journal\novo_fixture.log

# 4. Auditoria de dados matemáticos sem expor dados pessoais (8 testes oraculares + tempos de banco)
cargo run -p tmc-headless -- eval --timings

# 5. Averiguar consumo real de memória RAM e esforço de CPU em repouso contra os limites (< 150 MB RAM, < 1% CPU)
pwsh scripts/measure-footprint.ps1 -Exe target/release/TMC.exe -JournalDir tests/fixtures/journal
```

A diretiva `tmc-headless eval` realiza verificações matemáticas em diários autênticos preservando a privacidade para auditar o alcance FSD (`±0,05 al`), cotações da Universal Cartographics (`±5%`), estimativas da exobiologia com bônus `×5`, conciliação de abates em zonas de combate, taxas de extração em refinarias, rentabilidade comercial, contagem de baixas em pilhas de massacres e balanço de materiais de engenharia.

---

## 11. Compilação, Testes Automatizados e Empacotamento NSIS

### Ferramental necessário
- **Rust** estável (`1.90+`, edição 2024) com toolchain nativo (MSVC Build Tools no Windows; `libwebkit2gtk-4.1-dev` no Linux para o desktop).
- **Node.js** `22+` e `npm`.
- **Tauri CLI v2** (`cargo install tauri-cli@^2`) para compilar o executável de instalação.

### Procedimento completo de validação e compilação

```powershell
# 1. Instalar módulos da interface e compilar a apresentação web (Svelte 5 + Vite)
npm.cmd --prefix ui ci
npm.cmd --prefix ui run build

# 2. Testes e integridade de tipos da interface (TypeScript / svelte-check + Vitest)
npm.cmd --prefix ui run check
npm.cmd --prefix ui run test

# 3. Testes e checagem do backend em Rust (formatação, clippy sem pendências e > 1.400 testes no workspace)
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# 4. Modo de desenvolvimento dinâmico com hot-reload (proxy Vite -> 127.0.0.1:47320)
# Terminal 1: cargo run -p tmc-headless -- --journal-dir <caminho>
# Terminal 2: npm.cmd --prefix ui run dev

# 5. Compilar executáveis binários em modo Release otimizado
cargo build -p tmc-desktop --release
cargo build -p tmc-headless --release

# 6. Gerar instalador multi-idioma NSIS para ambiente Windows
Push-Location apps/desktop
cargo tauri build
Pop-Location
# Executável gerado: target/release/bundle/nsis/TMC_0.1.0_x64-setup.exe
```

> **Aviso de disciplina de compilação:** concentre todas as saídas no diretório canônico `target/` do projeto (nunca crie apontadores para `CARGO_TARGET_DIR` para não duplicar dezenas de gigabytes de arquivos intermediários).

---

## 12. Contribuição, Privacidade e Licença

- **Licença:** disponibilizado sob a **[PolyForm Noncommercial 1.0.0](LICENSE)** (concede permissão para leitura, auditoria, ajuste, compilação e distribuição do sistema sem custos para uso pessoal, educativo ou comunitário sem fins lucrativos). É terminantemente proibido incorporar códigos ou bases cobertos por licenças conflitantes como GPL ou AGPL; todas as dependências externas são registradas no arquivo [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
- **Diretrizes para envio de código:**
  - Nenhuma mensagem legível ou fala audível pode ser inserida diretamente no código-fonte: todas as strings devem ser catalogadas nos arquivos `locales/{es,en,fr,de,pt}/*.ftl` (`es` atua como matriz canônica checada por `cargo test -p tmc-i18n`).
  - Nunca envie arquivos de diário com dados reais de voo sem antes aplicar o utilitário `tmc-replay anonymize` acompanhado de revisão manual.
  - Siga com rigor os padrões de ergonomia definidos em [`docs/design-system-glass-hud.md`](docs/design-system-glass-hud.md).
- **Privacidade:** o TMC não possui telemetria oculta nem qualquer mecanismo de coleta analítica. Chaves de serviços externos ficam restritas ao seu banco SQLite pessoal no seu próprio disco rígido, a integração Frontier CAPI utiliza fluxo OAuth2 PKCE com o domínio oficial da Frontier Store e envios ao EDDN são protegidos por identificação seletiva com salt local.
- **Declaração legal:** *Elite Dangerous*, *Horizons* e *Odyssey* são marcas registradas pertencentes à Frontier Developments plc. O TMC é um desenvolvimento comunitário e autônomo, não vinculado e sem homologação oficial da Frontier Developments.
