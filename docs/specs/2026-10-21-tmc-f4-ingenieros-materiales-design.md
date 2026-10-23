# TMC · Especificación de F4 — Ingenieros y materiales

- **Estado:** ✅ aprobada por el propietario (2026-10-21); enmienda de catálogo completo y mapa auxiliar de IDs aprobados expresamente (2026-10-22).
- **Fecha:** 2026-10-21
- **Enmienda contractual vigente:** [catálogo completo, costes, bases y paginación](../plans/2026-10-22-tmc-f4-catalogo-completo-propuesta.md), aprobada con «Aprobar y ejecutar la enmienda» y «Aprobar el mapa auxiliar de IDs». Sus §§3–6/8 concretan los contratos de esta spec; ejecución en C0–C10 del plan activo, tras Tasks 26 y 27.
- **Depende de:** arquitectura F0, F1 (planificador, datos propios con procedencia, `api_cache`, opt-out), F2 (Ardent `NearestService`, `ship_pads.json`), F3 (tabla de coordenadas, `core.clock`, disposiciones por relevancia, contrato JSON Rust→UI) y el interruptor Auto-companion (`2026-10-21-tmc-auto-companion-design.md`). Diagnóstico debug: regla global de registro y canarios.
- **Maquetas aprobadas (2026-10-21):** `f4-ingenieria.html` y `f4-materiales.html` (sesión de diseño en `.superpowers/brainstorm/`). La descripción escrita es la referencia.

---

## 1. Intención

Que ingenierizar deje de ser una hoja de cálculo aparte: eliges un módulo de tu nave, lo que quieres aplicarle y hasta qué grado, y TMC te dice exactamente qué materiales necesitas, cuáles tienes, cuáles puedes cambiar con tus excedentes, dónde conseguir el resto y qué ingeniero cercano te lo hace. Además, vigila tu inventario para que no pierdas materiales por tener el máximo.

## 2. Objetivos, no objetivos y antiobjetivos

**Objetivos**
- Disposición **«Ingeniería»** con: inventario de materiales (nave y taquilla de a pie), ingenieros, mi nave modificada con el flujo **ingenierizar un módulo**, lista de la compra, planificador de intercambios, dónde conseguirlo y avisos.
- Catálogo completo de blueprints y experimentales de nave, incluido Operations y sus costes Merc Coins; distinguir variantes compradas/preingenierizadas de recetas fabricables, con auditoría reproducible de todas las filas de las fuentes fijadas.

**No objetivos**
- Ingeniería de trajes y armas de Odyssey (se listan los ingenieros de a pie y la taquilla, pero sin flujo de blueprints de traje/arma).
- Fleet carrier, colonización, Powerplay, BGS (F5). Envío de datos (F6).
- Simular valores exactos de las tiradas (solo se estiman cantidades de materiales por número de tiradas).

**Antiobjetivos**
- Recetas incorrectas presentadas como ciertas: todo dato estático lleva procedencia y `verified`, y lo no verificado se marca en la UI.
- Incorporar datos o código con licencia incompatible (GPL/AGPL; p. ej. no copiar tablas de proyectos GPL).
- Proponer intercambios que gasten materiales reservados por otro objetivo fijado.
- Presentar como completo un coste que omite moneda, una receta sin correspondencia de base, un proveedor inventado o un dato pendiente de interpretación. `verified:false` y cálculo incompleto son estados distintos.

## 3. Experiencia de usuario

### 3.1 Disposición y relevancia
- **«Ingeniería»** se activa sola (si Auto-companion está encendido) al atracar en el taller de un ingeniero, al recibir `EngineerCraft`/`EngineerProgress` o al abrir el flujo de ingenierizar. Prioridad: manual > combate > minería > ingeniería > misiones > comercio > exploración.

### 3.2 Inventario de materiales
- Materias primas, fabricados y codificados por grado, con cantidad frente al máximo del grado (300/250/200/150/100) y aviso «al máximo: lo que recojas se pierde».
- Pestaña **taquilla** (Odyssey): artículos, componentes, consumibles y datos desde `ShipLocker`/`BackpackChange`.
- Cambios de la sesión: recogidos, usados, intercambiados, con su origen (misión, escaneo, minería, intercambio, síntesis, ingeniería).

