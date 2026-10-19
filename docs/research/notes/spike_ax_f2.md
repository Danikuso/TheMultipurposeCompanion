# Spike AX para F2 (combate y minería)

Fecha: 2026-10-18. Alimenta las Tasks 5, 13, 15 y 26 del plan de F2 (`THARGOID_SHIPS`, regla de `RedeemVoucher`, detección del bono AX, presencia thargoide).

## 1. Método y privacidad

- Script desechable (fuera del repo, en el área temporal de la sesión), solo lectura sobre los 7 diarios del propietario. Solo se emiten agregados: recuentos por evento y por valor de campos que son enumeraciones del juego (tipo de nave, `ScanStage`, claves de campos). No se imprimió ni se guardó ningún nombre de comandante, FID, piloto, nave, chat ni sistema. Los valores de `VictimFaction` que no eran símbolos (`$...;`) se agruparon como `<named>`.
- Corpus: 3641 líneas, 0 corruptas, sesiones 2026-10-16 y 2026-10-18 (más `Status.json` actual).

## 2. Resultado: los diarios NO contienen datos AX

Ninguna actividad thargoide en el corpus. La única coincidencia textual con «thargoid» es un nombre de portaaviones en un `FSSSignalDiscovered` (`SignalType=FleetCarrier`), sin relación con AX.

| Evento / campo | Estado | Apariciones | Notas |
|---|---|---|---|
| `ShipTargeted.Ship` con valores thargoides | no visto | 0 de 150 `ShipTargeted` | Solo naves humanas (viper, cobramkiii, anaconda, eagle, type6/9, krait_mkii, mamba, asp, vulture, diamondbackxl, typex, ferdelance, empire_courier, belugaliner…). `ScanStage` 0–3 confirmado |
| `FactionKillBond` (cualquiera) | no visto | 0 | Sin zonas de conflicto en el corpus |
| `FactionKillBond.VictimFaction` thargoide | no visto | 0 | |
| `Bounty` | confirmado | 8 | Claves: `PilotName`, `PilotName_Localised`, `Rewards[{Faction,Reward}]`, `Target`, `Target_Localised` (opcional), `TotalReward`, `VictimFaction`, `event`, `timestamp`. `TotalReward` = suma de `Rewards[].Reward` en 8/8. `VictimFaction` = nombre de facción (no símbolo) en 8/8 |
| `SharedWithOthers` en `Bounty`/`FactionKillBond` | no visto | 0 | El campo no existe en ninguno de los 8 `Bounty` |
| `RedeemVoucher` | confirmado (1) | 1 | Claves: `Amount`, `Factions[{Faction,Amount}]`, `Type` (`bounty`), `event`, `timestamp`. Sin `Faction` de nivel superior ni `BrokerPercentage` |
| `RedeemVoucher.BrokerPercentage` | no visto | 0 | No se puede saber si `Amount` es neto o nominal con broker. Con 1 muestra sin broker, `Amount` = suma de `Factions[].Amount` |
| Bloque `ThargoidWar` en `FSDJump`/`Location` | no visto | 0 de 103 `FSDJump` y 3 `Location` | |
| Estados de facción `Thargoid_*` | no visto | 0 | |
| `Status.LegalState` = `Thargoid` | no visto | `Status.json` sin `LegalState` | |

Otros eventos de combate presentes (útiles para fixtures reales): `UnderAttack` 19, `ShieldState` 6, `HullDamage` 2, `HeatWarning` 2, `HeatDamage` 2, `Interdicted` 1, `NavBeaconScan` 4.

## 3. Notas de actualización 4.1–4.4.x (Step 2)

Consultadas con `curl` (1 petición/s): <https://www.elitedangerous.com/update-notes/4-1-0-0>, `4-2-0-0`, `4-3-0-0`, `4-3-0-1`, `4-4-0-0`, `4-4-1-0` (mismo prefijo `https://www.elitedangerous.com/update-notes/`). Únicas menciones AX/thargoides en todas ellas:

