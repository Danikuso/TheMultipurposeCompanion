# TMC · F4 · Propuesta de enmienda para el catálogo completo

- **Estado: PENDIENTE DE APROBACIÓN TÉCNICA.** Documento de diseño; no autoriza implementación ni sustituye la spec o el plan activos.
- Fecha: 2026-10-22. Redactor: Codex, rol arquitecto; skill `deep-plan`.
- Base: [spec F4](../specs/2026-10-21-tmc-f4-ingenieros-materiales-design.md), [plan F4](2026-10-21-tmc-f4-ingenieros-materiales.md), [investigación y fuentes](../research/notes/2026-10-22-catalogos-f4-web.md).
- **Decisión ya aprobada por el propietario:** EDDiscovery/EliteDangerousCore como fuente A. **Pendientes:** interfaces de esta enmienda y fuente auxiliar de IDs descrita en §3. Esta aprobación de EDDiscovery no se extiende automáticamente a EDDI.
- Alcance: completar Tasks 9–10 y adaptar consumidores de Tasks 11/14–16/18–24/26/28. Tasks 26–27 pueden continuar con su alcance vigente; no cerrar F4 con catálogos provisionales.

## 1. Diagnóstico

El catálogo actual contiene dos blueprints, un experimental y cinco ingenieros provisionales. La spec exige **todos los blueprints y experimentales de módulos de nave**, incluidos los de Operations que aparecen en la fuente aprobada. Ampliar solo los JSON rompe supuestos de la aplicación:

1. `Blueprint.grades` y `Experimental.ingredients` solo representan materiales; las recetas nuevas también pueden cobrar Merc Coins. Importar únicamente materiales daría un coste incompleto presentado como total.
2. El test previsto `every_blueprint_has_contiguous_grades_from_1` exige G1 para todo. La fuente contiene mejoras G2–G5 y variantes compradas/preingenierizadas sin receta de fabricación. Ausencia de ingredientes no significa fabricación gratuita.
3. `EngineerBlueprint {blueprint,max_grade}` y `pick_engineer(blueprint,grade,...)` pierden el módulo. Caso verificado: Lori Jameson ofrece `misc_shielded` G4 para AFMU, pero G3 para Cargo Scanner/Kill Warrant Scanner/Wake Scanner.
4. `blueprint(key)` selecciona por una clave global; el importador exploratorio compara solo ingredientes, sin Merc Coins, capacidades, variantes ni elegibilidad. Su ausencia de colisiones no prueba que esas dimensiones sean equivalentes.
5. `module_type_of` usa el primer prefijo coincidente. Un catálogo completo requiere símbolos y reglas sin ambigüedad, además de elegibilidad de variantes por ítem. La enumeración de tipos de `Modules.cs` no proporciona por sí sola todos los símbolos de `Loadout`.
6. `engineering.engineers.selected.blueprints`, opciones de `engineering.flow` y costes duplicados completos pueden crecer por encima de 16 KiB. La paginación aprobada solo cubre objetivos y taquilla.
7. `alerts::goal_blueprint` reconstruye blueprint/grado analizando `Goal.id`; no conserva el módulo para recomendar ingeniero. Los objetivos ya guardan `item` y `request.module_type`, que deben ser la autoridad.
8. Los 38 registros de `Engineers.cs` ofrecen nombres, coordenadas y requisitos, pero no IDs numéricos del journal. `ActorEngineerFDName.cs` no resuelve esa carencia. Los consumidores actuales usan `u64` para selección, estado y avisos.

**Medidas exploratorias, no completitud:** revisión EDDiscovery `7d30920e6822d4836e0d6dd5c91026ec7e2ab4c7`; 1.167 líneas constructoras de categorías diversas; patrón parcial: 924 filas, 895 con ingredientes e ingeniero, 83 claves, 46 tipos y 105 filas con Merc Coins en ese subconjunto. Existen sobrecargas con arrays de módulos, experimentales, broker y Odyssey fuera de ese patrón. Estos números son controles de regresión de la investigación, nunca el total definitivo del producto.

## 2. Opción recomendada y alternativas

Ampliar el modelo de F4 en sus límites actuales: **recetas por módulo y grado, coste separado de Merc Coins, variantes de adquisición explícitas y REST paginado reutilizando el actor**. Conservar inventario de materiales, tasas de intercambio, migración SQLite y estructura persistida de objetivos. Usar el mapa auxiliar de IDs únicamente tras aprobación y auditoría, evitando un cambio transversal de identidad si los 38 cruces son completos.

