# Revisión lingüística español e inglés

- Fecha: 2026-11-05.
- Alcance leído: los 21 archivos de `locales/es/`, sus 21 equivalentes ingleses y `scripts/engineer-guide-content.mjs` (incluidas las 181 claves `unlock-*` generadas por idioma).
- Veredicto: traducción comprensible en general, pero necesita correcciones de terminología y eliminación de anglicismos de interfaz. No se han modificado recursos ni ejecutado compilaciones.
- Los nombres propios de sistemas, facciones, talleres, productos, servicios y programas no son errores de traducción por sí mismos. Los nombres ingleses marcados `.pending = 1` están documentados deliberadamente como pendientes; no deben sustituirse por una traducción inventada presentada como oficial.

## Hallazgos priorizados

### P2 · Traducciones que alteran o dificultan identificar el concepto

| Archivo | Clave | Actual | Propuesta |
|---|---|---|---|
| es/bgs.ftl | bgs-controlling-state | Estado de gobierno | Estado de la facción |
| es/navigation.ftl | nav-plan-refuel-scoopable | Repostar en estrellas escoop | Repostar en estrellas compatibles |
| es/exploration.ftl | exploration-mark-scoopable | Recogible | Permite repostar |
| es/crime.ftl | crime-stealth-advice-step1 | …acércate al buzón desde el frente. | …acércate a la boca de entrada desde el frente. |
| es/crime.ftl | crime-stealth-advice-step2 | Activa Carrera Silenciosa (Silent Running) o usa Eyector Térmico (Heatsink) a menos de 4 km. | Activa la navegación silenciosa o lanza un disipador térmico a menos de 4 km de la estación. |
| es/ship_outfitting.ftl | outfitting-filter-carriers | Portaaviones privados | Portanaves privados |
| es/powerplay.ftl | powerplay-pledged-to | Jurado a | Afiliado a |
| es/missions.ftl | panel-missions-massacre | Pila de masacre | Misiones de masacre acumuladas |
| es/engineering-data.ftl | hint-hge | Sitios de señales de alto interés (HGE) | Emisiones de alto grado (HGE) |
| en/main.ftl | alert-module-crashed | Module { $module } has crashed | Module { $module } has failed |
| en/layout.ftl | layout-device-claim | I am an existing profile | Use an existing profile |

`bgs-controlling-state` muestra el estado de la facción y tiene otra columna independiente para el gobierno. `exploration-mark-scoopable` señala disponibilidad para recolectar combustible de la estrella, no que el astro sea recogible. Los dos textos de `crime` deben concordar con `flag-silent-running` y los términos del resto de la UI. No se propone modificar las reglas de juego ni los consejos tácticos como parte de esta revisión de idioma.

### P2 · Rangos y estados ingleses sin política explícita de localización

- `es/main.ftl`: `explore-rank-*`, `soldier-rank-*`, `exobiologist-rank-*`, `cqc-rank-*`, `federation-rank-*` y `empire-rank-*` conservan todos los valores ingleses.
- `es/combat.ftl`: `combat-rank-*` conserva los valores ingleses.
- `es/trade.ftl`: `trade-rank-*` conserva los valores ingleses.
- `federation-rank-0 = None` y `empire-rank-0 = None` son ausencia de rango, no nombres propios: se pueden corregir directamente a `Sin rango`.
- La guía generada inserta sin localizar `Merchant`, `Competent`, `Scout`, `Surveyor`, `Expert`, `Dealer`, `Dangerous`, `Midshipman`, `Outsider`, `Friendly`, `Allied`, `Cordial` y `Unfriendly` dentro de frases españolas.
- `scripts/engineer-guide-content.mjs` documenta que conserva nombres canónicos ingleses de los objetos. Por eso las entregas incluyen `Gold`, `Bromellite`, `Landmines`, `Meta-Alloys`, datos Odyssey, etc. Es una limitación deliberada, no un fallo de paridad entre claves.

Recomendación: usar nombres locales contrastados en datos autorizados del repositorio; cuando falte evidencia oficial, conservar el nombre inglés como identificación y añadir explicación local sin presentarla como traducción oficial. Ejemplo de redacción descriptiva segura: «Alcanza el rango de exploración Scout o superior» ya traduce la instrucción y conserva el identificador; puede acompañarse de una indicación local de traducción pendiente. No inventar equivalencias de rangos navales ni traducir facciones/sistemas.

### P3 · Anglicismos de interfaz y coherencia de glosario

