# Spike de datos y licencias para F4 (Task 0)

Fecha: 2026-10-21. Alcance: decidir A (dataset comunitario con licencia de datos compatible con PolyForm Noncommercial 1.0.0) o B (recopilación propia de hechos del juego) por archivo de datos. Verificado con la API de GitHub y los ficheros raw; no se ha copiado ningún dato al repositorio.

## 1. Fuentes revisadas

### 1.1 coriolis-data (EDCD)

| Campo | Valor |
|---|---|
| Repo | https://github.com/EDCD/coriolis-data (rama `master`, commit `0db9234b5b9ce8c939ea84133d7ce336eea88e27`, 2026-04-24) |
| Licencia del **dato** | **Ninguna abierta.** `LICENSE.md`: «All Data and associated JSON files are intellectual property and copyright of Frontier Developments plc … subject to their terms and conditions». Solo el código JS/CSS/HTML/SVG de Coriolis es MIT. https://github.com/EDCD/coriolis-data/blob/master/LICENSE.md |
| Cobertura | `modifications/blueprints.json` (blueprints por grado con `components` = materiales y `features`), `modifications/specials.json` (experimentales con `components`), `modules.json`, `ships`. Sin ingenieros ni tasas del comerciante |
| Nombres localizados | No (solo inglés) |

### 1.2 EDEngineer (msarilar)

| Campo | Valor |
|---|---|
| Repo | https://github.com/msarilar/EDEngineer (commit `2cadd66d19509221ea2e5c2c132b8f04dc8167c8`, 2024-03-22; sin actividad posterior, probablemente desfasado) |
| Licencia | MIT (fichero `LICENSE`, © 2016 Max). Es licencia del repositorio; los datos `EDEngineer/Resources/Data/*.json` no declaran licencia propia y son hechos de Frontier recopilados comunitariamente. `blueprints.json` incluye `CoriolisGuid`, es decir, deriva de coriolis-data (IP de Frontier). Procedencia de las tablas no documentada |
| Cobertura | `blueprints.json`: 1172 entradas, 63 tipos de módulo, 259 nombres de blueprint; grados 1–5 (183/192/192/171/169) más 265 sin grado (módulos de tecnología/broker, `Engineers: ["@Technology"]`); ingredientes con cantidades; ingenieros por blueprint (25 nombres). `entryData.json`: 371 materiales (Material 92, Data 47, Commodity 31, OdysseyIngredient 201) con rareza, grupo y «OriginDetails» (pistas de obtención). Experimentales: solo parcialmente (p. ej. «Drag Munition», «Auto Loader» aparecen como nombres), sin estructura propia. Sin sistema, grado máximo ni requisitos de desbloqueo de ingenieros; sin tasas del comerciante |
| Nombres localizados | `localization.json` (889 entradas, 13+ idiomas). Cobertura medida sobre tipos de módulo / nombres de blueprint / materiales: es 62/63, 227/259, 367/369; fr 62/63, 212/259, 358/369; de 62/63, 229/259, 367/369; pt 54/63, 118/259, 166/369. Son traducciones comunitarias de la app, no un volcado verificado del juego. El JSON tiene un error de sintaxis (clave `Boron`, `"pl": "Boron,`) |

### 1.3 FDevIDs (EDCD)

| Campo | Valor |
|---|---|
| Repo | https://github.com/EDCD/FDevIDs (commit `c35612952dd6a547d1a7ac4cffab9c7051e86579`, 2026-09-05) |
| Licencia del dato | **Ninguna declarada** (la API de licencias devuelve 404; sin fichero LICENSE; README sin cláusula). Sin licencia no hay permiso de reutilización |
| Cobertura | `material.csv` (id, símbolo, rareza, tipo, categoría, nombre en inglés), `engineers.csv` (id, `system_address`, `market_id`, nombre; 38 filas), `microresources.csv`, `outfitting.csv`. Sin blueprints, experimentales, grados máximos, requisitos ni tasas |
| Nombres localizados | No (inglés) |

### 1.4 Otros

