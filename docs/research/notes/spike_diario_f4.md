# Spike de diario F4 (materiales, ingenieros, taquilla)

Fecha: 2026-10-21. Método: script desechable de solo lectura sobre los 9 diarios del propietario (Odyssey, 5 sesiones `LoadGame`); solo se anotan **agregados** y claves de datos del juego (política de licencias del proyecto). Lo no visto se completa con el manual público del diario de Frontier y se marca «no visto / documentación».

## 1. Recuentos y campos por evento (9 ficheros, 5 sesiones)

| Evento | N | Campos observados |
|---|---|---|
| `Materials` | 5 | `Raw`, `Manufactured`, `Encoded` (listas siempre presentes). Entrada: `Name`, `Count`, `Name_Localised` (ausente en 5 de 50 entradas `Raw`; presente en `Manufactured`/`Encoded`). |
| `MaterialCollected` | 6 | `Category`, `Name`, `Name_Localised`, `Count` (categorías vistas: `Manufactured`, `Encoded`) |
| `MaterialDiscarded` | 0 | no visto / documentación: `Category`, `Name`, `Name_Localised`, `Count` |
| `MaterialTrade` | 1 | `MarketID`, `TraderType`, `Paid`, `Received` (`TraderType` visto: `encoded`; documentación: `raw`, `manufactured`, `encoded`). `Paid`/`Received` son objetos con `Material`, `Material_Localised`, `Category`, `Quantity` (documentación). |
| `Synthesis` | 0 | no visto / documentación: `Name`, `Materials` (lista `Name`/`Count`) |
| `EngineerCraft` | 9 | `Slot`, `Module`, `Ingredients`, `Engineer`, `EngineerID`, `BlueprintID`, `BlueprintName`, `Level`, `Quality`, `Modifiers`. `Ingredients` es una lista de `Name`, `Name_Localised`, `Count`. `Modifiers`: `Label`, `Value`, `OriginalValue`, `LessIsGood`. `ExperimentalEffect` no apareció (0/9); documentación: `ApplyExperimentalEffect` y `ExperimentalEffect(_Localised)` cuando se aplica un efecto. |
| `EngineerProgress` | 8 | ver §2 |
| `EngineerContribution` | 0 | no visto / documentación: `Engineer`, `EngineerID`, `Type` (`Commodity`/`Materials`/`Credits`/`Bond`/`Bounty`), `Commodity`/`Material`, `Quantity`, `TotalQuantity` |
| `TechnologyBroker` | 0 | no visto / documentación: `BrokerType` (`guardian`/`human`), `MarketID`, `ItemsUnlocked`, `Commodities`, `Materials` |
| `ShipLocker` | 296 | 273 con contenido, 23 stub (ver §3) |
| `BackpackChange` | 0 | no visto / documentación: `Added`/`Removed` (listas con `Name`, `Type`, `Count`, `OwnerID`) |
| `MissionCompleted` | 14 | `MaterialsReward` en 1/14 (ver §4) |
| `BuyMicroResources` | 6 | `Name`, `Name_Localised`, `Category`, `Count`, `Price`, `MarketID` |
| `ModuleRetrieve` | 12 | `EngineerModifications`, `Level`, `Quality` solo en 1/12 (módulo con ingeniería) |
| `ScientificResearch`, `EngineerApply`, `SellMicroResources`, `TradeMicroResources`, `TransferMicroResources`, `UseConsumable`, `CollectItems`, `DropItems`, `RestockVehicle` | 0 | no vistos |

Valores distintos: `TraderType` = {`encoded`}; `Category` en `MaterialCollected` = {`Manufactured`, `Encoded`}; `BrokerType` = ninguno (no visto).

## 2. EngineerProgress

Dos formas, ambas observadas:

1. **Instantánea con lista** (5): `{ Engineers: [ { Engineer, EngineerID, Progress, Rank, RankProgress } ] }`. Cada entrada tiene siempre `Engineer`, `EngineerID`; `Progress` en casi todas; `Rank` y `RankProgress` solo en ingenieros desbloqueados (`Rank` en 20, `RankProgress` en 18 de 61 entradas). Una entrada sin `Progress` (1 caso): debe tolerarse.
2. **Actualización suelta** (3): sin `Engineers`, campos en la raíz: `Engineer`, `EngineerID` y, según el caso, `Progress`, `Rank` o ambos (se vieron las combinaciones `Progress`; `Rank`; `Progress`+`Rank`). Sin `RankProgress`.

