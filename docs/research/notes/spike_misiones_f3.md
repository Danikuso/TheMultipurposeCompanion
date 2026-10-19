# Spike de misiones para F3 (2026-10-19)

Fuentes: (A) agregados de los diarios del propietario (7 ficheros `Journal.*.log`, solo lectura; sin nombres, IDs ni facciones en esta nota); (B) documentación pública (Frontier Journal Manual v4.x; comportamiento conocido en EDMC y EDDiscovery, consultado como descripción de formato, sin copiar código GPL). Todo lo marcado **no visto** viene solo de (B) y es **fuente secundaria / sin verificar**.

La muestra local es pequeña: 7 `MissionAccepted`, 7 `MissionCompleted`, 6 `MissionRedirected`, 3 `Missions`, 8 `Bounty`. Hay 0 de `MissionFailed`, `MissionAbandoned`, `CargoDepot`, `Passengers`, `CommunityGoal*`, `FactionKillBond`.

## 1. `Missions` (verificado la forma; contenido no visto)

- Campos: `Active`, `Failed`, `Complete` (las tres listas) + `timestamp`, `event`.
- En los 3 eventos de la muestra las tres listas están vacías (3 de 3 con recuento 0/0/0), así que los campos por elemento **no se han visto**.
- Por documentación (B, no visto): cada elemento trae `MissionID` (int), `Name` (str), `PassengerMission` (bool), `Expires` (int, segundos restantes **desde el momento del evento `Missions`**, no fecha absoluta; en `Failed`/`Complete` es 0). Regla de Task: `caducidad = timestamp(Missions) + Expires s`; `Expires <= 0` en `Active` = ya caducada o sin dato. No visto -> verificar con fixtures reales cuando haya misiones activas.
- `Missions` no trae destino, recompensa, KillCount ni facción: esos salen de `MissionAccepted` (historial), como dice la spec §3.3. Las misiones sin `MissionAccepted` en el historial se muestran con los pocos campos conocidos.

## 2. `MissionAccepted` por tipo (verificado)

Campos comunes (7/7): `timestamp`, `event`, `Name` (str), `LocalisedName` (str), `Faction` (str), `MissionID` (int), `Expiry` (str ISO `AAAA-MM-DDTHH:MM:SSZ`, UTC), `Wing` (bool), `Influence` (str: `+`, `++`...), `Reputation` (str: `+`, `++`...), `Reward` (int), `DestinationSystem` (str, presente 7/7). `DestinationStation` (str) presente 6/7: **ausente** en turismo.

Nombres internos (`Name`, sin sufijo en la muestra): `Mission_Massacre`, `Mission_Assassinate`, `Mission_PassengerVIP`, `Mission_Sightseeing`. En la muestra el `Name` de `MissionAccepted` es la base sin sufijo; en `MissionCompleted` aparece con sufijo `_name` (`Mission_PassengerVIP_name`, `Mission_Sightseeing_name`). **Regla para `classify_kind`: clasificar por prefijo tras normalizar (quitar `_name` final) y comparar sin distinguir mayúsculas.**

| Tipo | Campos específicos | Estado |
|---|---|---|
| Masacre (`Mission_Massacre*`) | `KillCount` (int), `TargetFaction` (str), `TargetType` (str, token `$MissionUtil_FactionTag_Pirate;`), `TargetType_Localised` (str) | verificado (2) |
| Asesinato (`Mission_Assassinate*`) | `Target` (str, nombre del objetivo), `TargetFaction`, `TargetType` (`$MissionUtil_FactionTag_PirateLord;`), `TargetType_Localised`. Sin `KillCount` | verificado (3) |
| Pasajeros VIP (`Mission_PassengerVIP*`) | `Commodity` (str `$X_Name;`), `Commodity_Localised`, `Count` (int), `PassengerCount` (int), `PassengerVIPs` (bool), `PassengerWanted` (bool), `PassengerType` (str: `Tourist` visto) | verificado (1) |
| Turismo (`Mission_Sightseeing*`) | mismos campos de pasajeros que arriba; `DestinationStation` ausente | verificado (1) |
| Entrega/recogida (`Mission_Delivery*`, `Mission_Collect*`, `Mission_Mining*`) | `Commodity`, `Commodity_Localised`, `Count` (int); `DestinationStation`/`System` | **no visto** (B) |

Observaciones:
- `Count` (cargo de misión de pasajeros) **no es** `PassengerCount`: en las dos misiones de pasajeros vistas difieren (Count 3 / PassengerCount 4 y 6). Para ocupación de cabinas usar `PassengerCount`.
- Otros prefijos conocidos (B, no vistos): `Mission_Courier*`, `Mission_Delivery*`, `Mission_Salvage*`, `Mission_Mining*`, `Mission_Rescue*`, `Mission_Altruism*`, `Mission_Collect*`, `Mission_Sightseeing*`, `Mission_PassengerBulk*`, `Mission_PassengerVIP*`, `Mission_Hack*`, `Mission_Scan*`, `Mission_Disable*`, `Mission_OnFoot_*`. Constantes sugeridas para `classify_kind` (Task 5): Massacre, Assassinate, Delivery/Courier/Collect/Mining/Salvage, PassengerBulk/PassengerVIP, Sightseeing, resto -> `Other`.

