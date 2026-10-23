# Plan de ejecución: Rediseño visual global de TMC (Design System «TMC Glass HUD»)

**Fecha:** 2026-10-22  
**Spec de referencia:** [`docs/specs/2026-10-22-tmc-ui-rediseno-global-design.md`](../specs/2026-10-22-tmc-ui-rediseno-global-design.md)  
**Rama de trabajo:** `fase/f4-ingenieros-materiales` (o rama dedicada tras autorización)  
**Estado:** Preparado para revisión y aprobación del propietario

---

## Bloques de trabajo y tareas

### Bloque 1: Fundamentos y Shell Global

#### Tarea 1: Tokens CSS compartidos y clases del Design System
- **Archivos:** `ui/src/styles/tokens.css`, `ui/src/styles/base.css`
- **Cambios:**
  - Añadir tokens semánticos: `--hud-cyan` (`#06b6d4`), `--hud-purple` (`#a855f7`), `--hud-surface-card` (`#161c2d`).
  - Definir clases utilitarias compartidas: `.hud-meter-track`, `.hud-meter-fill`, `.badge-*`, `.stepper`, `.step-item`, `.hud-btn`, `.metric-card`.
- **Verificación:** `npm --prefix ui run check`
- **Commit:** `feat(ui): tokens y componentes comunes del sistema de diseño TMC Glass HUD`

#### Tarea 2: Rediseño visual de `TopBar.svelte` y `Sidebar.svelte`
- **Archivos:** `ui/src/shell/TopBar.svelte`, `ui/src/shell/Sidebar.svelte`, `ui/src/shell/Panel.svelte`
- **Cambios:**
  - `TopBar`: Tarjetas HUD de cabina, micro-barras para combustible y casco, interruptor visual táctil para Auto-companion, saldo de créditos en color ámbar.
  - `Sidebar`: Iconografía unificada, separación de Layouts vs Vistas directas, indicadores de relevancia luminosos.
  - Preservar `data-testid="topbar-system"`, `data-testid="auto-companion"`, etc.
- **Verificación:** `npm --prefix ui test`, `npm --prefix ui run check`
- **Commit:** `feat(ui): rediseño visual de topbar y barra lateral de navegación`

---

### Bloque 2: Navegación, Exploración y Exobiología

#### Tarea 3: Rediseño de `RouteStrip.svelte` y `SystemCard.svelte`
- **Archivos:** `ui/src/modules/navigation/RouteStrip.svelte`, `ui/src/modules/navigation/SystemCard.svelte`
- **Cambios:**
  - Tira de cabina con próximo salto en grande, badge verde scoopable, combustible estimado y botón de copiar sistema al portapapeles.
  - Preservar `data-testid="route-strip"`, etc.
- **Verificación:** `npm --prefix ui test`
- **Commit:** `feat(ui): ficha de ruta estilo cabina y acciones directas en navegación`

#### Tarea 4: Rediseño de `SystemBodiesPanel.svelte`
- **Archivos:** `ui/src/modules/exploration/SystemBodiesPanel.svelte`
- **Cambios:**
  - Tarjetas superiores de valor: «Valor Datos Actual» y «Valor con Mapeo Recomendado».
  - Barra de progreso FSS estilizada.
  - Filas de cuerpos estilo tarjeta compacta con badges semánticos (`MAPEAR ★`, `6 SONDAS`, `BIO`, `GEO`).
  - Preservar `data-testid="system-name"`, `data-testid="system-stars"`, `data-testid="body-row"`, `data-testid="body-marks"`, `data-testid="mapped-value"`.
- **Verificación:** `npm --prefix ui test`
- **Commit:** `feat(ui): tarjetas visuales de cuerpos celestes y semáforos de sondas en exploración`

#### Tarea 5: Rediseño de `ExobiologyPanel.svelte`
- **Archivos:** `ui/src/modules/exobiology/ExobiologyPanel.svelte`
- **Cambios:**
  - Tarjeta destacada de especie activa con valor de muestra y bonificación por primera muestra.
  - Medidor visual de distancia genética (`520 m / 500 m ✔`).
  - Stepper horizontal de muestras (1 a 3).
  - Preservar `data-testid="exobiology-species"`, `data-testid="exobiology-sample"`, etc.
- **Verificación:** `npm --prefix ui test`
- **Commit:** `feat(ui): stepper de muestras y medidor genético en exobiología`