`Progress` distintos: `Known`, `Invited`, `Unlocked`. `Rank` y `RankProgress` son opcionales en ambas formas.
Orden en la sesión: `Commander`, `Materials`, `EngineerProgress` y `LoadGame` (en 4 sesiones `EngineerProgress` va antes de `LoadGame`; en 1, después), luego `ShipLocker`.

## 3. ShipLocker

- Con contenido (273 de 296): `Items`, `Components`, `Consumables`, `Data`; las cuatro listas están siempre presentes (pueden ir vacías). Entradas: `Name`, `Name_Localised`, `OwnerID`, `Count`. En los datos del propietario solo `Consumables` tenía elementos (1365 entradas); las demás siempre vacías en este conjunto (los items/componentes/datos tienen la misma forma según la documentación).
- Stub (23 de 296): únicamente `timestamp` y `event`, sin listas. **Un stub no debe vaciar el inventario**: el contenido real está en `ShipLocker.json` (clave `Items`/`Components`/`Consumables`/`Data`, `event`, `timestamp`) y se relee al ver el stub.
- `ShipLocker.json` presente en la carpeta, con las mismas claves de primer nivel.
- `Backpack.json` también existe; `BackpackChange` no apareció.

## 4. MaterialsReward

Una instancia en `MissionCompleted`: entrada con `Name`, `Name_Localised`, `Category`, `Category_Localised`, `Count`. **Trae `Category`**, pero con formato localizable: `$MICRORESOURCE_CATEGORY_Manufactured;` (no el simple `Manufactured`). Hay que normalizar (quitar `$MICRORESOURCE_CATEGORY_` y `;`) o resolverla por `materials.json`. Documentación: puede aparecer también como `Raw`/`Encoded` y, para microrecursos, `Item`/`Component`/`Data`/`Consumable`.

## 5. Loadout.Modules[].Engineering

8 módulos con ingeniería en 20 `Loadout`: presentes siempre `Engineer`, `EngineerID`, `BlueprintID`, `BlueprintName`, `Level`, `Quality`, `Modifiers`. **Ausentes** (0/8): `ExperimentalEffect` y `ExperimentalEffect_Localised` (no visto / documentación: solo aparecen si hay efecto experimental). `Modifiers` repite `Label`, `Value`, `OriginalValue`, `LessIsGood`.

Claves de blueprint distintas en `Loadout`: 1 (`Engine_Dirty`). En `EngineerCraft`: 4 (`Engine_Dirty`, `FSD_LongRange`, `PowerPlant_Boosted`, `Sensor_Expanded`). Claves experimentales: 0 vistas.

## 6. Respuestas del Step 2

- **¿`Materials` es instantánea completa en cada `LoadGame`?** Sí: 5 `Materials` en 5 sesiones, uno por `LoadGame` (siempre justo tras `Commander` y antes de `LoadGame`). Contiene siempre las tres listas completas (10 Raw, 19-20 Manufactured, 21 Encoded). Es la fuente de resincronización.
- **Consumidores de materiales no listados:** en los datos del propietario no se vieron. Por documentación consumen o aportan materiales además de los listados: `ScientificResearch` (`Name`, `Category`, `Count`), `MissionCompleted.MaterialsReward`, `EngineerContribution` (tipo `Materials`), `TechnologyBroker` (`Materials` en el desbloqueo), `EngineerCraft.Ingredients`, `Synthesis`, `MaterialTrade`, `MaterialDiscarded`, `MaterialCollected`. Otros: `DataScanned` no concede materiales al inventario por sí solo (la recogida llega como `MaterialCollected`).

## 7. Criterio de decisión

- Consumidores no listados en la spec (`ScientificResearch`, `EngineerContribution`, `TechnologyBroker`): no se tipan en F4; la siguiente instantánea `Materials` (cada `LoadGame`) resincroniza. Anotar en «Desviaciones» del plan.
- `MaterialsReward` trae `Category`, pero en forma `$MICRORESOURCE_CATEGORY_<X>;`: no sirve tal cual. Usar `materials.json` como fuente de categoría (decisión del criterio para «sin Category» aplicada por precaución) o normalizar el valor; en cualquier caso el parser debe tolerar `Category` ausente.

## 8. Contradicciones potenciales con el plan

- Stub de `ShipLocker` = solo `timestamp` + `event` (23/296, 8 %): el reducer debe tratarlo como «relee `ShipLocker.json`», no como inventario vacío.
- `EngineerProgress` suelto puede traer solo `Rank` o solo `Progress`; `Rank`/`RankProgress`/`Progress` opcionales.
- `Name_Localised` opcional en `Materials.Raw`.
- `ExperimentalEffect` nunca visto: fixtures de efecto experimental deben salir de documentación (claves p. ej. `special_*`), no del diario del propietario.