### 3.3 Ingenieros
- Lista con estado (desbloqueado / invitado / conocido), rango y progreso de `EngineerProgress`, sistema y distancia desde tu posición, y capacidades por **tipo de módulo + blueprint + conjunto de grados**; un máximo visible no autoriza rellenar grados ausentes.
- Siguiente paso de desbloqueo del ingeniero elegido (requisito, invitación, donación) con lo que tienes.
- Sección de ingenieros de a pie (Odyssey) con su estado.
- Mantener IDs numéricos del journal y selección `{id}`; catálogo de 38 ingenieros con mapa ID/nombre EDDI aprobado, cruce exacto 38/38 e IDs únicos. Los requisitos compuestos conservan su texto; no se convierten en donaciones simples ficticias.

### 3.4 Mi nave modificada e **ingenierizar un módulo** (petición del propietario)
1. Lista de módulos de la nave actual (`Loadout`) con blueprint, grado, calidad y experimental actuales, o «de serie».
2. Al elegir un módulo: blueprints válidos para su ítem/tipo y base actual, experimental opcional y solo grados aplicables (G5 por defecto si está disponible). Identidades/alias del journal se resuelven con contexto de módulo; opciones con base pendiente o ambigua explican el motivo y no habilitan cálculo completo ni fijación.
3. Cálculo ordinario desde el grado actual: de serie → desde G1; mismo blueprint G3 completo → desde G4; otro blueprint → desde G1 con aviso. Excepción explícita: mejoras `ExistingOnly` G2+ requieren correspondencia `BaseRequirement` entre blueprint/variante, ítem y grado base completo; parten después de esa base comprobada. Sin enlace → `MissingBaseMapping`; sin base observada válida → `BaseRequired`; varios contextos/puntos de partida → `AmbiguousRecipe`. Nunca G1 gratuito ni primera coincidencia arbitraria.
4. **Materiales totales** por grado y sumados, con **tiradas por grado ajustables** (por defecto: G1–G4 = 1, G5 = 4; se recuerdan en ajustes) y experimental una vez. Por material: necesarios, tienes, estado (OK / cubierto con intercambio concreto / falta N). **Merc Coins por separado**, por grado y totales, multiplicadas por las mismas tiradas; no son materiales ni admiten intercambio. El saldo permanece `None`/«sin datos» en F4: no se inventan eventos ni se reconstruye por consumo.
5. Porcentaje de materiales listos y resumen de costes/bloqueos. Coste no verificado conserva su aviso; receta ausente/base desconocida impide afirmar completitud aunque los materiales conocidos estén cubiertos. Variantes compradas se listan con su condición; sin precio de compra o fabricación simulados.
6. **Dónde conseguir lo que falta**: comerciante de materiales más cercano del tipo necesario (materias primas, fabricados, codificados), con distancia y plataforma, y el intercambio concreto; pistas de obtención del material; broker de tecnología si aplica.
7. **Ingeniero** más cercano capaz de aplicar ese blueprint para ese módulo y **todos los grados requeridos de la ruta**, con su estado; si no está desbloqueado, qué falta. Sin candidato que cubra la ruta, mostrar «sin datos» de proveedor/ruta; no fabricar capacidades ni itinerarios múltiples.
8. Botón «Añadir a la lista de la compra».

### 3.5 Lista de la compra
- Objetivos fijados (módulo + blueprint + experimental + grado + tiradas), guardados entre sesiones. Necesidades agregadas, porcentaje **de materiales** por objetivo, lo que falta y lo que sobra; coste Merc Coins y saldo conocido/desconocido separados.
- Aviso «ya puedes completar X» solo con materiales cubiertos, sin bloqueos de cálculo y, si cobra Merc Coins, saldo conocido suficiente. En F4 el saldo desconocido impide ese aviso para recetas con moneda. El ingeniero del aviso se obtiene del objetivo real, con ítem/módulo/base; no analizando su ID.
- Los materiales reservados por un objetivo no se ofrecen como excedente a otro.
- Conservar tabla `engineering-001`, JSON anterior, IDs y orden; resolver alias sin reescribir IDs persistidos. Fijar una selección semánticamente equivalente sustituye en su posición preservando ID. Un objetivo incompatible se conserva con motivo y se puede eliminar; nunca se convierte en objetivo vacío completable.

### 3.6 Planificador de intercambios
- Para lo que falta, propone intercambios desde excedentes con las reglas del comerciante: misma categoría subir 1 grado 6:1, bajar 1 grado 1:3; entre categorías con la penalización del juego; muestra das/recibes y comerciante cercano.

### 3.7 Dónde conseguirlo
- Pistas de obtención por material (dato propio con procedencia): fuente principal (minería, HGE/señales, escaneo de datos, misiones, sitios concretos).

