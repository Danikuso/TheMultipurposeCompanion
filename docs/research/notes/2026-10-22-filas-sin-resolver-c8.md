# Filas `unresolved` del importador de ingeniería (C8): evidencia y recomendación

Fecha: 2026-10-22. Contexto: la auditoría C1 dejó 8 filas de nave sin resolver del snapshot fijado de EDDiscovery (`7d30920`, `eddiscovery-RecipesEngineering.cs`). Esta nota solo registra hechos de juego con cita; no se copia código ni tablas con licencia incompatible (política de licencias del proyecto).

## Resumen de recomendaciones

| Filas | Recomendación | Confianza |
|---|---|---|
| `detailedsurfacescanner_longrange` G2–G5 (líneas 494–497) | **Remapear a `SurfaceScanner`** (Detailed Surface Scanner), conservando ingredientes, ingeniero (Etienne Dorn) y Merc Coins (5/5/10/20). Marcar el blueprint como «Merc» (solo aplica a la variante Merc del DSS). | Módulo: **alta**. Grados/ingredientes exactos: **media-baja** (sin segunda fuente). |
| `special_weapon_toughened` (1377), `special_weapon_efficient` (1388), `special_weapon_damage` (1407), `special_weapon_lightweight` (1429) | **Remapear a 11 tipos de arma de nave**: `BeamLaser, BurstLaser, PulseLaser, Cannon, Multi_Cannon, FragmentCannon, PlasmaAccelerator, RailGun, MissileRack, TorpedoPylon, MineLauncher`. No son duplicados, ni de traje: son filas de nave genuinas. | Lista de 8 (láseres, cañones, plasma, rail, fragment): **alta**. `MissileRack`, `TorpedoPylon`, `MineLauncher`: **media**. Tipos nuevos (Seeker, Enzyme, Shock, etc.): **sin evidencia → no incluir**. |

## 1. Detailed Surface Scanner: `detailedsurfacescanner_longrange`

