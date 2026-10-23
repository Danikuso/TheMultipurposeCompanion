# TMC · Plan de ejecución de F5 — Fleet Carrier, Colonización, Powerplay 2.0 y BGS

- **Fecha:** 2026-10-23
- **Spec de referencia:** [`docs/specs/2026-10-23-tmc-f5-carrier-colonizacion-powerplay-design.md`](../specs/2026-10-23-tmc-f5-carrier-colonizacion-powerplay-design.md)
- **Rama de trabajo:** `fase/f5-carrier-colonizacion-powerplay`
- **Estado:** 🔄 en propuesta / pendiente de aprobación del propietario

---

## Bloques de trabajo y tareas

### Bloque 0: Eventos y modelos de datos (`tmc-model`)

#### Tarea 1: Tipado de eventos de Fleet Carrier
- **Archivos:** `crates/tmc-model/src/events.rs`, `crates/tmc-model/src/carrier.rs`
- **Cambios:**
  - Definir eventos del diario: `CarrierStats`, `CarrierJumpRequest`, `CarrierJump`, `CarrierDepositFuel`, `CarrierFinance`, `CarrierDecommission`.
  - Estructuras: ID/callsign, capacidad de bodega (mercado, equipamiento, libre), combustible (depósito 1.000 t + bodega de tritio), finanzas (saldo, reservado, coste semanal, servicios activos).
- **Verificación:** `cargo test -p tmc-model`
- **Commit:** `feat(model): eventos y tipos de diario para Fleet Carrier`

#### Tarea 2: Tipado de eventos de Colonización y Powerplay 2.0
- **Archivos:** `crates/tmc-model/src/events.rs`, `crates/tmc-model/src/colonisation.rs`, `crates/tmc-model/src/powerplay.rs`
- **Cambios:**
  - Colonización: `ColonisationConstructionDepot` (`ResourcesRequired[]`, `ConstructionProgress`), `ColonisationContribution`.
  - Powerplay 2.0: `Powerplay` (potencia, rango, méritos acumulados, tiempo), `PowerplayMerits`, `PowerplayVote`.
- **Verificación:** `cargo test -p tmc-model`
- **Commit:** `feat(model): eventos de diario para Colonización y Powerplay 2.0`

---

### Bloque 1: Cliente de red y sincronización (`tmc-net`)

#### Tarea 3: Cliente para endpoint de tick de BGS (`tick.edcd.io`)
- **Archivos:** `crates/tmc-net/src/bgs_tick.rs`, `crates/tmc-net/src/lib.rs`
- **Cambios:**
  - Petición HTTP asíncrona GET a `https://tick.edcd.io/api/ticks`.
  - Caché de 30 minutos respetando política offline/opt-out.
  - Fixture sintético para pruebas offline.
- **Verificación:** `cargo test -p tmc-net`
- **Commit:** `feat(net): cliente para consulta de tick galáctico del BGS`

---

### Bloque 2: Lógica de módulos en el núcleo (`tmc-modules`)

#### Tarea 4: Módulo `carrier` en Rust
- **Archivos:** `crates/tmc-modules/src/carrier/mod.rs`, `crates/tmc-modules/src/carrier/tests.rs`
- **Cambios:**
  - Topic `carrier.overview`: ubicación, combustible (depósito + bodega), saltos posibles de 500 al.
  - Topic `carrier.jump`: cuenta atrás activa de salto (15 min), sistema destino, estados de bloqueo y enfriamiento.
  - Topic `carrier.finances`: coste semanal, semanas de mantenimiento cubiertas, semáforo (seguro/aviso/crítico) y fecha del próximo cobro (jueves 07:00 UTC).
- **Verificación:** `cargo test -p tmc-modules carrier`
- **Commit:** `feat(modules): lógica del módulo carrier con telemetría, cuenta atrás y finanzas`

#### Tarea 5: Módulo `colonisation` en Rust
- **Archivos:** `crates/tmc-modules/src/colonisation/mod.rs`, `crates/tmc-modules/src/colonisation/tests.rs`
- **Cambios:**
  - Topic `colonisation.depot`: seguimiento del proyecto de construcción en curso, porcentaje de obra y lista de recursos requeridos.
  - Cálculo de viajes restantes: `ceil(faltante / capacidad_carga_actual)`.
  - Sourcing de compras: enlace con `PriceBook` para sugerir compras cercanas de los materiales que faltan.
- **Verificación:** `cargo test -p tmc-modules colonisation`
- **Commit:** `feat(modules): módulo de colonización con progreso de obra y cálculo de viajes`

#### Tarea 6: Módulo `powerplay` en Rust
- **Archivos:** `crates/tmc-modules/src/powerplay/mod.rs`, `crates/tmc-modules/src/powerplay/tests.rs`
- **Cambios:**
  - Topic `powerplay.status`: potencia jurada, rango, méritos acumulados, progreso al siguiente rango según la fórmula oficial de Powerplay 2.0.
  - Topic `powerplay.session`: méritos obtenidos en la sesión actual y tasa méritos/hora.
  - Temporizador semanal: cálculo del tiempo restante hasta el tick de los jueves 07:00 UTC.
