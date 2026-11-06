# Revisión lingüística DE/PT — 2026-11-05

Revisor: reviewer-sonnet (skill review). Alcance leído: los 21 archivos de `locales/de` y los 21 de `locales/pt`, incluidos ambos `engineering-data.ftl` completos. Solo revisión; no se han modificado traducciones ni ejecutado compilaciones. Comparación con claves y significado de EN/ES; los nombres importados con `.pending = 1` se consideran deliberados.

## Veredicto

Traducciones funcionalmente comprensibles, pero hay errores confirmados de significado, restos de español en portugués, anglicismos evitables en texto de interfaz y faltas de acentuación. No se recomienda declarar la revisión limpia hasta corregirlos. La mezcla de variantes PT-PT/PT-BR y de tratamientos DE du/Sie se documenta como deuda estilística; no hace falta convertir globalmente de variante.

## Correcciones confirmadas

| Prioridad | Archivo | Clave | Actual | Propuesta |
|---|---|---|---|---|
| P1 | de/crime.ftl | crime-notoriety-decay-note | Requiere vuelo en `Hyperraum/Normalraum` | `Erfordert aktiven Flug im Supercruise/Normalraum (1 Stufe pro 2 Stunden)`; Hyperraum significa hyperspace y cambia la instrucción de origen |
| P1 | pt/engineering-data.ftl | modtype-hullreinforcementpackage | Pacote de Reforços de Casco de Meta Liga | Pacote de Reforço de Casco; el tipo genérico Hull Reinforcement Package no es la variante de metaaleación |
| P2 | pt/main.ftl | auto-companion-priorities-desc | ... `cuando` múltiplos eventos ... | Escolha qual atividade tem preferência quando múltiplos eventos ocorrem simultaneamente. |
| P2 | pt/main.ftl | location-home-base-cleared | Base de `operaciones` removida | Base de operações removida |
| P2 | pt/combat.ftl | combat-subsystem-cargohatch | Compartimento de carga | Escotilha de carga; hatch es la escotilla, no la bodega |
| P2 | de/combat.ftl | combat-pips-summary | ... MOT { $eng } ... | Energia de motores debe usar `ANT` como combat-pips-eng: `Energie: SYS { $sys }, ANT { $eng }, WAF { $wep } von 4` |
| P2 | pt/engineering.ftl | engineering-ready-percent | Materiais: { $pct } % pronto | Materiais: { $pct } % prontos |
| P2 | pt/ship_outfitting.ftl | outfitting-afford-tight | Recompra justa | Recompra no limite; justa significa fair y no comunica margen financiero estrecho |
| P2 | pt/main.ftl | ship-unladen-mass | Massa descarregada | Massa sem carga |
| P2 | pt/exobiology.ftl | exobiology-signals, rama one | 1 sinal biológico | { $count } sinal biológico; CLDR pt trata 0 en one |
| P2 | pt/mining_targets.ftl | mining-hotspots-count, rama one | 1 hotspot | { $count } hotspot; evitar 0→1 |
| P2 | pt/missions.ftl | missions-massacre-missing, rama one | falta 1 abate | falta { $kills } abate; evitar 0→1 |
| P2 | pt/engineering-data.ftl | exp-special_auto_loader | Carregador Automatico | Carregador Automático |
| P2 | pt/engineering-data.ftl | exp-special_concordant_sequence | Sequencia Concordante | Sequência Concordante |
| P2 | pt/engineering-data.ftl | exp-special_corrosive_shell | Projeticl Corrosivo | Projétil Corrosivo |
| P2 | pt/engineering-data.ftl | exp-special_phasing_sequence | Sequencia Faseada | Sequência Faseada |
| P2 | pt/engineering-data.ftl | exp-special_plasma_slug | Projetil de Plasma | Projétil de Plasma |
| P2 | pt/engineering-data.ftl | exp-special_plasma_slug_cooled | Projetil de Plasma | Projétil de Plasma |
| P2 | pt/engineering-data.ftl | exp-special_regeneration_sequence | Sequencia de Regeneração | Sequência de Regeneração |
| P2 | pt/engineering-data.ftl | exp-special_smart_rounds | Projetil Inteligente | Projétil Inteligente |

## Inglés residual y coherencia (P2/P3)

Estas correcciones son de texto de interfaz, no requieren inventar nombres oficiales del juego:

