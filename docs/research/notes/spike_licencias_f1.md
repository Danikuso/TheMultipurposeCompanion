# Spike 2 de F1: licencias de los datos estáticos (FSD, especies, reglas de exobiología, sondas)

Fecha: 2026-10-17. Todas las licencias se han verificado el 2026-10-17 contra fuentes primarias (archivos LICENSE/README en `raw.githubusercontent.com`, API de GitHub, cabecera de edsy.org y `api.php?meta=siteinfo` de Fandom). Esto es una **decisión de proyecto, no asesoría legal**.

## 1. Candidatas

| Fuente | Licencia verificada | Evidencia | Datos que aportaría | Compatible con PolyForm NC |
|---|---|---|---|---|
| coriolis-data (EDCD) | Los datos y JSON son «propiedad intelectual y copyright de Frontier Developments» y quedan sujetos a sus términos; solo el código JS/CSS/HTML/SVG de Coriolis es MIT. GitHub: `NOASSERTION` | [LICENSE.md](https://raw.githubusercontent.com/EDCD/coriolis-data/master/LICENSE.md) | FSD por clase/calificación: masa óptima, combustible máx., coeficiente, exponente, SCO/Mk II, booster Guardian | **No**: no hay licencia sobre los datos |
| coriolis (EDCD, código) | MIT solo para el código; mismo aviso de datos de Frontier | [LICENSE.md](https://raw.githubusercontent.com/EDCD/coriolis/develop/LICENSE.md) | Nada de datos | El código MIT sí, pero no lo necesitamos |
| EDSY (EDCD/EDSY, edsy.org) | Código/diseño: CC BY-NC 4.0 (taleden). Los datos de juego «siguen siendo propiedad de Frontier», usados con autorización de Frontier para EDSY. Repo sin archivo LICENSE | Comentario de cabecera de [edsy.org](https://edsy.org/); [README del repo](https://github.com/EDCD/EDSY) | Mismas tablas de módulos | **No** para copiar: CC BY-NC es CC-BY (regla del propietario) y la autorización de Frontier es para EDSY, no transferible |
| Wiki de Fandom (Elite Dangerous Wiki) | CC-BY-SA (`rightsinfo` del propio wiki, enlazando a fandom.com/licensing) | [API siteinfo](https://elite-dangerous.fandom.com/api.php?action=query&meta=siteinfo&siprop=rightsinfo&format=json) | Tablas de FSD, valores de especies, texto | Requiere aprobación del propietario; ShareAlike obligaría a relicenciar derivados, **choca con PolyForm NC** |
| Canonn (CAPIv2-Data, CAPIv2-Docs, EDMC-Canonn) | GPL-3.0 (CAPIv2-Data y otros, según GitHub). Algunos repos (CanonnApi, canonn-api, Valknut) son MIT/Apache-2.0, pero son código, no el dataset de códex | [LICENSE de CAPIv2-Data](https://raw.githubusercontent.com/canonn-science/CAPIv2-Data/master/LICENSE) (GPL v3); [LICENSE de canonn-api](https://raw.githubusercontent.com/canonn-science/canonn-api/master/LICENSE) (Apache 2.0) | `codexRef`, especies, géneros, ubicaciones | **No** (GPL prohibida por AGENTS.md §6). Consumir su API por red (si se decide en el futuro) es otro asunto y no se hace en F1 |
| `codexRef` / reglas de SrvSurvey | Repo SrvSurvey: GPL-3.0 | [LICENSE](https://raw.githubusercontent.com/njthomson/SrvSurvey/main/LICENSE) (GPL v3) | Valores, CCR y reglas de predicción por especie | **Excluido sin excepción** (AGENTS.md §6). Cierra el «MIT-style por comprobar» de `player_roles_domain.md` §2: es GPL-3.0 |

Nota: en ningún caso hay datos con licencia MIT/BSD/Apache/CC0 que cubran las tablas necesarias. Lo más cercano a datos abiertos es CC-BY-SA (Fandom), que se descarta por la regla del propietario.

## 2. Razonamiento sobre «hechos»

Cifras de juego (masa óptima de un FSD, valor en créditos de una especie, distancia de clon) son hechos del juego y en general no son protegibles por derechos de autor; lo que puede estar protegido es la **compilación** (selección, orden, estructura) y los textos. Criterio conservador del proyecto: no copiar archivos ni tablas de ninguna fuente de la sección 1; reunir los valores a mano o con herramientas propias desde fuentes primarias (juego, Frontier, nuestro propio historial de diarios), con estructura propia, y **verificarlos** contra datos observables. Ante la duda, construir desde hechos y preguntar al propietario (AGENTS.md §6).

## 3. Decisión por archivo

| Archivo | Decisión | Cómo se construye y verifica |
|---|---|---|
| `fsd.json` | **Se construye desde hechos y se verifica contra el historial.** Debe incluir `fuel_power` y `fuel_multiplier` (spike de Spansh §4), además de masa óptima, combustible máx. por salto, y variantes SCO/Mk II y booster Guardian | Valores del juego (taller/Frontier) transcritos por nosotros. Verificación: el evento `Loadout` da `MaxJumpRange` y los `FSDJump` dan `FuelUsed`/`JumpDist`; un test calcula el alcance con la fórmula y la tabla y exige coincidencia con el historial real (anonimizado). Una clase/calificación sin verificar queda marcada como no verificada y el modo «Galaxia» se desactiva para ese FSD |
| `species.json` | **Se construye desde hechos y se verifica contra el historial** (nombres de género/especie, valor, CCR). Aprobación del propietario necesaria solo si se quisiera usar el wiki (CC-BY-SA) | Nombres y valores de `CodexEntry`/`ScanOrganic`/`SellOrganicData` (campos `Genus_Localised`, `Species_Localised`, `Value`, `Bonus`) del propio diario y fixtures anonimizados; CCR y valores contrastados con lo observado en el juego. Mejor opción por cobertura sería el wiki: **requiere aprobación del propietario** (se registra en Task 3); alternativa ya definida: esta |
| `bio_rules.json` | **Se construye desde hechos y se verifica contra el historial** | Reglas de aparición (atmósfera, gravedad, temperatura, vulcanismo, tipo de estrella) derivadas de escaneos propios (`Scan` + `ScanOrganic`) y de documentación primaria. Las reglas de SrvSurvey/Canonn quedan excluidas (GPL). Cobertura parcial permitida: lo no verificado se muestra como «sin predicción» |
| `probes.json` | **Se construye desde hechos y se verifica contra el historial** | Datos mínimos (alcance/propiedades de sondas y escáneres) tomados de las propiedades en juego y del evento `Loadout`; sin fuente externa |

## 4. Asuntos para el propietario

- **Requiere aprobación del propietario (a registrar en Task 3):** usar la wiki de Fandom (CC-BY-SA) como fuente directa de `species.json` o `fsd.json`. Recomendación: **no**, por ShareAlike frente a PolyForm NC; seguir con construcción desde hechos.
- Ninguna otra fuente es utilizable sin riesgo: coriolis-data y EDSY son datos de Frontier sin licencia abierta; Canonn/SrvSurvey son GPL-3.0.

## 5. Riesgos y deuda

- La verificación de `fsd.json` por historial depende de tener diarios con cada combinación clase/calificación; las demás quedarán «no verificadas».
- Los términos de uso de contenido de Frontier (<https://forums.frontier.co.uk/threads/elite-dangerous-media-usage-rules.510879/>) no se han analizado en este spike; si F1 incorpora cifras de juego, conviene que el propietario confirme que son aceptables.
- Los valores de especies cambian con parches; `species.json` debe llevar versión y fecha.