| Opción | Complejidad y riesgo | Mantenimiento | Tokens/contexto futuro | Decisión |
|---|---|---|---|---|
| Modelo ampliado + importación reproducible + IDs auxiliares auditados | Media; cambia consumidores conocidos, conserva arquitectura | Una fuente normalizada y manifiesto por fila | Moderado al implementar; bajo después, contratos explícitos y generados | Recomendada |
| Añadir JSON sin cambiar contratos | Baja inicialmente; alto riesgo de coste/capacidad falsos | Excepciones dispersas, pruebas engañosas | Barato ahora, caro por diagnósticos recurrentes | Descartada |
| Excluir Operations/Merc Coins o fabricar G1 vacío | Baja; incumple alcance y produce cálculos falsos | Deuda sobre contenido actual | Ahorro aparente; requiere renegociar/repetir catálogo | Descartada |
| Recopilar todas las recetas a mano, fuente B | Alta; errores de transcripción y obsolescencia | Revisión manual completa en cada actualización | Alto y recurrente | Descartada tras aprobación A |
| Motor genérico para toda economía/adquisición del juego | Alta; amplía F4 hacia Odyssey, broker y F5 | Más abstracciones que requisitos actuales | Alto contexto permanente | Descartada para F4 |
| Identidad nueva `engineer_key` con `journal_id: Option<u64>` | Media/alta; toca acciones, filas, alertas, book y contratos UI | Correcta si faltan IDs; compatibilidad adicional | Alto frente al mapa auxiliar si este cubre los 38 | Reserva si falla la fuente auxiliar; exigir decisión concreta antes de implementarla |

## 3. Fuentes, importación y cobertura comprobable

### 3.1 Entrada fijada y procedencia

Importar datos, sin ejecutar ni integrar código C# de EDDiscovery. Fuentes aprobadas fijadas al commit anterior: `RecipesEngineering.cs`, `Engineers.cs`, `MCMRType.cs`, `Modules.cs`, `RecipeFDName.cs` y licencia. Cada archivo descargado debe registrar URL con commit, SHA-256, licencia, copyright y fecha de lectura. Cualquier archivo adicional requerido para resolver símbolos se audita con el mismo criterio y queda en el manifiesto; no asumir que un archivo GPL en otro proyecto queda cubierto por esta autorización.

