# Plan de ejecución: Tarjetas Compactas, Desplegables y Acordeones por Defecto («TMC Glass HUD»)

**Fecha:** 2026-10-24  
**Spec de referencia:** [`docs/specs/2026-10-24-tmc-ui-tarjetas-compactas-desplegables-design.md`](../specs/2026-10-24-tmc-ui-tarjetas-compactas-desplegables-design.md)  
**Rama de trabajo:** `main`  
**Estado:** Preparado para revisión y aprobación del propietario  

---

## Resumen de la iniciativa

Extender el principio de diseño de cabina **«Compacto por defecto»** a todos los paneles y tarjetas de TMC:
1. Eliminar módulos y elementos no accionables o irrelevantes (ej. módulos que no admiten ingeniería en `ShipPanel.svelte`).
2. Sustituir listas largas y despliegues masivos por un selector desplegable (`<select class="glass-select">`) asociado a una tarjeta activa condensada.
3. Incorporar un acordeón de lista completa (`[ ▼ Ver lista ({total}) / ▲ Ocultar lista ]`) que inicia siempre en estado colapsado (`expanded = false`).
4. Preservar estrictamente todos los atributos `data-testid` y contratos de websocket/REST existentes.

---

## Bloques de trabajo y tareas

### Bloque 1: Módulo de Ingeniería y Cimientos del Patrón

#### Tarea 1: `ShipPanel.svelte` — Filtrado de no ingenierizables y selector desplegable compacto
- **Archivos:** `ui/src/modules/engineering/ShipPanel.svelte`, `ui/tests/engineering.test.ts`
- **Cambios:**
  - Filtrar `engineerableModules = data.modules.filter(row => row.engineerable)`.
  - Añadir selector `<select data-testid="ship-module-select">` con el módulo activo.
  - Diseñar tarjeta activa compacta (`.active-module-card`) con slot, nombre, plano, grado y calidad.
  - Implementar acordeón colapsado por defecto con botón de alternancia `<button data-testid="toggle-module-list">`.
  - Actualizar test 364 en `engineering.test.ts` para verificar que `data-slot="Unsupported"` no se renderiza.
  - Añadir test que verifique el funcionamiento del desplegable y la alternancia de colapsado.
- **Verificación:** `npm --prefix ui test -- engineering.test.ts` (107/107 PASS).
- **Commit:** `feat(engineering): filtrar modulos no ingenierizables y anadir selector desplegable compacto con acordeon`

#### Tarea 2: `MaterialsPanel.svelte` — Selector compacto de categorías y buscador colapsable
- **Archivos:** `ui/src/modules/engineering/MaterialsPanel.svelte`, `ui/tests/engineering.test.ts`
- **Cambios:**
  - Sustituir barra horizontal fija o listas densas por un selector de categoría compacto (`Codificados`, `Manufacturados`, `Materias primas`, `Taquilla`).
  - Resumen métrico superior compacto del inventario (`Almacenados: X / Max`).
  - Lista de grados colapsable por defecto si hay más de 3 grados con existencias vacías.
- **Verificación:** `npm --prefix ui test -- engineering.test.ts`
- **Commit:** `feat(engineering): selector de categorias compacto y acordeon en panel de materiales`

---

### Bloque 2: Cuentas, Flota Remota y Sincronización

#### Tarea 3: `AccountsPanel.svelte` — Flota remota en selector desplegable y colapso de logs
- **Archivos:** `ui/src/modules/accounts/AccountsPanel.svelte`, `ui/tests/accounts.test.ts`
- **Cambios:**
  - Sustituir la cascada de tarjetas de naves de Frontier CAPI por un selector desplegable `<select data-testid="remote-fleet-select">` y una tarjeta activa de la nave seleccionada (modelo, nombre, estación, sistema, coste de transferencia).
  - Añadir acordeón colapsado por defecto para ver la flota completa tabular (`[ ▼ Ver flota completa ({naves}) ]`).
  - Colapsar por defecto los historiales de eventos de EDDN, Inara y EDSM, dejando solo visible el badge de última sincronización exitosa.
- **Verificación:** `npm --prefix ui test -- accounts.test.ts`
- **Commit:** `feat(accounts): selector desplegable para flota remota de capi y acordeon en sincronizacion`

---

### Bloque 3: Misiones, Pasajeros y Combate