- **EDDN** (BSD-3-Clause): código y esquemas, no contiene tablas de blueprints.
- **Wikis (Fandom, Inara)**: contenido CC BY-SA o propietario; excluidos (política de licencias del proyecto; SrvSurvey y plugins EDMC GPL también excluidos).
- **Diario del juego (fuente primaria, hecho del juego)**: `Materials` / `MaterialCollected` traen `Name` (clave interna) y `Name_Localised` en el idioma del juego del jugador; `EngineerCraft` trae `BlueprintName` (clave interna, p. ej. `FSD_LongRange`), `Level`, `ExperimentalEffect` y `ExperimentalEffect_Localised`, `Engineer`, `Ingredients[]`; `EngineerProgress` trae ingeniero, `Progress` y `Rank`. Los nombres de blueprint **no** vienen localizados en el diario (solo la clave interna).
- **Localización oficial publicada de blueprints/experimentales (es/fr/de/pt)**: no se ha encontrado ningún archivo público con licencia. Los textos del juego son IP de Frontier y viven dentro del cliente; no se pueden redistribuir.

## 2. Decisión por archivo

Criterio (Task 0): A solo con licencia del dato claramente compatible y sin SA/copyleft. Ninguna fuente lo cumple (coriolis-data: IP de Frontier explícita; FDevIDs: sin licencia; EDEngineer: MIT del repo pero procedencia del dato dudosa y derivada de Coriolis). Con duda, B.

| Archivo | Decisión | Justificación |
|---|---|---|
| `materials` | **B** | Lista de ~170 materiales (Raw/Manufactured/Encoded, grado 1–4, categoría, familia) recopilada a mano desde el juego y el diario (`Name`, `Name_Localised`, `Category`). Se pueden contrastar identificadores con FDevIDs/EDEngineer sin copiar tablas |
| `blueprints` (módulos de nave G1–G5, materiales por grado) | **B** | Recopilar como hechos del juego (pantalla del ingeniero, notas de parche); la clave interna se confirma con `EngineerCraft.BlueprintName`. No copiar `blueprints.json` de Coriolis/EDEngineer |
| `experimentals` | **B** | Ídem; la clave es `ExperimentalEffect` del diario |
| `engineers` (sistema, grados máx., requisitos de desbloqueo) | **B** | Ninguna fuente tiene requisitos ni grados máx.; son hechos del juego. El sistema se verifica con `EngineerProgress` y `system_address` de FDevIDs solo como contraste |
| `trader_rules` (tasas del comerciante de materiales/broker) | **B** | Ninguna fuente lo cubre; son reglas del juego verificables in-game |
| `hints` (pistas de obtención) | **B** | `OriginDetails` de EDEngineer es texto comunitario de procedencia incierta; redactar texto propio y breve |
| Nombres por idioma | **B** (inglés + `_Localised` del diario) | Ver §3 |

## 3. Nombres oficiales es/fr/de/pt

Procedimiento propuesto (B):
1. Fuente de verdad en inglés: clave interna → nombre en inglés recopilado a mano.
2. Para materiales y experimentales, usar en ejecución `Name_Localised` / `ExperimentalEffect_Localised` del diario del jugador (nombre oficial en el idioma de su juego, sin redistribuir nada).
3. Para nombres estáticos por idioma, el propietario captura en el juego (cambiando el idioma del cliente) los nombres de blueprint y experimentales, que se anotan en `locales/{es,en,fr,de,pt}/*.ftl`. Mientras tanto, cada clave sin traducción se marca pendiente y la UI cae a inglés (spec §9).
4. Los nombres de EDEngineer (comunitarios) no se copian: solo sirven para contrastar capturas propias.

## 4. Texto para THIRD_PARTY_NOTICES.md

No procede (decisión B en todos los archivos; no se incorpora código ni datos de terceros). Si el propietario aprueba usar alguna fuente, borrador:

> ### EDEngineer (datos de materiales)
> - Fuente: https://github.com/msarilar/EDEngineer, commit 2cadd66d19509221ea2e5c2c132b8f04dc8167c8
> - Licencia: MIT (© 2016 Max)
> - Usada por: tmc-modules (materials)

## 5. Preguntas abiertas para el propietario

1. ¿Acepta B para todos los archivos (recopilación propia a mano, con contraste puntual en el juego)? Es el volumen principal de trabajo de las Tasks 8–11.
2. ¿Acepta usar el repositorio MIT de EDEngineer como A para `materials` y nombres (es/fr/de), asumiendo que el MIT cubre el repo aunque los datos deriven de Frontier/Coriolis? Recomendación: no.
3. Capturas del juego en es/fr/de/pt de blueprints y experimentales: ¿las aporta el propietario o quedan «pendientes» (inglés) en F4?
4. Se asume que los hechos del juego (cantidades por grado) no son copyrightables pero el texto/orden de tablas ajenas sí; ¿conforme?