- **Verificación:** `cargo test -p tmc-modules powerplay`
- **Commit:** `feat(modules): módulo de powerplay 2.0 con seguimiento de méritos y rangos`

#### Tarea 7: Módulo `bgs` en Rust
- **Archivos:** `crates/tmc-modules/src/bgs/mod.rs`, `crates/tmc-modules/src/bgs/tests.rs`
- **Cambios:**
  - Topic `bgs.system`: facciones presentes en el sistema estelar, influencias relativas, facción controladora y estados de conflicto/economía.
  - Topic `bgs.tick`: fecha/hora del último tick detectado del BGS y tiempo transcurrido.
- **Verificación:** `cargo test -p tmc-modules bgs`
- **Commit:** `feat(modules): módulo BGS con facciones locales y sincronización de tick diario`

---

### Bloque 3: Traducciones e i18n

#### Tarea 8: Cadenas y localización en los 5 idiomas
- **Archivos:** `locales/{es,en,fr,de,pt}/carrier.ftl`, `colonisation.ftl`, `powerplay.ftl`, `bgs.ftl`
- **Cambios:**
  - Todas las cadenas requeridas para los nuevos paneles en español (referencia), inglés, francés, alemán y portugués.
- **Verificación:** `cargo test -p tmc-i18n`
- **Commit:** `feat(i18n): cadenas en 5 idiomas para carrier, colonización, powerplay y bgs`

---

### Bloque 4: Interfaz de usuario Glass HUD (`ui/src/modules/`)

#### Tarea 9: Componentes y paneles de Fleet Carrier
- **Archivos:** `ui/src/modules/carrier/CarrierOverviewPanel.svelte`, `ui/src/modules/carrier/CarrierJumpPanel.svelte`, `ui/src/modules/carrier/CarrierFinancesPanel.svelte`, `ui/src/modules/carrier/index.ts`
- **Cambios:**
  - Paneles según el sistema de diseño «TMC Glass HUD».
  - Medidor visual dual de tritio, cuenta atrás de 15 min de salto con botón de copiar sistema destino y tarjeta de semáforo de mantenimiento.
- **Verificación:** `npm --prefix ui test`
- **Commit:** `feat(ui): paneles glass hud de fleet carrier con cuenta atrás y salud financiera`

#### Tarea 10: Paneles de Colonización
- **Archivos:** `ui/src/modules/colonisation/ColonisationProgressPanel.svelte`, `ui/src/modules/colonisation/ColonisationSupplyPanel.svelte`, `ui/src/modules/colonisation/index.ts`
- **Cambios:**
  - Barra de progreso total y barras individuales de recursos faltantes.
  - Indicador de viajes restantes con la nave actual y tarjetas de proveedores cercanos.
- **Verificación:** `npm --prefix ui test`
- **Commit:** `feat(ui): paneles de colonización con progreso de obra y cálculo de viajes`

#### Tarea 11: Paneles de Powerplay 2.0 y BGS
- **Archivos:** `ui/src/modules/powerplay/PowerplayPanel.svelte`, `ui/src/modules/bgs/BgsPanel.svelte`, `ui/src/modules/powerplay/index.ts`, `ui/src/modules/bgs/index.ts`
- **Cambios:**
  - Stepper de rango de potencia y velocímetro de méritos/hora en Powerplay.
  - Barras de influencia de facciones del sistema y estado del tick diario en BGS.
- **Verificación:** `npm --prefix ui test`
- **Commit:** `feat(ui): paneles de powerplay 2.0 y estado del bgs local`

#### Tarea 12: Integración de disposiciones en el Shell y Auto-companion
- **Archivos:** `ui/src/shell/Sidebar.svelte`, `ui/src/lib/layouts.ts`, `crates/tmc-app/src/config.rs`
- **Cambios:**
  - Registrar las disposiciones «Carrier», «Colonización» y «Powerplay».
  - Reglas de Auto-companion: activación automática de Carrier a bordo del carrier o en cuenta atrás de salto; activación automática de Colonización al atracar en depósitos de obra.
- **Verificación:** `npm --prefix ui test`, `npm --prefix ui run check`
- **Commit:** `feat(ui): disposiciones carrier, colonización y powerplay en el shell y auto-companion`

---

### Bloque 5: Verificación Integral, Cierre y Compilación

#### Tarea 13: Verificación de calidad, footprint y build
- **Acciones:**
  - `npm --prefix ui test` (100% Vitest pasando).
  - `npm --prefix ui run check` (0 errores Svelte/TypeScript).
  - `cargo test --workspace` (tests Rust íntegros).
  - `npm --prefix ui run build` (UI compilada).
  - `cargo build -p tmc-desktop` (ejecutable actualizado con assets).
  - Actualización de `status.md`.
- **Commit:** `chore: verificación y cierre de F5 (Carrier, Colonización, Powerplay y BGS)`