| Archivo | Clave(s) | Actual | Propuesta |
|---|---|---|---|
| es/accounts.ftl | accounts-eddn-uploader-id | Uploader ID (Seudonimizado) | ID de remitente (seudonimizado) |
| es/accounts.ftl | accounts-eddn-connected | Conectado al Gateway | Conectado a la pasarela |
| es/accounts.ftl | accounts-inara-desc | …y Community Goals. | …y objetivos comunitarios. |
| es/accounts.ftl | accounts-inara-api-key-label | InAPI Key personal | Clave InAPI personal |
| es/accounts.ftl | accounts-inara-whitelist-badge | Whitelist oficial | Autorización oficial |
| es/accounts.ftl | accounts-capi-empty-hint | snapshot de naves | inventario de naves |
| es/main.ftl | settings-online-services | Consultar servicios online… | Consultar servicios en línea… |
| es/main.ftl | settings-feedback-title | Soporte, Feedback y Comunidad | Soporte, sugerencias y comunidad |
| es/engineering.ftl | engineering-col-blueprint | Blueprint | Plano |
| es/engineering.ftl | engineering-replaces-blueprint | Se sustituye el blueprint actual… | Se sustituye el plano actual… |
| es/engineering.ftl | engineering-blocker-incompatible-item | …compatible con este blueprint | …compatible con este plano |
| es/engineering.ftl | engineering-blocker-unknown-blueprint | Blueprint desconocido | Plano desconocido |
| es/engineering.ftl | engineering-source-disabled | Servicios online desactivados | Servicios en línea desactivados |
| es/engineering.ftl | engineering-broker-nearest | Broker de tecnología más cercano… | Intermediario tecnológico más cercano… |
| es/engineering-data.ftl | hint-broker | Broker de tecnología | Intermediario tecnológico |
| es/mining.ftl | mining-limpets | Limpets | Drones |
| es/mining.ftl | mining-limpets-left | queda 1 limpet / quedan … limpets | queda 1 dron / quedan … drones |
| es/mining.ftl | mining-col-pad | Pad | Plataforma |
| es/mining_targets.ftl | mining-min-hotspots | Hotspots mínimos… | Zonas de concentración mínimas… |
| es/mining_targets.ftl | mining-hotspots-count | hotspot / hotspots | zona de concentración / zonas de concentración |
| es/mining_targets.ftl | mining-offline-catalog | Catálogo offline curado (TMC) | Catálogo local revisado (TMC) |
| es/missions.ftl | missions-cabin-business | Business | Ejecutiva |
| es/ship_outfitting.ftl | outfitting-col-pad | Pad | Plataforma |
| es/ship_outfitting.ftl | outfitting-module-search-placeholder | …Frame Shift Drive, Multi-Cannon… | …motor de distorsión, multicañón… |
| es/ship_outfitting.ftl | textos stock | stock | existencias |
| es/voice_assistant.ftl | voice-assistant-engine-whisper | …100% offline… | …100 % sin conexión… |
| es/exobiology.ftl | exobiology-waiting | Esperando ScanOrganic… | Esperando una muestra biológica… |

Los nombres `Fleet Carrier`, `Stronghold`, `Interstellar Factors`, `Powerplay`, `VoiceAttack`, `Whisper`, `Merc Coins`, `Apex` y las siglas técnicas se pueden conservar si existe una política de producto explícita; conviene aplicarla igual en cabeceras, badges y tooltips. Si se cambia `Fleet Carrier` a `portanaves`, debe hacerse de forma consistente, incluidas locuciones y el módulo de finanzas. No es necesario traducir `app-name` ni las marcas.

### P3 · Inglés heredado de nombres fuente

`en/engineering-data.ftl` contiene `bp-shieldcellbank_rapid = Rapid Charge ShieldCellBank Bank` y `bp-shieldcellbank_specialised = Specialised ShieldCellBank Bank`. La repetición `Bank` y el identificador CamelCase son artefactos de la fuente EDDiscovery, también en el catálogo JSON y sus aliases. Se recomienda normalizar únicamente la etiqueta visible a `Rapid Charge Shield Cell Bank` y `Specialised Shield Cell Bank`, conservando los aliases originales de compatibilidad. Cualquier normalización debe mantenerse reproducible al regenerar catálogos. No modificar snapshots fijados ni atribuciones de nombres oficiales.

## Pruebas recomendadas

1. Paridad de claves, ausencia de duplicados, parsing Fluent y equivalencia de variables entre idiomas.
2. Tests de textos representativos para `None` traducido, glosario de ingeniería, repostaje y tipología de portanaves.
3. Regeneración del contenido de la guía y `--check` del importador: las correcciones no deben desaparecer al regenerar.
4. Revisión visual con español y textos largos; comprobar plataformas, etiquetas de cabecera, badges de autorización y la guía a anchura reducida.
5. Mantener la excepción documentada de `.pending = 1`; la simple igualdad con inglés no basta para declarar un texto sin traducir.

## Criterios de aceptación

- Corregidos los errores P2 de significado sin alterar reglas de juego.
- Terminología de interfaz uniforme y en español, conservando identificadores propios verificables.
- Ningún rango o nombre de juego inventado presentado como oficial.
- Variables, cantidades, condiciones y nombres de destinatarios conservados en la guía.
- Catálogos reproducibles y validaciones de Fluent/UI en verde.