#### Tarea 4: `MissionsPanel.svelte`, `MassacrePanel.svelte` y `PassengersPanel.svelte`
- **Archivos:**
  - `ui/src/modules/missions/MissionsPanel.svelte`
  - `ui/src/modules/missions/MassacrePanel.svelte`
  - `ui/src/modules/missions/PassengersPanel.svelte`
  - `ui/tests/missions.test.ts`
- **Cambios:**
  - Selector desplegable de misiones activas por facción/sistema destino.
  - Tarjeta condensada de la misión prioritaria con tiempo restante y botón de copiar sistema objetivo.
  - Acordeón colapsado para misiones secundarias.
  - `MassacrePanel`: selector de objetivo y tarjeta de recuento óptimo.
  - `PassengersPanel`: selector de grupo de pasajeros y estado de cabina.
- **Verificación:** `npm --prefix ui test -- missions.test.ts`
- **Commit:** `feat(missions): selector desplegable de misiones activas y tarjetas compactas por defecto`

---

### Bloque 4: Comercio, Minería y Navegación

#### Tarea 5: `MarketPanel.svelte`, `CargoPanel.svelte` y `SellPanel.svelte`
- **Archivos:**
  - `ui/src/modules/trade/MarketPanel.svelte`
  - `ui/src/modules/trade/CargoPanel.svelte`
  - `ui/src/modules/mining/SellPanel.svelte`
  - `ui/tests/trade.test.ts`, `ui/tests/mining.test.ts`
- **Cambios:**
  - `MarketPanel`: selector compacto de las 5 mejores mercancías por beneficio/t; catálogo completo colapsado en acordeón.
  - `CargoPanel`: resumen visual de capacidad y selector desplegable de inventario si hay más de 4 tipos de carga.
  - `SellPanel`: tarjeta activa de la mejor estación de venta para la carga actual, colapsando alternativas en acordeón.
- **Verificación:** `npm --prefix ui test -- trade.test.ts mining.test.ts`
- **Commit:** `feat(trade): mercado y venta con selector compacto y catalogo colapsable`

---

### Bloque 5: Fleet Carrier, Colonización, Powerplay 2.0 y BGS

#### Tarea 6: `CarrierOverviewPanel.svelte`, `ColonisationProgressPanel.svelte`, `PowerplayPanel.svelte`, `BgsPanel.svelte`
- **Archivos:**
  - `ui/src/modules/carrier/CarrierOverviewPanel.svelte`
  - `ui/src/modules/carrier/CarrierPacksPanel.svelte`
  - `ui/src/modules/colonisation/ColonisationProgressPanel.svelte`
  - `ui/src/modules/powerplay_bgs/PowerplayPanel.svelte`
  - `ui/src/modules/powerplay_bgs/BgsPanel.svelte`
  - `ui/tests/carrier.test.ts`, `ui/tests/colonisation.test.ts`, `ui/tests/powerplay_bgs.test.ts`
- **Cambios:**
  - `Carrier`: selector desplegable de dependencias y servicios activos; desglose colapsado por defecto.
  - `Colonisation`: tarjeta activa de proyecto y desglose de materiales colapsado en acordeón.
  - `Powerplay` y `BGS`: selector desplegable de potencia o facción en conflicto con tarjeta activa y listas de sistemas/facciones secundarias colapsadas.
- **Verificación:** `npm --prefix ui test -- carrier colonisation powerplay_bgs`
- **Commit:** `feat(carrier,colonisation,powerplay): tarjetas compactas por defecto y listas en acordeon`

---

### Bloque 6: Internacionalización (i18n), Verificación Integral y Build

#### Tarea 7: Claves i18n y verificación global
- **Archivos:** `locales/{es,en,fr,de,pt}/*.ftl`
- **Cambios:**
  - Incorporar claves normalizadas en los 5 idiomas:
    - `ui-compact-show-list = ▼ Ver lista ({$count})`
    - `ui-compact-hide-list = ▲ Ocultar lista`
    - `ui-compact-select-item = Seleccionar ({$count})...`
    - `ui-compact-all-items = Ver lista completa ({$count})`
- **Verificación:**
  - `npm --prefix ui test` (suite completa en verde).
  - `npm --prefix ui run check` (0 errores, 0 advertencias).
  - `cargo test --workspace` (suite Rust intacta).
  - `cargo clippy --workspace --all-targets -- -D warnings`.
  - Build de producción: `npm --prefix ui run build` y `cargo build -p tmc-desktop`.
- **Commit:** `chore(i18n): claves de tarjetas compactas y acordeones en 5 idiomas y verificacion`
