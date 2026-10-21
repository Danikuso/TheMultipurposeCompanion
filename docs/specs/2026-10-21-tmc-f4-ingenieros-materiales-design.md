# TMC · Especificación de F4 — Ingenieros y materiales

- **Estado:** ✅ aprobada por el propietario (2026-10-21)
- **Fecha:** 2026-10-21
- **Depende de:** arquitectura F0, F1 (planificador, datos propios con procedencia, `api_cache`, opt-out), F2 (Ardent `NearestService`, `ship_pads.json`), F3 (tabla de coordenadas, `core.clock`, disposiciones por relevancia, contrato JSON Rust→UI) y el interruptor Auto-companion (`2026-10-21-tmc-auto-companion-design.md`). Diagnóstico debug: regla global de registro y canarios.
- **Maquetas aprobadas (2026-10-21):** `f4-ingenieria.html` y `f4-materiales.html` (sesión de diseño en `.superpowers/brainstorm/`). La descripción escrita es la referencia.

---

## 1. Intención

Que ingenierizar deje de ser una hoja de cálculo aparte: eliges un módulo de tu nave, lo que quieres aplicarle y hasta qué grado, y TMC te dice exactamente qué materiales necesitas, cuáles tienes, cuáles puedes cambiar con tus excedentes, dónde conseguir el resto y qué ingeniero cercano te lo hace. Además, vigila tu inventario para que no pierdas materiales por tener el máximo.

## 2. Objetivos, no objetivos y antiobjetivos

**Objetivos**
- Disposición **«Ingeniería»** con: inventario de materiales (nave y taquilla de a pie), ingenieros, mi nave modificada con el flujo **ingenierizar un módulo**, lista de la compra, planificador de intercambios, dónde conseguirlo y avisos.

**No objetivos**
- Ingeniería de trajes y armas de Odyssey (se listan los ingenieros de a pie y la taquilla, pero sin flujo de blueprints de traje/arma).
- Fleet carrier, colonización, Powerplay, BGS (F5). Envío de datos (F6).
- Simular valores exactos de las tiradas (solo se estiman cantidades de materiales por número de tiradas).

**Antiobjetivos**
- Recetas incorrectas presentadas como ciertas: todo dato estático lleva procedencia y `verified`, y lo no verificado se marca en la UI.
- Incorporar datos o código con licencia incompatible (GPL/AGPL; p. ej. no copiar tablas de proyectos GPL).
- Proponer intercambios que gasten materiales reservados por otro objetivo fijado.

## 3. Experiencia de usuario

### 3.1 Disposición y relevancia
- **«Ingeniería»** se activa sola (si Auto-companion está encendido) al atracar en el taller de un ingeniero, al recibir `EngineerCraft`/`EngineerProgress` o al abrir el flujo de ingenierizar. Prioridad: manual > combate > minería > ingeniería > misiones > comercio > exploración.

### 3.2 Inventario de materiales
- Materias primas, fabricados y codificados por grado, con cantidad frente al máximo del grado (300/250/200/150/100) y aviso «al máximo: lo que recojas se pierde».
- Pestaña **taquilla** (Odyssey): artículos, componentes, consumibles y datos desde `ShipLocker`/`BackpackChange`.
- Cambios de la sesión: recogidos, usados, intercambiados, con su origen (misión, escaneo, minería, intercambio, síntesis, ingeniería).

### 3.3 Ingenieros
- Lista con estado (desbloqueado / invitado / conocido), rango y progreso de `EngineerProgress`, sistema y distancia desde tu posición, y qué blueprints aplica y hasta qué grado.
- Siguiente paso de desbloqueo del ingeniero elegido (requisito, invitación, donación) con lo que tienes.
- Sección de ingenieros de a pie (Odyssey) con su estado.

### 3.4 Mi nave modificada e **ingenierizar un módulo** (petición del propietario)
1. Lista de módulos de la nave actual (`Loadout`) con blueprint, grado, calidad y experimental actuales, o «de serie».
2. Al elegir un módulo: blueprints válidos para ese tipo de módulo, efecto experimental opcional y grado objetivo (G5 por defecto).
3. Cálculo desde el grado actual: de serie → desde G1; con el mismo blueprint en G3 → desde G4; con otro blueprint → desde G1 (aviso de que se sustituye).
4. **Materiales totales** por grado y sumados, con **tiradas por grado ajustables** (por defecto: G1–G4 = 1, G5 = 4; se recuerdan en ajustes) y el experimental una vez. Por material: necesarios, tienes, estado (OK / cubierto con intercambio concreto / falta N).
5. Porcentaje listo y resumen.
6. **Dónde conseguir lo que falta**: comerciante de materiales más cercano del tipo necesario (materias primas, fabricados, codificados), con distancia y plataforma, y el intercambio concreto; pistas de obtención del material; broker de tecnología si aplica.
7. **Ingeniero** más cercano que aplica ese blueprint hasta el grado objetivo, con su estado; si no está desbloqueado, qué falta.
8. Botón «Añadir a la lista de la compra».