### 3.8 Avisos
- Material al máximo; objetivo completable; ingeniero desbloqueado o invitación nueva; material escaso de una lista conseguido.

## 4. Arquitectura

- Módulo nuevo **`engineering`** (`crates/tmc-modules/src/engineering/`, `ui/src/modules/engineering/`): inventario, ingenieros, nave, cálculo de ingenierizar, lista de la compra, intercambios, avisos. Registro en `all_modules()`, `MODULE_PREFIXES`, allowlists de diagnóstico y contrato JSON Rust→UI.
- **`tmc-model`** tipa: `Materials`, `MaterialCollected`, `MaterialDiscarded`, `MaterialTrade`, `Synthesis`, `EngineerCraft`, `EngineerProgress`, `EngineerContribution`, `TechnologyBroker`, `ShipLocker`, `BackpackChange`, recompensas de materiales ya tipadas (F3), y el bloque `Engineering` de los módulos de `Loadout` (ya parcialmente usado en F1).
- **Datos estáticos propios** (`crates/tmc-modules/data/`): `materials.json` (clave, categoría, grado, máximo, pistas de obtención), `blueprints.json` (recetas por contexto de módulo/ítem/grado, materiales y Merc Coins, experimentales, variantes, requisitos explícitos de base y alias), `engineers.json` (ID, nombre, sistema, coordenadas, capacidades por módulo/blueprint/grados y desbloqueo ship/a pie), `trader_rules.json` (tasas de materiales). Procedencia y `verified` por dato; EDDiscovery Apache-2.0 fijado como fuente A y EDDI Apache-2.0 fijado **solo** para ID/nombre (§10.3). Importador Node sin dependencias nuevas, entrada fijada/hash y manifiesto exhaustivo por constructor/fila; no descargas en build/test/runtime ni ejecución del C# de origen. Una fila en alcance no resuelta bloquea declarar catálogo completo; los recuentos exploratorios parciales no certifican cobertura.
- **Resolución:** símbolo exacto de ítem antes que prefijo más largo; alias con contexto y sin ambigüedad; recetas/base según `CurrentEng`. Carga de datos corruptos tolerante en ejecución, pero conversor/`--check` estricto sobre artefactos que se activan. Grados ausentes no tienen coste cero. Contratos completos en enmienda §4.
- **Persistencia**: `engineering-001` y campos previos de objetivos sin migración SQL destructiva; tiradas por defecto en `config.toml`. Necesidades recalculadas con el catálogo vigente, no con cantidades provisionales preservadas.
- **Listas grandes (aprobado 2026-10-21 y extendido 2026-10-22):** objetivos, taquilla, catálogo, ingenieros y flujo publican resúmenes con revisión por WebSocket; detalles se consultan al actor mediante REST paginado por bytes, sin leer SQLite para reconstruir cálculos vivos. Recursos `/api/engineering/{goals,locker,catalogue,engineers,flow}` y secciones definidas en enmienda §5; cada topic/página serializados con envoltorio **<16.384 bytes**. Recetas/capacidades/costes se aplanan por fila para que tampoco haya una fila individual ilimitada. Revisión monotónica invalida al cambiar datos, selección o comandante, nunca solo por `as_of`; cursores privados por cliente y respuestas obsoletas descartadas. Se mantienen todas las filas sin truncado silencioso/polling; GET no modifica selección y conserva auth/no-store/400/404/409/413/503 existentes.
- **Red**: Ardent `NearestService` para comerciantes de materiales (tres tipos) y broker de tecnología (confirmar en el spike); misma infraestructura (limitador, salud, `api_cache`, opt-out, nunca en Legacy ni replay).

## 5. Servicios externos

| Servicio | Uso | Límite | Caché |
|---|---|---|---|
| Ardent | Comerciante de materiales / broker más cercanos | 1 petición/s | 1 h |
| Spansh (respaldo) | Búsqueda de estaciones con servicio si Ardent no lo ofrece | límite existente de F1 | 1 h |

Solo se envía el sistema actual y el tipo de servicio. Sin servicio, los comerciantes cercanos muestran «sin datos» y el resto funciona (las distancias a ingenieros salen de `engineers.json` y tu posición).

## 6. Rendimiento y robustez
- Cálculos puros y baratos; topics solo al cambiar, sin campos dependientes del reloj.
- Eventos desconocidos o materiales no catalogados no rompen nada (se muestran con su nombre del diario y «sin datos»).