---

### Bloque 3: Combate y Minería

#### Tarea 6: Rediseño de `TargetPanel.svelte`, `ShipPanel.svelte` y `AxPanel.svelte`
- **Archivos:** `ui/src/modules/combat/TargetPanel.svelte`, `ui/src/modules/combat/ShipPanel.svelte`, `ui/src/modules/combat/AxPanel.svelte`
- **Cambios:**
  - `TargetPanel`: Barras duales grandes para escudo y casco, badge de recompensa/se busca, y tarjetas de 3 subsistemas críticos.
  - `AxPanel`: Tarjeta de Interceptor Thargoide, indicador visual de 4 corazones activos/destruidos y alerta de daño cáustico.
  - Preservar `data-testid="target-name"`, `data-testid="target-bounty"`, etc.
- **Verificación:** `npm --prefix ui test`
- **Commit:** `feat(ui): barras duales de combate, estado de subsistemas y tarjetas AX`

#### Tarea 7: Rediseño de `ProspectorPanel.svelte`, `MiningSessionPanel.svelte` y `SellPanel.svelte`
- **Archivos:** `ui/src/modules/mining/ProspectorPanel.svelte`, `ui/src/modules/mining/MiningSessionPanel.svelte`, `ui/src/modules/mining/SellPanel.svelte`
- **Cambios:**
  - `ProspectorPanel`: Porcentaje restante del asteroide en número grande, barras de composición mineral ordenadas por valor económico.
  - `SellPanel`: Tarjetas de mejores estaciones compradoras cercanas con botón directo de copia de ruta.
  - Preservar `data-testid="prospector-composition"`, etc.
- **Verificación:** `npm --prefix ui test`
- **Commit:** `feat(ui): barras de composición de prospector y tarjetas de venta en minería`

---

### Bloque 4: Comercio, Misiones y Pasajeros

#### Tarea 8: Rediseño de `MarketPanel.svelte`, `CargoPanel.svelte` y `TradeRoutePanel.svelte`
- **Archivos:** `ui/src/modules/trade/MarketPanel.svelte`, `ui/src/modules/trade/CargoPanel.svelte`, `ui/src/modules/trade/RoutePanel.svelte`
- **Cambios:**
  - `MarketPanel`: Buscador de mercancías integrado, badges destacados de beneficio por tonelada (`+18.450 Cr/t`).
  - `CargoPanel`: Barra de capacidad de bodega y beneficio estimado de descarga.
  - `TradeRoutePanel`: Stepper visual de bucle comercial (Paso A ➔ Paso B).
  - Preservar `data-testid="market-station"`, `data-testid="market-commodity"`, etc.
- **Verificación:** `npm --prefix ui test`
- **Commit:** `feat(ui): mercado visual con márgenes destacados y stepper de ruta de comercio`

#### Tarea 9: Rediseño de `MissionsPanel.svelte`, `MassacrePanel.svelte`, `PassengersPanel.svelte`
- **Archivos:** `ui/src/modules/missions/MissionsPanel.svelte`, `ui/src/modules/missions/MassacrePanel.svelte`, `ui/src/modules/missions/PassengersPanel.svelte`
- **Cambios:**
  - `MissionsPanel`: Grupos por destino con temporizadores semafóricos de expiración (`>2h`, `<1h`, `<15m`) y botón «Añadir a ruta».
  - `MassacrePanel`: Barras de progreso de bajas por facción contratante para optimizar el apilamiento de masacre pirata.
  - Preservar `data-testid="missions-group"`, `data-testid="mission-to-route"`, etc.
- **Verificación:** `npm --prefix ui test`
- **Commit:** `feat(ui): tarjetas de misión por destino, semáforo de tiempo y optimizador de masacre`

---

### Bloque 5: Verificación Integral y Compilación

#### Tarea 10: Verificación de calidad, footprint y build
- **Acciones:**
  - `npm --prefix ui test` (378+ tests Vitest pasando).
  - `npm --prefix ui run check` (0 errores TypeScript/Svelte).
  - `cargo test --workspace` (tests Rust íntegros).
  - `npm --prefix ui run build` (UI compilada).
  - `cargo build -p tmc-desktop` (ejecutable actualizado).
  - Actualización de `status.md`.
- **Commit:** `chore: verificación y cierre del rediseño visual global de TMC`