### 3.5 Lista de la compra
- Objetivos fijados (módulo + blueprint + experimental + grado + tiradas), guardados entre sesiones. Necesidades agregadas, porcentaje por objetivo, lo que falta y lo que sobra.
- Aviso «ya puedes completar X» con el ingeniero cercano que lo hace.
- Los materiales reservados por un objetivo no se ofrecen como excedente a otro.

### 3.6 Planificador de intercambios
- Para lo que falta, propone intercambios desde excedentes con las reglas del comerciante: misma categoría subir 1 grado 6:1, bajar 1 grado 1:3; entre categorías con la penalización del juego; muestra das/recibes y comerciante cercano.

### 3.7 Dónde conseguirlo
- Pistas de obtención por material (dato propio con procedencia): fuente principal (minería, HGE/señales, escaneo de datos, misiones, sitios concretos).

### 3.8 Avisos
- Material al máximo; objetivo completable; ingeniero desbloqueado o invitación nueva; material escaso de una lista conseguido.

## 4. Arquitectura

- Módulo nuevo **`engineering`** (`crates/tmc-modules/src/engineering/`, `ui/src/modules/engineering/`): inventario, ingenieros, nave, cálculo de ingenierizar, lista de la compra, intercambios, avisos. Registro en `all_modules()`, `MODULE_PREFIXES`, allowlists de diagnóstico y contrato JSON Rust→UI.
- **`tmc-model`** tipa: `Materials`, `MaterialCollected`, `MaterialDiscarded`, `MaterialTrade`, `Synthesis`, `EngineerCraft`, `EngineerProgress`, `EngineerContribution`, `TechnologyBroker`, `ShipLocker`, `BackpackChange`, recompensas de materiales ya tipadas (F3), y el bloque `Engineering` de los módulos de `Loadout` (ya parcialmente usado en F1).
- **Datos estáticos propios** (`crates/tmc-modules/data/`): `materials.json` (clave, categoría, grado, máximo, pistas de obtención), `blueprints.json` (tipo de módulo, blueprint, grados, materiales por grado, experimentales y sus materiales), `engineers.json` (nombre, sistema, coordenadas, blueprints y grado máximo, requisitos de desbloqueo, ship/a pie), `trader_rules.json` (tasas). Todos con procedencia y `verified`; construidos desde hechos del juego tras el spike de licencias.
- **Persistencia**: tabla del módulo para la lista de la compra y las tiradas por defecto en ajustes (`config.toml`).
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

## 8. Criterios de aceptación de F4
1. Checks de Rust y UI en verde (incluidos e2e y clippy release).
2. Inventario: con un fixture, cantidades por categoría/grado iguales al evento `Materials` más los cambios posteriores; aviso al máximo.
3. Ingenieros: estado, rango y progreso desde `EngineerProgress` del fixture; distancia desde la posición.
4. Ingenierizar: para «propulsores de serie → Motor sucio G5 + Drag Drives» con tiradas por defecto, la tabla de materiales coincide con el cálculo manual desde `blueprints.json`; con el módulo ya en G3 del mismo blueprint, cuenta desde G4.
5. Intercambios: propuestas con las tasas correctas y sin tocar materiales reservados por otro objetivo.
6. Comerciante más cercano con un Ardent falso; «sin datos» sin servicios.
7. Lista de la compra persiste entre reinicios y avisa cuando un objetivo es completable.
8. Datos estáticos con procedencia y `verified`; nada GPL/AGPL incorporado; `THIRD_PARTY_NOTICES.md` al día.
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
3. Cambiar de blueprint en un módulo ya modificado cuenta desde G1 con aviso.
4. **Materiales necesarios:** cada tirada consume la receta de su grado una vez; el experimental, una vez. Si el módulo está a medias en su grado actual, solo se cuentan las tiradas que faltan para el objetivo (no se modela la calidad exacta).
5. **Intercambios:** solo dentro del tipo de comerciante (materias primas, fabricados, codificados). Misma familia: subir 1 grado 6:1, bajar 1 grado 1:3; otra familia del mismo tipo con la penalización del juego (tasas exactas en `trader_rules.json`, confirmadas en el spike). Primero los de la misma familia; se elige la propuesta que gasta menos excedente.
6. **Reservas** de la lista de la compra por orden de los objetivos fijados (reordenable); lo reservado por un objetivo anterior no es excedente para los siguientes.
7. **Inventario:** manda la última instantánea `Materials` (y `ShipLocker` para la taquilla) más los cambios posteriores; la siguiente instantánea resincroniza.
8. **Persistencia:** migración propia del módulo `engineering-001` para la lista de la compra; tiradas por defecto en `config.toml`.
9. **Comerciantes cercanos:** Ardent primero; si no ofrece comerciantes de materiales o broker, búsqueda de estaciones de Spansh (spike); sin servicios, «sin datos» y desviación documentada.

## 11. Ramas y entregas
- Rama `fase/f4-ingenieros-materiales` desde `main`.
- Bloques sugeridos: 0) spikes de datos/licencias, diario y Ardent; 1) eventos tipados y datos estáticos; 2) inventario, ingenieros y nave; 3) ingenierizar, lista de la compra, intercambios y dónde conseguir; 4) UI; 5) disposición, ajustes, fixtures, e2e, eval y cierre.