### Hechos y fuentes
- El enum de EDDiscovery marca `DiscoveryScanner` como «primer tipo **no comprable**» (`eddiscovery-Modules.cs` líneas 486–487, 515). No es un módulo concreto: es un marcador del enum. El módulo correcto es `SurfaceScanner` (línea 395), que es el que usa el propio snapshot para `sensor_expanded` y para `int_detailedsurfacescanner_tiny`.
- Las 4 filas llevan además el último argumento 5/5/10/20, que el constructor de EDDiscovery guarda como `MercCoins` (líneas 37–61). El snapshot hace lo mismo con el blueprint Merc «Scoop rate enhanced» (`fuelscoop_efficiency`, líneas 613–617), asociado al tipo estándar `FuelScoop`. Por tanto el patrón de la fuente para blueprints Merc es «tipo de módulo estándar + Merc Coins».
- El blueprint es real y de Merc: el artículo de Frontier/EDFieldManual sobre Merc Coin dice que se ganan en Operations y se gastan en módulos MercGear y blueprints de ingeniería especializados, que el catálogo MercGear inicial incluye un «Long Range Detailed Surface Scanner», y que un blueprint Merc solo aplica a módulos Merc compatibles (no convierte un módulo estándar en Merc). Ver: https://edfieldmanual.com/wiki/Merc_Coin (accedido 2026-10-22).
- Inara lista la página «Detailed Surface Scanner – Long range detailed – blueprint» (https://inara.cz/elite/blueprint/205/), con Etienne Dorn como ingeniero y costes de 10 y 20 Merc Coins visibles en los grados altos (según el extracto del buscador, accedido 2026-10-22). La página devolvió 503 al intentar leerla directamente (Retry-After 3600 s), así que **no se pudo verificar fila por fila**.
- Notas de versión que introducen Merc Engineering/Merc Modules: Operations (4.4.0.0), https://www.elitedangerous.com/news/updates/4-4-0-0 (no accesible, 403) y 4.4.0.3 (julio 2026), citada por el buscador. Solo para contexto.
- EDEngineer 2cadd66 (2024-03-22, MIT, anterior a Merc) y coriolis-data (comparación) **no** contienen este blueprint: para el DSS solo existe `Sensor_Expanded` («Expanded Probe Scanning Radius»), y los ingredientes difieren de las filas Merc (G2: `1MS,1Ge` frente a `1MS,1Ge,1MCF`). Es decir, es un blueprint distinto y nuevo, no un duplicado de `sensor_expanded`.
- La última versión de EDDiscovery (`EliteDangerousCore` master, 2026-10-22 09:30Z) tiene el commit `7d30920`, el mismo que el snapshot fijado: **no hay corrección posterior** de estas filas.

### Coincidencia de grados e ingredientes
- Grados 2–5 presentes en el snapshot; falta el G1 (consistente con el blueprint Merc de la fuente; no hay evidencia de un G1). Sin segunda fuente verificable de ingredientes (Inara 503, wiki Fandom 402/403).
- Las filas tienen la misma forma que otros blueprints Merc (monedas crecientes con el grado), lo que apoya que son datos de juego y no un error.

### Recomendación por fila (G2, G3, G4, G5)
Remapear las 4 a `SurfaceScanner` como blueprint `detailedsurfacescanner_longrange` («Long range detailed»), conservando `merc_coins` 5/5/10/20 y el ingeniero. Anotar en `source_note`: «blueprint Merc; módulo exacto: Detailed Surface Scanner Merc; EDDiscovery 7d30920 usa DiscoveryScanner (marcador no comprable); verificar en Inara/juego». Si C8 modela solo ingeniería estándar, la alternativa válida es marcarlas `out_of_scope` (motivo: blueprint Merc, solo aplica al módulo Merc), pero mientras el esquema ya soporta `merc_coins` (115 filas) lo coherente es remapear.

## 2. Experimentales de arma: `special_weapon_*`

### Hechos y fuentes
- En el snapshot **no existen filas explícitas** para `special_weapon_toughened/efficient/damage/lightweight` (solo la fila genérica `Weapon` de cada una), así que no son duplicados. `special_weapon_rateoffire` (línea 1404) sí es explícita (8 tipos) y es una clave distinta.
- `Weapon` es solo un marcador del enum (línea 507, junto a `FighterWeapon`). No son experimentales de traje: EDEngineer (2cadd66) agrupa las mejoras de armas a pie bajo `Type: "Weapon"` con nombres como «Stability» o «Magazine size», pero «Oversized», «Double Braced», «Flow Control» y «Stripped Down» no aparecen bajo ese tipo.
- EDEngineer 2cadd66 (MIT, ya aprobado en el repo; `blueprints.json`, accedido 2026-10-22) lista estos cuatro efectos para **estos 11 tipos**: Beam Laser, Burst Laser, Pulse Laser, Cannon, Multi-cannon, Fragment Cannon, Plasma Accelerator, Rail Gun, Missile Rack, Torpedo Pylon, Mine Launcher. (Los mismos efectos también aplican a Power Plant, Thrusters, FSD, Shield Generator, Shield Cell Bank, Shield Booster y Power Distributor en sus variantes de módulo no-arma; eso ya lo cubren filas explícitas.)
  - Ingredientes de EDEngineer coinciden con las filas fijadas: Oversized `5MS,3MC,1Ru`; Double Braced `5MS,5CC,3V`; Flow Control `5MS,3HC,1EFW`; Stripped Down `5SAll,5C,1Sn`.
- coriolis-data (solo comparación, no se incorpora nada): los cuatro efectos aplican a beam, burst, pulse, cannon, fragment cannon, multi-cannon (y variante avanzada), plasma accelerator, rail gun, torpedo pylon y mine launcher. **No** lista Missile Rack. En coriolis `Stripped Down` pide `Carbon 3` (EDEngineer y EDDiscovery: 5). Discrepancia no resuelta; el snapshot fijado (5) coincide con EDEngineer.
- Fandom (CC BY-SA, solo para contrastar un hecho): indica que Oversized/Flow Control se pueden aplicar a «todas las armas excepto Experimentales y herramientas de minería» (https://elite-dangerous.fandom.com/wiki/Oversized, extracto del buscador; página no accesible, 402). Compatible con la lista de 11 y no concluyente sobre tipos concretos.
- EDEngineer (2024) no incluye módulos posteriores (Seeker Missile Rack, Enzyme, Shock Cannon, Remote Release Flak/Flechette, Abrasion Blaster, Sub-surface Displacement, etc.): no hay evidencia sobre ellos, ni a favor ni en contra.

### Recomendación por fila
Las cuatro filas: remapear a `BeamLaser, BurstLaser, PulseLaser, Cannon, Multi_Cannon, FragmentCannon, PlasmaAccelerator, RailGun, MissileRack, TorpedoPylon, MineLauncher`, con nota «lista contrastada con EDEngineer 2cadd66; coriolis-data no incluye MissileRack; sin evidencia para armas posteriores a 2024».

| Fila | Clave | Efecto | Tipos | Confianza |
|---|---|---|---|---|
| 1377 | `special_weapon_toughened` | Double Braced | los 11 | alta (8 básicos), media (MissileRack, Torpedo, Mine) |
| 1388 | `special_weapon_efficient` | Flow Control | los 11 | idem |
| 1407 | `special_weapon_damage` | Oversized | los 11 | idem |
| 1429 | `special_weapon_lightweight` | Stripped Down | los 11 | idem |

Si se prefiere cautela máxima: remapear solo a los 8 tipos confirmados por EDEngineer **y** coriolis (sin `MissileRack`) y dejar `MissileRack` marcado `verified: false`. La recomendación es incluirlo con `verified: false`.

## Limitaciones
- Inara (503) y Fandom (402/403) no fueron accesibles; el wiki de Frontier/Galnet devolvió 403. Los detalles de ingredientes del blueprint Merc DSS no tienen segunda fuente.
- Se recomienda una comprobación en juego (menú de ingeniero de Etienne Dorn con un DSS Merc) antes de publicar `verified: true`.

## Fuentes (accedidas 2026-10-22)
- EDEngineer `blueprints.json` @ 2cadd66: https://raw.githubusercontent.com/msarilar/EDEngineer/2cadd66d19509221ea2e5c2c132b8f04dc8167c8/EDEngineer/Resources/Data/blueprints.json (MIT)
- EDDiscovery EliteDangerousCore master (misma versión que el snapshot): https://github.com/EDDiscovery/EliteDangerousCore (Apache-2.0)
- coriolis-data (solo comparación): https://github.com/EDCD/coriolis-data (`modifications/specials.json`, `modifications/modules.json`)
- Merc Coin: https://edfieldmanual.com/wiki/Merc_Coin
- Inara DSS Long range detailed: https://inara.cz/elite/blueprint/205/ (solo extracto del buscador; 503 al leer)
- Notas de versión Operations: https://www.elitedangerous.com/news/updates/4-4-0-0 (403 al leer)