| Archivo | Clave | Actual | Propuesta |
|---|---|---|---|
| de/engineering.ftl | engineering-blocker-incompatible-item | Das Modul ist mit diesem Blueprint nicht kompatibel | Das Modul ist mit dieser Blaupause nicht kompatibel |
| de/engineering.ftl | engineering-blocker-unknown-blueprint | Unbekannter Blueprint | Unbekannte Blaupause |
| pt/engineering.ftl | engineering-col-blueprint | Blueprint | Projeto |
| pt/engineering.ftl | engineering-replaces-blueprint | O blueprint atual será substituído: começa do G1. | O projeto atual será substituído: começa no G1. |
| pt/engineering.ftl | engineering-blocker-incompatible-item | ... este blueprint | O módulo não é compatível com este projeto |
| pt/engineering.ftl | engineering-blocker-unknown-blueprint | Blueprint desconhecido | Projeto desconhecido |
| pt/engineering.ftl | engineering-col-rank | Rank | Nível |
| pt/accounts.ftl | accounts-eddn-uploader-id | Uploader ID (Pseudonimizado) | ID do remetente (pseudonimizado) |
| de/mining.ftl | mining-limpets | Limpets | Drohnen |
| de/mining.ftl | mining-limpets-left | noch 1 Limpet / noch { $count } Limpets | noch { $count } Drohne / noch { $count } Drohnen |
| pt/mining.ftl | mining-limpets | Limpets | Drones |
| pt/mining.ftl | mining-limpets-left | resta { $count } limpet / restam { $count } limpets | resta { $count } drone / restam { $count } drones |
| de/ship_outfitting.ftl | outfitting-afford-ok | Rebuy OK | Rückkauf gesichert |
| de/ship_outfitting.ftl | outfitting-afford-tight | Rebuy knapp | Rückkauf knapp |
| de/ship_outfitting.ftl | outfitting-afford-tight-tooltip | ... Fliegen Sie nie ohne Rebuy! | ... Fliegen Sie nie ohne Rückkaufreserve! |
| pt/navigation.ftl | nav-plan-refuel-scoopable | Reabastecer em estrelas com scoop | Reabastecer em estrelas adequadas |
| de/navigation.ftl | nav-plan-refuel-scoopable | An Scoop-Sternen tanken | An geeigneten Sternen tanken |
| pt/tts.ftl | tts-nav-jump-scoopable | ... Estrela coletável. | Salto concluído para { $system }. É possível coletar combustível desta estrela. |
| pt/tts.ftl | tts-nav-jump-unscoopable | ... estrela não coletável. | Salto concluído para { $system }. Aviso: não é possível coletar combustível desta estrela. |
| de/tts.ftl | tts-nav-jump-scoopable | ... Stern schöpfbar. | Sprung nach { $system } abgeschlossen. Treibstoffsammeln an diesem Stern möglich. |
| de/tts.ftl | tts-nav-jump-unscoopable | ... Stern nicht schöpfbar. | Sprung nach { $system } abgeschlossen. Warnung: Treibstoffsammeln an diesem Stern nicht möglich. |

## Términos que no deben corregirse sin comprobar la fuente

- Nombres de blueprints, experimentales y módulos con `.pending = 1`: fallback inglés intencionado, no un fallo de Fluent.
- Todos los rangos (`combat-rank-*`, `trade-rank-*`, `explore-rank-*`, `soldier-rank-*`, `exobiologist-rank-*`, `cqc-rank-*`, `federation-rank-*`, `empire-rank-*`) siguen en inglés en DE/PT. El estado `None` sí es texto genérico traducible: `Kein Rang` / `Sem patente`. Para los demás se recomienda fuente oficial antes de hacer pasar traducción propia por nombre canónico.
- La guía inserta rangos/reputaciones ingleses y nombres de recursos. Traducir los nombres confirmados por el catálogo, o conservar original entre paréntesis si se añade explicación; no sustituir facciones, estaciones, ingenieros ni Odyssey/Powerplay/Stronghold por traducciones inventadas.
- PT `exp-special_powerplant_highcharge = Parrudo` coincide con Double Braced aunque EN dice Monstered. Candidato de mapeo, no cambio confirmado: comprobar fuente antes de modificar.
- PT `exp-special_powerdistributor_fast = SuperConduites`: grafía importada dudosa, verificar nombre oficial; no imponer por intuición.
- Mezcla PT-BR/PT-PT existente (`você`/`tu`, `salvar`/`guardar`, `nêutrons`/`vaivéns`) y DE du/Sie: coherencia de estilo pendiente de política editorial, no un bloqueo de comprensión ni permiso para cambiar variante globalmente.

## Pruebas recomendadas y aceptación

1. Parsear los 105 FTL; claves, atributos, variables y selectores compatibles con ES.
2. Resolver todos los plurales con 0, 1 y 2 en PT y FR: la cantidad renderizada debe ser la recibida.
3. Probar las cuatro etiquetas de ingeniería que dejan Blueprint y los errores de bloqueo.
4. Regenerar/importar donde haya fuente activa para evitar reintroducir textos corregidos; mantener marcadores pending y procedencia oficial.
5. UI check/test/build; suites requeridas por AGENTS antes de commit.
6. Revisar visualmente la guía en alemán/portugués con textos largos y anchura estrecha después de cambios de redacción.

Criterio de aceptación: no quedan errores confirmados arriba; ninguna variable/nombre propio se pierde; `0` nunca se convierte en `1`; no se declara nombre oficial una traducción sin fuente.