## 3. Destinos múltiples del turismo (VERIFICADO)

`Mission_Sightseeing` (1 de 1 en la muestra) trae en `DestinationSystem` **varios sistemas en el mismo string**, separados por el token literal `$MISSIONUTIL_MULTIPLE_FINAL_SEPARATOR;`, sin espacios alrededor. Forma real (nombres ficticios):

```json
{"event":"MissionAccepted","Name":"Mission_Sightseeing","LocalisedName":"Tour of Example Region","Faction":"Ficticia Ltd","Commodity":"$Clothing_Name;","Commodity_Localised":"Clothing","Count":3,"DestinationSystem":"Alfa Test$MISSIONUTIL_MULTIPLE_FINAL_SEPARATOR;Beta Test","Expiry":"2026-01-01T12:00:00Z","Wing":false,"Influence":"+","Reputation":"+","Reward":500000,"PassengerCount":6,"PassengerVIPs":false,"PassengerWanted":false,"PassengerType":"Tourist","MissionID":1}
```

- Con 2 destinos solo aparece `…FINAL_SEPARATOR;` (visto). Los nombres de sistema pueden contener espacios; **no dividir por espacios**.
- `DestinationStation` no existe en turismo (el turista no se entrega en estación).
- **No visto** (B, EDMC/EDDiscovery): con 3 o más destinos los intermedios se separan con `$MISSIONUTIL_MULTIPLE_INNER_SEPARATOR;` y el último con `…FINAL_SEPARATOR;`. Regla de parseo tolerante: dividir por la regex `\$MISSIONUTIL_MULTIPLE_[A-Z_]+;` (INNER o FINAL) y descartar segmentos vacíos.
- `MissionCompleted` de turismo repite `DestinationSystem` con el mismo token (visto) y `Name` con sufijo `_name`.
- `MissionRedirected` de turismo (1): `OldDestinationSystem` y `NewDestinationSystem` **sin tokens**, un solo sistema (`NewDestinationStation` no vacío, `OldDestinationStation` vacío). Es decir, tras redirigir ya no hay lista; el nuevo destino es único (típicamente la vuelta/entrega final).
- Regreso al origen: **no visto**. El juego no marca en el diario si la misión exige volver; el `MissionRedirected` posterior (nuevo destino con estación) es la señal real. Regla: la ruta óptima abierta se calcula sobre las paradas; el regreso solo se modela al llegar el `MissionRedirected`.

Decisión (spec §10.7): hay destinos múltiples reales -> **opción B viable, sin desviación** y sin aviso «paradas no disponibles en el diario» salvo cuando `DestinationSystem` no contenga token y sea turismo (1 segmento).

## 4. `MissionCompleted` (verificado)

Campos: `Name` (con `_name`), `LocalisedName`, `Faction`, `MissionID`, `Reward` (int), `DestinationSystem`, `DestinationStation` (según tipo), `Commodity`/`Commodity_Localised`/`Count` (entrega, pasajeros), `KillCount`/`TargetFaction`/`TargetType`(+`_Localised`)/`Target` (combate), `FactionEffects` (lista, 7/7).

`FactionEffects[]`: `Faction` (str), `Effects` (lista), `Influence` (lista de `{SystemAddress:int, Trend:str, Influence:str}`), `Reputation` (str), `ReputationTrend` (str). Valores vistos: `Trend` ∈ {`UpGood`, `DownBad`}, `Influence` ∈ {`+`, `++`}, `ReputationTrend` ∈ {`UpGood`, `DownBad`}. Cada misión tiene 1-2 entradas de facción (13 entradas en 7 misiones).

- `MaterialsReward`: **no visto** (0 de 7). Por (B): lista de `{Name, Name_Localised?, Category, Category_Localised?, Count}`. Tipar como opcional.
- `CommodityReward`: no visto (B: lista `{Name, Count}`). `Donation`/`Donated`, `PermitsAwarded`: no vistos (B). Tipar opcionales con `#[serde(default)]`.
- Para «recompensas de sesión»: créditos = `Reward`; reputación/influencia = `FactionEffects[].Reputation` / `Influence[].Influence` (tokens `+` a `+++++`, **no numéricos**).

## 5. `MissionRedirected` (verificado)

6 eventos. Campos: `MissionID` (int), `Name`, `LocalisedName`, `NewDestinationStation`, `NewDestinationSystem`, `OldDestinationStation`, `OldDestinationSystem` (todos str). En 6 de 6 el sistema nuevo es distinto del antiguo. Las estaciones antiguas pueden venir vacías. En masacre (2) y asesinato (3) es el aviso de «vuelve a la emisora» (misión completada en objetivos). `MissionRedirected` en masacre = misión completa (spec §10.10 correcto).