## 7. i18n
- Textos de la UI en `locales/*/engineering.ftl` (5 idiomas).
- Nombres de materiales desde `_Localised` del diario con respaldo propio.
- Nombres de blueprints y experimentales: **traducción oficial del juego** en cada idioma de la UI (decisión del propietario, 2026-10-21: «en español, como en el juego»). Las claves internas del diario (`BlueprintName`, `ExperimentalEffect`) se mapean a nombres por idioma en un archivo propio de localización de datos (`locales/*/engineering-data.ftl` o tabla por idioma), recopilado de fuentes públicas con procedencia. Idioma sin traducción conocida → nombre inglés del juego, marcado como pendiente.
- Aplicar la misma cobertura/procedencia a variantes y requisitos. Reutilizar nombres oficiales EDEngineer ya autorizados cuando su mapeo canónico sea inequívoco, sin importar sus recetas. Textos de moneda, saldo desconocido, base requerida, ambigüedad y receta incompleta en los cinco idiomas. Etiqueta `Merc Coins` con respaldo inglés marcado mientras falte denominación oficial por idioma; ninguna traducción inventada presentada como oficial.

## 8. Criterios de aceptación de F4
1. Checks de Rust y UI en verde (incluidos e2e y clippy release).
2. Inventario: con un fixture, cantidades por categoría/grado iguales al evento `Materials` más los cambios posteriores; aviso al máximo.
3. Ingenieros: estado, rango y progreso desde `EngineerProgress` del fixture; distancia desde la posición; cruce exacto 38/38 de IDs y capacidades por módulo/grado. Lori/`misc_shielded` admite G4 para AFMU y solo hasta G3 para Cargo Scanner, también en recomendaciones/objetivos/avisos.
4. Ingenierizar: «propulsores de serie → Motor sucio G5 + Drag Drives», tiradas por defecto, coincide con oráculo manual del fuente fijado (enmienda §8); ya en G3 completo cuenta desde G4. Operations conserva Merc Coins: Far-reaching G2/G3/G4/G5 = 5/5/10/20 por tirada, total 100 para G2–G5 con defaults sobre base demostrada. Sin correspondencia/base o con ambigüedad, cálculo bloqueado; variantes sin receta no se fabrican gratis. Saldo desconocido no permite aviso de completable para coste con moneda.
5. Intercambios: propuestas con las tasas correctas y sin tocar materiales reservados por otro objetivo.
6. Comerciante más cercano con un Ardent falso; «sin datos» sin servicios.
7. Lista de la compra persiste entre reinicios y avisa cuando un objetivo es completable; JSON/IDs/orden antiguos conservados y reemplazo semántico sin duplicados. Objetivos desconocidos conservan motivo y no se vuelven completable por coste vacío.
8. Catálogo completo con procedencia, `verified`, manifiesto por fila, hashes e importación determinista verificable mediante `--check`; nada GPL/AGPL incorporado; `THIRD_PARTY_NOTICES.md` al día. Cobertura de todas las sobrecargas/arrays, experimentales y variantes, con exclusiones motivadas y cero filas de nave no resueltas al cierre. Todos los topics/páginas <16.384 bytes, sin pérdidas, con invalidación y dos clientes independientes.
9. Todas las cadenas nuevas en los 5 idiomas; nombres de blueprints y experimentales con la traducción oficial del juego (respaldo inglés marcado).
10. `measure-footprint.ps1` < 150 MB y < 1 % CPU en reposo con la disposición «Ingeniería».

## 9. Riesgos y spikes previos

| Riesgo | Mitigación |
|---|---|
| Licencia y exactitud de recetas de blueprints, ingenieros y pistas de obtención | **Spike de datos** (decisión del propietario): **(A)** comprobar primero datasets comunitarios con licencia permisiva (p. ej. Coriolis, EDEngineer): si la licencia del dato es MIT o equivalente compatible, se importan con procedencia y aviso en `THIRD_PARTY_NOTICES.md`; **(B)** si no hay ninguno utilizable, datos propios recopilados a mano como hechos del juego desde fuentes públicas, sin copiar tablas. Excluido siempre: GPL/AGPL y wikis CC BY-SA. El spike deja escrito en `docs/research/` qué fuente se usa y por qué. |
| Traducciones oficiales de blueprints y experimentales | Mismo spike: localizar fuentes públicas de los nombres del juego en es/fr/de/pt con licencia compatible o recopilarlos como hechos; registrar procedencia. |
| Formato del bloque `Engineering` de `Loadout` y de `EngineerProgress`/`ShipLocker` | **Spike de diario** con los diarios del propietario (solo agregados) y documentación. |
| Endpoints de Ardent para comerciantes de materiales y broker | Incluido en el spike; si no existen, Spansh (búsqueda de estaciones); si tampoco, «sin datos» y desviación documentada. |