- «Removed AX Combat chatter from AI pilots from the Titan Aftermath scenario in Sol» (aparece en la lista de 4.1.0.0; las páginas posteriores repiten ese texto en el historial embebido).
- Ninguna nota de 4.1–4.4.1.0 menciona cambios en interceptores, zonas de conflicto AX ni recompensas AX.

Conclusión: sin cambios documentados de mecánica AX tras la guerra (fin 2024-12-13); coincide con `player_roles_domain.md` §8. No se puede afirmar que las ZC AX o los interceptores sigan activos (laguna ya anotada allí).

## 4. Fuentes secundarias (solo nombres de campo; no confirmadas con datos reales)

Referencias: manual del diario (<https://elite-journal.readthedocs.io/en/latest/Combat.html>, `Trade.html`), definiciones de eventos de EDDiscovery (<https://github.com/EDDiscovery/EliteDangerousCore>, solo como referencia de nombres; sin copiar código). Estos nombres no se pudieron re-verificar en esta sesión y deben tratarse como «fuente secundaria».

- `ShipTargeted.Ship` thargoides (valores en minúsculas, a verificar): `scout`, `scout_hq`, `scout_q`, `scout_c`, `scout_nq` (scouts), `cyclops`, `basilisk`, `medusa`, `hydra`, `scythe`, `glaive`, `orthrus` (interceptores y variantes), más swarm/«Thargon» (nombre de nave no verificado).
- `FactionKillBond.VictimFaction` thargoide: símbolo `$faction_Thargoid;` (a verificar).
- `ThargoidWar` (en `FSDJump`/`Location`): `CurrentState`, `NextStateSuccess`, `NextStateFailure`, `SuccessStateReached`, `WarProgress`, `RemainingPorts`, `EstimatedRemainingTime` (ya citado en `player_roles_domain.md` §8).
- Estados de facción: `Thargoid_Probing`, `Thargoid_Harvest`, `Thargoid_Controlled`, `Thargoid_Stronghold`, `Thargoid_Recovery`.
- `Status.LegalState`: valor `Thargoid` (manual del Status File).
- `RedeemVoucher`: campos documentados `Type`, `Amount`, `Faction`, `BrokerPercentage`; el manual no aclara si `Amount` es neto o nominal.

## 5. Decisiones para las tasks consumidoras

- **`BrokerPercentage` (Task 13):** no confirmado con datos reales (0 apariciones) y la fuente secundaria no aclara neto/nominal. Regla: usar `Amount` tal cual (tratarlo como valor nominal) y no aplicar corrección; si existe `BrokerPercentage`, solo mostrarlo como dato informativo. Si en el futuro un diario real demuestra que `Amount` es neto, aplicar `Amount / (1 − BrokerPercentage/100)`. Anotar la omisión en «Desviaciones» del plan.
- **Asistencias / `SharedWithOthers`:** no confirmado (campo ausente en 8/8 `Bounty`). Omitir cualquier lógica de asistencia o reparto; contar cada `Bounty`/`FactionKillBond` como un evento completo. Anotar la omisión en «Desviaciones».
- **`THARGOID_SHIPS` (Task 5):** lista no confirmada. Recomendación: incluirla como constante pequeña basada en la fuente secundaria (§4), tolerante a valores desconocidos, y marcarla como no verificada; o bien omitir el bono AX. Decisión de la task: omitir lo no confirmado salvo que el propietario indique lo contrario.
- **Detección del bono AX (Task 15) y presencia thargoide (Task 26):** sin evidencia real. Fixtures AX sintéticos (criterio 7 de la spec lo permite), construidos solo con los nombres de campo de §4, y avisando en la documentación de que son sintéticos. `ThargoidWar`, `Thargoid_*` y `LegalState=Thargoid` deben deserializarse con campos opcionales y no fallar si faltan.
- Prioridad: AX es un rol nicho/heredado tras el fin de la guerra; mantener coste mínimo.