## 6. `MissionFailed` / `MissionAbandoned` (no visto)

0 eventos. Por (B): `MissionFailed` {`Name`, `LocalisedName`, `MissionID`, `Fine`(int, opcional)}; `MissionAbandoned` {`Name`, `MissionID`, `Fine`? (opcional)}. Solo `MissionID` es imprescindible.

## 7. `CargoDepot` (no visto)

0 eventos. Por (B): `MissionID`, `UpdateType` ∈ {`Collect`, `Deliver`, `WingUpdate`}, `CargoType` (str), `Count` (int), `StartMarketID`, `EndMarketID`, `ItemsCollected`, `ItemsDelivered`, `TotalItemsToDeliver`, `Progress` (float 0-1). Progreso = `ItemsDelivered/TotalItemsToDeliver`. Es el único progreso de entrega fiable.

## 8. `Passengers` (no visto)

0 eventos. Por (B): `Manifest` lista de `{MissionID, Type, VIP:bool, Wanted:bool, Count}`. **No incluye clase de cabina.** `Type` ∈ {`Tourist`, `Soldier`, `Explorer`, ... } (solo `Tourist` visto en `MissionAccepted.PassengerType`).

**Decisión (criterio del brief): sin clase de cabina en el manifiesto -> ocupación agregada (Task 26)**. La clase de cabina solo puede deducirse de los módulos de pasajeros (`Loadout`) con la tabla estática `crates/tmc-modules/data/` de clases por módulo; no del evento.

## 9. `CommunityGoal` (no visto)

0 eventos. Por (B): `CurrentGoals` lista de `{CGID, Title, SystemName, MarketName, Expiry, IsComplete, CurrentTotal, PlayerContribution, NumContributors, PlayerPercentileBand, TierReached, Bonus?, TopTier{Name,Bonus}, TopRankSize?, PlayerInTopRank?}`. `Expiry` es fecha ISO absoluta. Tipar casi todo como opcional.

## 10. Inventario `Cargo` y `MissionID` (verificado parcialmente)

`Cargo`: en 6 eventos, solo 2 traen `Inventory` (lista; el resto es resumen `Count`+`Vessel`). Los dos `Inventory` de la muestra están vacíos: campos de ítem y `MissionID` **no vistos**. Por (B): `{Name, Name_Localised?, Count, Stolen, MissionID?}`; `MissionID` presente en carga de misión. Task 27: avisar «carga de misión que falta» comparando `CargoDepot`/inventario con `MissionID`.

## 11. Masacre: qué cuenta como baja

- `Bounty.VictimFaction` coincide literalmente con `TargetFaction` de las misiones activas: **4 de 8** eventos `Bounty` (los otros 4 son de facciones distintas, coherente con la regla). Comparar por igualdad exacta de cadena es válido.
- `FactionKillBond`: **0 eventos**, no confirmado. **`MASSACRE_COUNTS_KILL_BONDS = false`** (por defecto del brief; subir a `true` solo con evidencia).
- `Bounty` trae `VictimFaction`, `Target` (tipo de nave, token), `TotalReward`, `Rewards[]`, `PilotName`. La masacre cuenta `Bounty`, no `PilotName`.

## 12. `Docked`: plataformas y servicios (verificado)

`LandingPads` = objeto `{Small:int, Medium:int, Large:int}` (12/12); `StationServices` = lista de str (12/12); `StationType` str; `MarketID` int.

## 13. Resumen de decisiones para tareas posteriores

1. `classify_kind` (Task 5): por prefijo del `Name` normalizado (sin `_name`, sin mayúsculas); Massacre/Assassinate/Delivery-like/Passenger (`PassengerBulk`|`PassengerVIP`)/Sightseeing/Other.
2. Parseo de destinos (Task 23): regex `\$MISSIONUTIL_MULTIPLE_[A-Z_]+;` — verificado el token `FINAL_SEPARATOR` con 2 sistemas sin espacios; `INNER_SEPARATOR` por documentación. No desviación de §10.7.
3. Regreso en turismo: señal = `MissionRedirected` (no hay indicador en `MissionAccepted`).
4. Ocupación de pasajeros agregada (sin clase de cabina en `Passengers`) (Task 26).
5. `MASSACRE_COUNTS_KILL_BONDS = false`.
6. `Expires` de `Missions` = segundos restantes relativos al `timestamp` del evento (no visto, B).
7. Reputación/influencia son tokens (`+`..`+++++`, `UpGood`/`DownBad`), no números.

## Contradicciones con la spec §10

Ninguna. §10.7 se confirma (token real). Matiz: `MissionAccepted.Count` ≠ `PassengerCount`; `DestinationStation` ausente en turismo; `Name` con sufijo `_name` solo en `MissionCompleted`.