## 10. Decisiones

### 10.1 Del propietario (2026-10-21)
1. Fuente de recetas: comprobar (A) datasets con licencia permisiva; si no se puede, (B) datos propios recopilados a mano (§9).
2. Alcance: **todos** los blueprints y experimentales de módulos de nave en F4, con `verified:false` hasta comprobarlos.
3. Nombres de blueprints y experimentales en el idioma de la UI con la traducción oficial del juego (§7).

### 10.2 Del redactor (a revisar al leer)
1. Un módulo `engineering` (no dos).
2. Tiradas por defecto G1–G4 = 1 y G5 = 4, ajustables y recordadas.
3. Cambiar de blueprint ordinario en un módulo ya modificado cuenta desde G1 con aviso; excepción aprobada `ExistingOnly` con `BaseRequirement` explícito y comprobado (§3.4).
4. **Costes necesarios:** cada tirada consume materiales y Merc Coins de la receta de su grado una vez; experimental una vez. Grado parcial cuenta tiradas restantes, sin modelar valores exactos. Saldo Merc Coins `None` en F4; separado del inventario/intercambios y sin evento inventado. Cálculo completo y `verified` son dimensiones distintas.
5. **Intercambios:** solo dentro del tipo de comerciante (materias primas, fabricados, codificados). Misma familia: subir 1 grado 6:1, bajar 1 grado 1:3; otra familia del mismo tipo con la penalización del juego (tasas exactas en `trader_rules.json`, confirmadas en el spike). Primero los de la misma familia; se elige la propuesta que gasta menos excedente.
6. **Reservas** de la lista de la compra por orden de los objetivos fijados (reordenable); lo reservado por un objetivo anterior no es excedente para los siguientes.
7. **Inventario:** manda la última instantánea `Materials` (y `ShipLocker` para la taquilla) más los cambios posteriores; la siguiente instantánea resincroniza.
8. **Persistencia:** migración propia del módulo `engineering-001` para la lista de la compra; tiradas por defecto en `config.toml`.
9. **Comerciantes cercanos:** Ardent primero; si no ofrece comerciantes de materiales o broker, búsqueda de estaciones de Spansh (spike); sin servicios, «sin datos» y desviación documentada.

### 10.3 Del propietario (2026-10-22) — catálogo completo

1. **Fuente A:** EDDiscovery/EliteDangerousCore fijado a `7d30920e6822d4836e0d6dd5c91026ec7e2ab4c7`; archivos de recetas, ingenieros y mapas auxiliares Apache-2.0 documentados en la investigación/enmienda. Todos los blueprints/experimentales de nave incluidos Operations; importados siguen `verified:false` hasta comprobación.
2. **Mapa auxiliar aprobado expresamente:** solo ID/nombre de EDCD/EDDI `Engineer.cs` a `d3b964ea7c8bb959ad6537f55b308f293a326905`, con licencia/atribución. Auditoría exploratoria exacta 38/38; repetir en C1. Se conservan `u64`/acciones numéricas; sin nueva identidad/clase de ingeniero.
3. **Enmienda aprobada para ejecutar:** contratos de costes/bases, resolución única, capacidades por módulo/grado, páginas y compatibilidad de objetivos de [enmienda §§3–6/8](../plans/2026-10-22-tmc-f4-catalogo-completo-propuesta.md). No se certifican enlaces de base/símbolos ni balances todavía no demostrados.
4. **Orden:** cerrar Task 26 revisada y Task 27 vigentes, ejecutar C1–C9 del plan y cerrar Task 28 mediante C10. Las tareas hechas conservan su historia; las adaptaciones se siguen por C y no se renumeran.

## 11. Ramas y entregas
- Rama `fase/f4-ingenieros-materiales` desde `main`.
- Bloques sugeridos: 0) spikes de datos/licencias, diario y Ardent; 1) eventos tipados y datos estáticos; 2) inventario, ingenieros y nave; 3) ingenierizar, lista de la compra, intercambios y dónde conseguir; 4) UI; 5) disposición, ajustes, fixtures, e2e, eval y cierre.