**Fuente auxiliar propuesta, aún sin aprobar:** exclusivamente pares `(EngineerID, nombre)` de [EDCD/EDDI `DataDefinitions/Engineer.cs` fijado a `d3b964ea7c8bb959ad6537f55b308f293a326905`](https://github.com/EDCD/EDDI/blob/d3b964ea7c8bb959ad6537f55b308f293a326905/DataDefinitions/Engineer.cs). Investigación local: 38 declaraciones; `LICENSE.md` y `LicenseFile.txt` de la misma revisión declaran Apache-2.0. No se observó referencia a Coriolis/FDevIDs ni reserva específica de datos en el archivo examinado; esa observación no sustituye la revisión de procedencia antes de importar. No importar sus especialidades/recetas: la capacidad sigue saliendo de cada fila de EDDiscovery. No ampliar a `marketId` sin necesidad/aprobación; conservar el fallback de taller `(system,workshop)`.

Requisito de fase 0: comprobar unión uno a uno de los 38 nombres EDDiscovery con los 38 IDs auxiliares, documentar alias ortográficos explícitos y verificar unicidad de IDs. Mantener 300100/Felicity y 300160/Elvira. El fallback existente de eventos sin `EngineerID` por nombre se conserva. Un ID desconocido observado con nombre conocido se puede asociar al registro por correspondencia explícita no ambigua; una contradicción ID/nombre no se resuelve sobrescribiendo silenciosamente. No inventar secuencias numéricas ni asignar cero a varios ingenieros. Si el propietario no aprueba la fuente o el cruce falla, bloquear ese paso y revisar la alternativa de identidad opcional; la propuesta principal no necesita ese refactor por anticipado.

### 3.2 Conversor propuesto y artefactos

Añadir `scripts/import-engineering-catalogue.mjs` usando Node ya disponible para UI, sin dependencia de ejecución nueva. Interfaz:

```text
node scripts/import-engineering-catalogue.mjs --source-dir <snapshot> --out-dir <staging>
node scripts/import-engineering-catalogue.mjs --source-dir <snapshot> --check
```

`--check` no escribe; verifica hashes, todas las filas y coincidencia exacta con los artefactos versionados. Nunca descarga durante compilación, tests o ejecución. La obtención explícita de fuentes es un paso separado. El conversor tokeniza literales/argumentos, comentarios, arrays y sobrecargas conocidas del subconjunto C#; no usar una sola regex como parser completo ni ejecutar el fuente. Una sobrecarga nueva o argumento no comprendido falla con archivo/línea.

Salidas: `blueprints.json`, `engineers.json`, tabla de símbolos/alias y `engineering-catalogue-manifest.json`. Fuente fijada y licencias en una carpeta de datos de importación versionada o paquete de snapshot con hashes recuperable por el comando documentado. `.tmp/catalog-research` es exploración local, no una dependencia reproducible del build.

Cada constructor recibe un `row_id` estable dentro del snapshot (`archivo:línea:ordinal`), texto/hash original, clase, resolución de ingredientes/IDs y referencias a filas de salida. La clasificación debe cubrir **todos** los constructores:

- `ship_recipe`: receta aplicable, incluidos módulos múltiples expandidos y costes Merc Coins.
- `ship_experimental`: experimental aplicable, con todos sus módulos.
- `ship_variant`: variante comprada/preingenierizada; identidad visible y sin receta gratuita.
- `out_of_scope`: traje/arma Odyssey o adquisición broker que no es blueprint/experimental de ingeniería de nave; motivo verificable. Los ingenieros Odyssey sí se importan.
- `obsolete`: filas explícitamente obsoletas de la fuente; por ejemplo `UnknownType` comentadas «wrong data». No convertirlas en recetas ordinarias.
- `unresolved`: dato de nave sin interpretación fiable; bloquea declarar completitud y la activación final del catálogo.

Una fila puede expandirse a varios módulos; el manifiesto debe permitir reconstruir la correspondencia. Igualdad de totales globales no sustituye igualdad de conjuntos de filas. Duplicados solo se fusionan con igualdad de **todos** los costes, contexto, grado y semántica; conservar todos los `row_id`. No fusionar capacidades de ingenieros por clave global.

Pruebas de importación: 100 % de constructores clasificados, cero filas ignoradas sin motivo, cero ingredientes sin correspondencia, cero Merc Coins perdidas, cero referencias de módulo/ingeniero desconocidas y cero conflictos no resueltos. El total final por categoría se fija tras esa auditoría. Los 924/895/83/46/105 solo deben reproducirse al ejecutar el patrón exploratorio original, no imponerse como número final de importación.

La heurística upstream `PreEngineered = Ingredients.Length == 0 && MercCoins == 0` se conserva como indicio, no como verdad del juego. Filas como `cargorack_increasedcapacity` sin ingredientes y variantes de superficie deben revisarse una a una. `cargoracks5c1_extended` y `cargoracks6c1_extended` G5 contienen Merc Coins = 1 pero ningún ingeniero: conservar el coste, mostrar proveedor desconocido y resolver su significado antes de afirmar cobertura jugable completa. No convertir ingeniero vacío en «cualquier ingeniero».

## 4. Contratos de datos y cálculo propuestos

### 4.1 Recetas y elegibilidad

Sustituir el supuesto de receta global de `Blueprint.grades` por recetas de cada contexto. Conservar la clave canónica para nombres e identidad del objetivo. Interfaces propuestas de `data.rs`:

```rust
struct RecipeCost { materials: Vec<Ingredient>, merc_coins: u32 }
struct ModuleSelector { items: Vec<String>, item_prefixes: Vec<String> }
enum BaseIdentity { Blueprint { key: String }, Variant { key: String } }
struct BaseRequirement {
    identity: BaseIdentity,
    selector: ModuleSelector,
    completed_grade: u8,
    source_rows: Vec<String>, // Evidencia de la correspondencia, no solo de ambas identidades.
}
enum EntryPolicy { Stock, ExistingOnly { bases: Vec<BaseRequirement> } }
struct ModuleBlueprint {
    module_type: String,
    selector: Option<ModuleSelector>,
    entry: EntryPolicy,
    grades: BTreeMap<u8, RecipeCost>,
}
struct Blueprint {
    key: String,
    recipes: Vec<ModuleBlueprint>,
    source: String,
    verified: bool,
}
struct Experimental {
    key: String, module_types: Vec<String>,
    cost: RecipeCost, source: String, verified: bool,
}
struct ShipVariant {
    key: String, module_type: String, grade: u8,
    selector: Option<ModuleSelector>,
    journal_blueprints: Vec<String>, // Correspondencias comprobadas; vacío = identidad no resoluble aún.
    source: String, verified: bool,
}
struct BlueprintAlias { module_type: String, alias: String, canonical: String }
enum RecipeResolutionError {
    UnknownBlueprint, IncompatibleItem, MissingBaseMapping, BaseRequired, AmbiguousRecipe,
}
struct BlueprintOption<'a> {
    blueprint: &'a Blueprint,
    availability: Result<&'a ModuleBlueprint, RecipeResolutionError>,
}
// Métodos nuevos/sustitutivos de BlueprintCatalog:
fn blueprint_for_item(
    &self, item: &str, key: &str, current: Option<&CurrentEng>,
) -> Result<(&Blueprint, &ModuleBlueprint), RecipeResolutionError>;
fn blueprints_for_item(&self, item: &str, current: Option<&CurrentEng>) -> Vec<BlueprintOption<'_>>;
fn canonical_blueprint(&self, module_type: &str, raw: &str) -> Option<&str>;
fn canonical_experimental(&self, module_type: &str, raw: &str) -> Option<&str>;
```

`ModuleTypeDef` conserva sus IDs actuales (`engine`, `fsd`) y añade símbolos exactos si el origen los permite. Resolver ítem por símbolo exacto antes que prefijo más largo; dos reglas de igual especificidad con distinto resultado son error de importación. `ModuleSelector` restringe variantes por tamaño/clase/símbolo solo cuando hay evidencia. No adivinar que el nombre `cargoracks5c1` es un símbolo real de `Loadout`. Si falta evidencia para el selector, la fila permanece visible en catálogo como pendiente y no se ofrece para todos los Cargo Rack.

Por cada `(blueprint,module_type,selector)` los grados reales se ordenan y validan en 1..=5. `Stock` requiere cadena desde G1. `ExistingOnly` admite G2–G5, pero requiere base instalada comprobada compatible; no crear G1 vacío, no saltar al primer grado cobrable desde un módulo de serie. Grados ausentes dentro de una ruta requerida producen cálculo incompleto, nunca cero. Una variante de compra se lista con su condición de adquisición y puede identificarse desde `Loadout` cuando existe correspondencia inequívoca; no se simula su precio de compra ni su fabricación.

La correspondencia base→receta vive en `ModuleBlueprint.entry.bases`, dentro del JSON normalizado y del manifiesto. Cada `BaseRequirement` enlaza una identidad canónica de blueprint o variante, símbolos de ítem comprobados y un grado base completo, con `source_rows` que acreditan **la transición**. El selector debe contener al menos un símbolo/prefijo respaldado por evidencia. Para `Variant`, se exige además que `ShipVariant.selector` y `journal_blueprints` permitan reconocer esa variante sin ambigüedad en el ítem y `CurrentEng` observados. Una entrada sin correspondencias se puede listar, pero no reconocer como base instalada ni usar para autorizar otra mejora. Compartir tipo de módulo, nombre parecido o dos filas G1/G2 contiguas no acredita un enlace. Este documento no rellena ningún enlace concreto de compra G1→mejora G2+ pendiente de demostrar.

La resolución reúne todos los contextos de receta que coinciden con clave canónica y elegibilidad de ítem, y evalúa sus requisitos con `current`. `Blueprint` exige coincidencia del blueprint resuelto; `Variant` exige una única variante resuelta. Para un requisito de G*n*, `current.level > n` o `current.level == n && current.quality >= 0.999` acredita grado base completo; calidad/nivel ausentes no lo acreditan. Para continuar una mejora ya aplicada, el catálogo debe contener también el requisito explícito para su blueprint/grado base, respaldado por la cadena de recetas; no deducir la variante de compra anterior desde una mejora posterior.

Resultado: cero contextos de ítem→`IncompatibleItem`; `ExistingOnly` sin enlaces documentados→`MissingBaseMapping`; enlaces conocidos pero ninguna base observada válida→`BaseRequired`; **exactamente un** contexto válido→receta; más de uno→`AmbiguousRecipe`. Si queda un contexto compatible pendiente de correspondencia que podría competir con otro válido, la resolución sigue bloqueada, hasta poder descartar la competencia con evidencia. Nunca elegir el primer contexto ni desempatar por orden de archivo. `blueprints_for_item` conserva las opciones pendientes con su motivo, por lo que la UI puede explicarlas sin habilitar cálculo/pin; el importador rechaza solapamientos de contextos no resueltos antes de declarar completitud.

Resolver alias del journal con contexto de módulo: por ejemplo las reglas de `FindEngineeringFDName` generan formas compuestas y excepciones `AFM`/`CollectionLimpet`; convertirlas en tabla auditable sin reproducir el criterio upstream de «primera coincidencia». Case folding de claves ASCII permitido; cada alias debe resolver inequívocamente dentro de su contexto. No usar etiquetas traducidas como claves internas; `_Localised` sirve para mostrar texto, no para cambiar identidad. Alias desconocido conserva el raw y produce «sin datos». Aplicar la resolución a `Loadout`, `EngineerCraft`, comparación de `CurrentEng` y objetivos cargados.

### 4.2 PlanRequest, PlanCost, moneda y reservas

`PlanRequest` conserva exactamente sus campos persistidos. `Goal` conserva `id`, `item`, `request`, `base` y orden. El cálculo pasa a recibir el ítem para comprobar elegibilidad:

```rust
fn plan_cost(
    catalogue: &BlueprintCatalog, item: &str,
    current: Option<&CurrentEng>, req: &PlanRequest,
) -> PlanCost;
enum PlanBlocker {
    MissingRecipe { grade: u8 }, MissingBaseMapping, BaseRequired, AmbiguousRecipe,
    IncompatibleItem, UnknownBlueprint, UnknownExperimental,
}
struct PlanCost {
    // Mantener per_grade/experimental/total de materiales y replaces_blueprint/verified.
    per_grade: BTreeMap<u8, (u8, BTreeMap<String, u32>)>,
    experimental: Option<BTreeMap<String, u32>>,
    total: BTreeMap<String, u32>,
    merc_coins_by_grade: BTreeMap<u8, u64>,
    merc_coins_experimental: u64,
    merc_coins_total: u64,
    blockers: Vec<PlanBlocker>,
    missing_grades: Vec<u8>, // Compatibilidad de lectura/UI; derivado de blockers.
    replaces_blueprint: bool, verified: bool,
}
struct MercCoinsView { need: u64, have: Option<u64>, missing: Option<u64> }
```

`plan_cost` llama primero a `blueprint_for_item(item, req.blueprint, current)` y traduce un fallo a su `PlanBlocker`, sin sumar recetas de un contexto elegido arbitrariamente. Para una transición `ExistingOnly` aprobada desde otra identidad base, empieza después del `completed_grade` acreditado por el requisito coincidente; si el módulo ya lleva el blueprint objetivo, conserva las reglas actuales de grado/calidad parcial. Varios requisitos cumplidos del mismo contexto que implican puntos de partida diferentes son `AmbiguousRecipe`, no una elección del más barato. Una base no demostrada impide el cálculo completo; no representa un plan de coste cero ni permite fijarlo como calculado. Los datos de la receta siguen consultables en catálogo con el motivo de bloqueo.

Cada coste en moneda se multiplica por las mismas tiradas de su grado; experimental una vez conforme a la regla actual. Usar suma/multiplicación comprobadas. Mantener G1–G4=1/G5=4, calidad parcial y cambio de blueprint ordinario; la transición explícita desde base comprada es la excepción a reiniciar en G1. `verified:false` significa dato sin verificar; **no equivale** a cálculo incompleto. `blockers` no vacío impide porcentaje/aviso de objetivo completo aunque todos los materiales conocidos estén disponibles. Mantener visible la necesidad parcial con su motivo.

No añadir Merc Coins a `materials.json`, `grade_cap`, inventario por categoría o `trader_rules`: no son materiales comerciables. El saldo empieza y permanece `None` mientras no exista una fuente de eventos demostrada/aprobada; no inventar eventos del journal, no asumir cero ni restar costes de `EngineerCraft` para reconstruir saldo desconocido. La UI muestra el total requerido y «saldo sin datos». La infraestructura puede aceptar `Option<u64>` en el cálculo puro sin añadir proveedor de saldo en F4.

Ampliar `GoalView`/detalle de compras con `merc_coins: MercCoinsView`, `materials_ready: bool`, `blockers`. `percent` conserva porcentaje **de materiales** y la etiqueta debe decirlo. `completable = materials_ready && blockers.is_empty() && (need_coins == 0 || have_coins_known_and_sufficient)`. Saldo desconocido con coste positivo impide aviso `GoalCompletable`; no impide fijar un objetivo cuyo cálculo es completo. Sin saldo no puede mostrarse «ya puedes completar» solo por tener materiales. Si en futuro se conoce saldo, reservar monedas por el mismo orden que materiales y dejar el flujo abierto al final; dos objetivos nunca consumen la misma disponibilidad. En F4 las reservas de materiales siguen funcionando aunque el saldo sea desconocido.

### 4.3 Capacidades y recomendación de ingeniero

```rust
struct EngineerBlueprint { module_type: String, blueprint: String, grades: Vec<u8> }
fn pick_engineer(
    module_type: &str, blueprint: &str, required_grades: &[u8],
    book: &EngineerBook, pos: Option<[f64; 3]>,
) -> EngineerPick;
```

`EngineerDef.id` se mantiene si se aprueba el mapa auxiliar. Sus capacidades se generan de cada fila aplicable: unión por `(ingeniero,módulo,blueprint)` con conjunto de grados exacto. `max_grade` puede derivarse para presentación; no usarlo para rellenar capacidades ausentes. Un candidato debe poder aplicar todos los grados efectivamente requeridos por la ruta; si no hay uno, mostrar «sin ingeniero que cubra esta ruta», sin inventar proveedor ni itinerario múltiple. La capacidad para mostrar G5 puede existir aunque la fuente no asigne ese mismo ingeniero a G1; recomendarla solo con una base que requiera esos grados.

Preservar preferencia desbloqueado/cercanía, estado, distancias y siguiente paso actual. Pasar siempre módulo/ruta al flujo, detalles de objetivos, resumen, avisos y panel de ingenieros. Cambiar callback de alertas a `engineer_for_goal: &dyn Fn(&str) -> Option<String>`: recibe `Goal.id`, localiza el `Goal` real y usa `request`/`item`/base; eliminar la inferencia de contexto mediante `split(':')`. No modificar formato de IDs de objetivos para añadir el módulo.

Desbloqueo: `Engineers.cs` contiene textos y requisitos compuestos que `Contribution` simple no modela. Traducir fielmente requisito/invitación y permitir `contribution: None` cuando no es una donación simple verificable; no convertir todos los requisitos de reputación/cadenas en una cantidad de material ficticia. Sección de a pie y requisitos completos visibles; su flujo de recetas sigue fuera del alcance F4.

## 5. Transporte, tamaño y UI

Reutilizar `ModulePageQuery`, `CoreHandle::query_module`, `pagination::page` y el controlador REST existente. No añadir base de datos paralela, polling permanente ni cursores globales por cliente. Todas las respuestas serializadas, envoltorio incluido, deben cumplir **<16.384 bytes**.

Propuesta concreta de topics y recursos (sustituye las formas grandes, manteniendo nombres de topic existentes):

| Topic WS | Contenido acotado | GET REST y secciones |
|---|---|---|
| `engineering.catalogue` nuevo | `{as_of,revision,counts:{module_types,blueprints,recipes,experimentals,variants}}` | `/api/engineering/catalogue`: `module_types`, `blueprints`, `recipes`, `experimentals`, `variants`, `aliases` |
| `engineering.engineers` | `{as_of,revision,total,selected_id}` | `/api/engineering/engineers`: `engineers`, `capabilities`, `unlock` |
| `engineering.flow` | `{as_of,revision,status,slot,module_type,item,current,request,percent,materials_ready,merc_coins,blockers,engineer,counts}`; opciones/costes por fila en REST | `/api/engineering/flow`: `blueprints`, `experimentals`, `grades`, `costs`, `lines` |

Cada GET usa el mismo contrato existente `?revision=<u64>&cursor=<usize>&section=<name>` y respuesta `{revision,section,cursor,next_cursor,total,items}`. Secciones por defecto: `blueprints`, `engineers`, `lines` respectivamente. Mantener 400/404/409/413/503, auth y `Cache-Control: no-store`. Añadir rutas explícitas, allowlists de diagnóstico y fixtures Rust→TypeScript.

Las filas no contienen listas ilimitadas: recetas se aplanan por `(blueprint,módulo,selector,grado)`; capacidades por `(ingeniero,módulo,blueprint,grado)`; costes por `(grado,material)` o `(grado,merc_coins)`. Separar requisitos largos por campo/paso en `unlock`. Comprobar límite por fila durante importación y tests. Una fila imposible de servir devuelve 413 y error visible; no truncar ni avanzar cursor. La importación de un artefacto cuya fila no cabe debe fallar en `--check`, para no descubrirlo solo en producción.

Mantener la revisión monotónica existente para todos los recursos: cambiar comandante, selección, base/cálculo, inventario o progreso de ingeniero invalida páginas; `as_of` solo no. Puede ser conservador e invalidar catálogo estático al cambiar otros detalles, evitando introducir varias generaciones de sincronización. Los datos estáticos se cargan una vez y se comparten; se cachean solo filas calculadas necesarias. Revisar coste de regenerar páginas en `refresh` con el catálogo grande; no serializar todo el catálogo en cada evento.

Los clientes de páginas se reutilizan por recurso/sección; descartan respuestas de revisión anterior, cancelan al desmontarse/cambiar comandante/ranura y preservan sus cursores locales. GET no modifica la selección global del actor. UI reconstruye todas las páginas antes de habilitar una elección que depende del conjunto; un fallo muestra estado recuperable. El flujo muestra solo grados realmente aplicables y tiradas para esos grados; nunca la lista fija `[1,2,3,4,5]` para una receta G2+. El panel de ingenieros muestra sus capacidades con módulo y grados. Probar los casos de 16 KiB con catálogo real completo y nombres UTF-8/escapados, no con los cinco ingenieros provisionales.

Si tras aplanar se demuestra con bytes que algún resumen propuesto sigue creciendo sin límite, trasladar ese campo a la sección de detalle antes de activar el catálogo. Los topics de nave/inventario/sources también se vuelven a medir; no asumir que la ampliación carece de efecto en otros topics.

## 6. Persistencia y nombres

Conservar `engineering-001` y la tabla `engineering_goals`; no hace falta migración SQL por actualizar recetas que se recalculan. Deserializar los JSON antiguos sin nuevos campos obligatorios. Conservar **exactamente** los IDs ya persistidos al leer/escribir/reordenar/eliminar. Canonicalizar claves para resolver cálculos, no reescribir `id` ni aplicar reemplazos de texto sobre él. Al fijar de nuevo una selección equivalente, comparar identidad semántica resuelta `(ship,slot,module_type,blueprint,experimental,grade)` para sustituir en su posición conservando el ID anterior; un grado distinto conserva comportamiento de objetivo distinto. Respetar máximo 20 y escritura atómica actual por comandante.

Un objetivo incompatible con el nuevo catálogo no se borra ni aparece vacío/completable: conservarlo, mostrar motivo y permitir eliminarlo. Las necesidades se recalculan con la fuente nueva; no preservar costes provisionales incorrectos para que pasen tests. Lecturas tardías de almacén siguen respetando operaciones concurrentes de pin/unpin/move.

Generar claves `bp-`, `exp-`, `modtype-`, `unlock-` y nombres de variantes donde corresponda en cinco idiomas. Reutilizar nombres oficiales de EDEngineer ya aprobados cuando el mapeo canónico es inequívoco, sin importar sus recetas por asociación. Fuente EDDiscovery aporta nombre inglés; idioma sin traducción oficial conocida usa ese inglés con `.pending = 1`, nunca una traducción inventada presentada como oficial. Conservar procedencia de nombres/alias y comprobar igualdad de claves entre idiomas. Añadir textos de Merc Coins, saldo desconocido, base requerida, receta incompleta y proveedor desconocido en `engineering.ftl` de los cinco idiomas. El nombre «Merc Coins» se mantiene como nombre de la fuente hasta comprobar su denominación oficial por idioma; marcar pendiente cuando proceda.

## 7. Plan de ejecución por fases tras aprobación

1. **Ratificar cambios documentales y mapa de IDs.** Actualizar spec §§3.3–3.5/4/7/8 y §10.2 para coste adicional, base existente, completitud y paginación. Actualizar Interfaces/Tests de Tasks 9–11/14–16/18–24/26/28 y status. Registrar aprobación separada del mapa EDDI y decisión ante filas ambiguas. Un commit documental. No reabrir trabajo correcto de Tasks 26–27 por simple renumeración.
2. **Auditar fuente y conversor, RED→GREEN.** Snapshot, hashes, manifiesto exhaustivo de constructores, cruce de los 38 IDs, mapeo completo de ingredientes y módulos, aliases y categorías. Resolver filas `unresolved` en alcance; no activar datos parciales como catálogo completo. Commit de herramienta/documentación y atribución. No incorporar balances ni acceso de red nuevo al producto.
3. **Ampliar contratos con catálogo provisional, RED→GREEN.** Modelos de costes/recetas, resolución de alias/ítem, capacidades por módulo/grado, cálculo con bloqueos, goals/avisos y compatibilidad persistida. Añadir fixtures sintéticos de nuevas estructuras para aislar fallos del cambio de datos. Commit por tarea lógica, checks globales y actualización de status.
4. **Extender páginas y consumidores, RED→GREEN.** REST/actor, tres topics, tipos TS, clientes de páginas, controles por grados, moneda y nombres pendientes. Generar contratos Rust→UI. Dos clientes, invalidación y límites de bytes; QA visual 2560 y 800. No activar aún un catálogo que la UI no puede representar correctamente.
5. **Importar catálogo completo en una entrega coherente.** Reemplazar datos provisionales, generar nombres, actualizar todos los oráculos independientes/fixtures dependientes. Auditoría final por fila/categoría, IDs y licencias. Commit separado de Tasks 9–10 de completitud; no confundir cada grado con un blueprint distinto.
6. **Revalidar cierre F4.** Revisión contra spec/plan modificados, e2e/restart/multicliente, checks Rust/UI/clippy release, consumo en disposición Ingeniería con catálogo completo. Registrar números finales y evidencias. Task 28 solo cierra cuando las filas pendientes del alcance y los diez criterios están resueltos.

Si antes de fases 3–5 aparece una contradicción de datos o de alcance que impida representar alguna fila actual, registrar bloqueo y propuesta concreta; no ampliar silenciosamente F4 hacia un motor de adquisición o ingeniería de Odyssey.

## 8. Criterios de validación y oráculos concretos

- **Importador:** salida determinista byte a byte con entrada idéntica; `--check` detecta un constructor desconocido, SHA alterado, ingrediente sin mapear, alias ambiguo, coste Merc Coins perdido, módulo sin selector comprobado y fila en alcance sin clasificación. Igualdad por `row_id` y por expansión de arrays; números finales por categoría fijados en manifiesto y tests después de auditar todas las sobrecargas.
- **Receta verificada en el fuente fijado, no en el juego:** `engine_dirty`: G1 `1SLF`; G2 `1SLF,1ME`; G3 `1SLF,1Cr,1MC`; G4 `1MCF,1Se,1CCom`; G5 `1CIF,1Cd,1PI`. `special_engine_overloaded`: `5Fe,3HC,1SFP`. Con `[1,1,1,1,4]`, total manual: `legacyfirmware=3`, `mechanicalequipment=1`, `chromium=1`, `mechanicalcomponents=1`, `consumerfirmware=1`, `selenium=1`, `configurablecomponents=1`, `industrialfirmware=4`, `cadmium=4`, `pharmaceuticalisolators=4`, `iron=5`, `hybridcapacitors=3`, `securityfirmware=1`; Merc Coins = 0. No Carbon/Polonium provisional. Contrastar nombres de materiales con `MCMRType.cs` y catálogo existente; mantener `verified:false`.
- **Mismo blueprint G3 completo→G5:** solo G4×1/G5×4; sin experimental, seis claves con totales `consumerfirmware=1`, `selenium=1`, `configurablecomponents=1`, `industrialfirmware=4`, `cadmium=4`, `pharmaceuticalisolators=4`. G5 calidad 0,5 y cuatro tiradas→dos recetas G5. Alias/case distintos resuelven al mismo blueprint.
- **Operations con moneda:** `abrasionblaster_farreaching` contiene recetas G2/G3/G4/G5 con Merc Coins 5/5/10/20 respectivamente. Sobre una base compatible demostrada y con tiradas `[1,1,1,1,4]`, el coste de G2–G5 es **100 Merc Coins**. Sin correspondencia documentada: `MissingBaseMapping`; con correspondencia conocida pero módulo de serie: `BaseRequired`. Ninguno crea G1 gratuito ni objetivo completable. Este test comprueba multiplicación desde fuente; la relación con una variante G1 concreta requiere evidencia adicional.
- **Fila con proveedor desconocido:** Cargo Rack extended G5 conserva Merc Coins = 1; no aparece un ingeniero ficticio. La auditoría debe resolver si el dato es aplicable/placeholder antes de cierre, sin ocultar la fila ni certificarla por tener coste positivo.
- **Capacidad real:** Lori/`misc_shielded`/AFMU permite G4; Lori/`misc_shielded`/Cargo Scanner no permite G4 y sí G3. La recomendación, tabla de ingeniero, objetivo y aviso usan el mismo resultado. No inferir que toda capacidad ≤ máximo existe.
- **Variantes/alias/módulos:** todas las filas de variante preservadas sin coste de ingeniería cero; exact match precede prefijo largo; tipos desconocidos/alias ambiguos muestran sin datos. Caso de sobrecarga array `weapon_highcapacity` MissileRack/SeekerMissileRack conserva ambos módulos y sus grados/capacidades.
- **Correspondencia base→receta:** fixture sintético, identificado como tal: variante `variant_test` de G1 con símbolo exacto y alias journal comprobables en el propio fixture; requisito explícito de esa variante→blueprint `upgrade_test` G2–G5. Base observada G1 completa habilita G2; de serie o G1 parcial→`BaseRequired`; borrar el enlace→`MissingBaseMapping`; dos contextos válidos o dos puntos de partida distintos→`AmbiguousRecipe`. Cambiar orden de filas no cambia el resultado. El fixture no certifica que esa transición exista en el juego ni añade un enlace real al catálogo.
- **Moneda y completitud:** materiales suficientes + 100 monedas requeridas + saldo `None`→`materials_ready=true`, `completable=false`, sin aviso de objetivo completo. Receta sin monedas conserva la semántica anterior. En prueba pura con saldo 100 y objetivos de 60/60, reservas 60/40, segundo faltan 20; intercambios nunca ofrecen monedas. Bloqueo de receta impide `completable` aunque total conocido esté vacío.
- **Persistencia:** base SQLite creada con JSON anterior, alias antiguo e ID original→carga, recálculo, reorder, reemplazo semántico y reinicio sin pérdida/duplicado/cambio de ID; comandante B no ve objetivos de A; desconocido conservado con diagnóstico. Ninguna migración destructiva.
- **Páginas:** catálogo final entero, 38 ingenieros con todas sus capacidades, 60 módulos y 20 objetivos; recorrer cada recurso/sección devuelve igualdad de filas sin pérdida/duplicados. Página 16.383 bytes aceptada; 16.384 rechazada; UTF-8 y comillas escapadas incluidas. Revisión obsoleta 409, dos clientes independientes, GET sin mutación, sin tormenta WS/polling. No cerrar por pasar solo el fixture pequeño.
- **Nombres y cierre:** cobertura exacta de claves en es/en/fr/de/pt y fallback inglés marcado; actualizar oráculos/contratos UI sin derivar todo del mismo conversor. Ejecutar checks obligatorios completos, e2e y medición <150 MB/<1 % CPU con catálogo completo; medir frecuencia UI ≤10/s y ausencia de trabajo continuo en reposo. No se han ejecutado estas validaciones al redactar esta propuesta.

## 9. Cambios precisos pendientes de aprobación

La aceptación técnica de esta propuesta autoriza definir los contratos anteriores en spec/plan y ejecutarlos por fases: costes Merc Coins visibles sin inventar saldo, bases existentes para mejoras G2+, variantes diferenciadas, capacidades por módulo/grado, paginación de catálogo/ingenieros/flujo y compatibilidad de objetivos. La aprobación de EDDI se solicita **por separado**, limitada al mapa ID/nombre fijado y sus avisos de licencia. Las filas fuente ambiguas y símbolos/bases aún no demostrados siguen siendo preguntas de datos que deben resolverse antes de declarar completitud; no quedan aprobados por aceptar la arquitectura.
