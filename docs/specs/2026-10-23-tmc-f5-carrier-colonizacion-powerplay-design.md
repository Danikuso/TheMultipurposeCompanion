# TMC · Especificación de F5 — Fleet Carrier, Colonización, Powerplay 2.0 y BGS

- **Estado:** 🔄 en propuesta / pendiente de aprobación del propietario
- **Fecha:** 2026-10-23
- **Depende de:**
  - [`2026-10-17-tmc-arquitectura-y-f0-design.md`](2026-10-17-tmc-arquitectura-y-f0-design.md) (Arquitectura y cimientos).
  - F1 ([`2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md`](2026-10-17-tmc-f1-navegacion-exploracion-exobiologia-design.md): Spansh, alcance de salto, portapapeles).
  - F2 ([`2026-10-18-tmc-f2-combate-mineria-design.md`](2026-10-18-tmc-f2-combate-mineria-design.md): `PriceBook`, Ardent, `core.clock`).
  - F3 ([`2026-10-19-tmc-f3-comercio-misiones-design.md`](2026-10-19-tmc-f3-comercio-misiones-design.md): EDDN, búsqueda de mercancías, bodega y misiones).
  - F4 ([`2026-10-21-tmc-f4-ingenieros-materiales-design.md`](2026-10-21-tmc-f4-ingenieros-materiales-design.md): taquilla `ShipLocker`, inventario y catálogo).
  - Guía normativa de diseño: [`docs/design-system-glass-hud.md`](../design-system-glass-hud.md) y filosofía Companion [`AGENTS.md` §0](../../AGENTS.md).

---

## 1. Intención y filosofía de Companion

TMC tiene como meta ser el **ayudante y copiloto indispensable** tanto para comandantes que gestionan su propio megacrucero estelar como para aquellos que participan en la expansión de la burbuja humana o luchan por su potencia:

1. **Gestión sin calculadora del Fleet Carrier:** El jugador no debe hacer cálculos mentales de mantenimiento ni arriesgarse a perder su portaaviones por impago. TMC debe alertar visualmente de las semanas de mantenimiento cubiertas, calcular saltos posibles con el tritio actual y guiar la cuenta atrás de salto con precisión.
2. **Impulso a la Colonización de Sistemas (Trailblazers):** Si el comandante participa en la construcción de una nueva base o estación (`ColonisationConstructionDepot`), TMC calcula automáticamente los viajes restantes según la bodega de su nave actual, indica dónde comprar los materiales que faltan más cerca y al mejor precio, y permite copiar el sistema de compra con un solo clic.
3. **Powerplay 2.0 comprensible y motivador (Ascendancy):** Traduce el sistema de méritos de Powerplay 2.0 en barras de progreso claras hacia el siguiente rango, calculando los méritos/hora en la sesión y mostrando el tiempo hasta el próximo tick semanal.
4. **Pulso galáctico con BGS en tiempo real:** Muestra las facciones del sistema actual, su influencia y estados de guerra/elección, informando además de cuándo ocurrió el último tick del BGS del día mediante la API ligera de `tick.edcd.io`.
5. **Apoyo táctico a pie (Odyssey):** Monitoriza la telemetría vital del traje (oxígeno, salud, temperatura y gravedad) e inventario de consumibles clave (células de energía, botiquines y ganzúas e-breaches).

---

## 2. Objetivos, no objetivos y antiobjetivos

### Objetivos
- Módulo **`carrier`**:
  - Estado del portaaviones: ubicación, combustible en depósito + bodega, capacidad ocupada/libre.
  - Salto en curso: cuenta atrás de 15 min, sistema destino, estado de bloqueo de salto (lockout de 5 min) y tiempo de enfriamiento (cooldown de 5 min).
  - Salud financiera: coste semanal (base 5M Cr + servicios activos + saltos a 100k Cr), saldo bancario del carrier, balance reservado vs disponible, semanas de mantenimiento cubiertas con semáforo y fecha del próximo cobro (jueves 07:00 UTC).
  - Asistente de ruta para carrier (saltos de hasta 500 al) con consumo estimado de tritio y botón para copiar sistema destino.
- Módulo **`colonisation`**:
  - Detección automática al atracar en un depósito de construcción espacial o planetario (`SpaceConstructionDepot`, `PlanetaryConstructionDepot`).
  - Progreso global y por recurso requerido (`ResourcesRequired[]`).
  - Cálculo de viajes pendientes (`ceil(faltante / capacidad_carga)`).
  - Sourcing integrado: mejores estaciones cercanas para adquirir los recursos que faltan.
- Módulo **`powerplay`**:
  - Potencia jurada, rango actual (R1 a R100+), méritos acumulados y méritos de la sesión con tasa méritos/hora.
  - Progreso al siguiente rango con la fórmula de méritos oficial de Powerplay 2.0.
  - Temporizador semanal hasta el tick de los jueves 07:00 UTC.
- Módulo **`bgs`**:
  - Facciones del sistema con porcentajes de influencia, facción controladora y estados de conflicto/economía.
  - Consulta pasiva y ligera de `https://tick.edcd.io/api/ticks` para saber la antigüedad del último tick diario del BGS.
- Soporte a pie (**`onfoot`**):
  - Telemetría básica de traje en `ShipPanel` / subpanel cuando `Flags2.OnFoot` está activo.

### No objetivos
- Gestión remota de órdenes de mercado del carrier mediante CAPI (F6).
- Módulos de voz (F7) u overlay transparente (F8).
- Cambios de legalidad y criminalidad de *Law & Disorder* (F9).

### Antiobjetivos
- Consumo excesivo: el límite estricto de < 150 MB RAM y < 1% CPU se mantiene firmemente.
- Dependencia de APIs frágiles: si `tick.edcd.io` está caído, TMC funciona 100% con los datos del diario local degradándose con elegancia.
- Spamear al usuario con alertas irrelevantes: solo avisos de mantenimiento bajo (< 2 semanas) o cuenta atrás de salto crítica (< 3 min).

---

## 3. Experiencia de usuario (TMC Glass HUD)

### 3.1 Disposiciones y auto-companion
- Se añade la disposición **«Carrier»**: se activa automáticamente al estar a bordo del propio portaaviones o cuando hay un salto de carrier en cuenta atrás.
- Se añade la disposición **«Colonización»**: se activa automáticamente al entrar en el espacio o atracar en un depósito de construcción de colonización.
- Se añade la disposición **«Powerplay»**: accesible desde la barra lateral y priorizada cuando el comandante realiza actividades de Powerplay en la sesión.
- Disposición **«A pie»**: los paneles adaptan su vista si el jugador está a pie (`Flags2.OnFoot = true`).

### 3.2 Paneles y componentes Glass HUD

#### Módulo Carrier:
1. `CarrierOverviewPanel.svelte`:
   - Cabecera: Nombre del carrier, callsign (ej. `V3L-72B`), sistema actual y coordenadas.
   - Medidor de tritio: Barra dual con depósito principal (capacidad 1.000 t) y tritio en bodega de carga. Indicador de saltos disponibles de 500 al.
   - Capacidad del carrier: Barra segmentada de almacenamiento (carga de mercado, equipamiento, libre).
2. `CarrierJumpPanel.svelte`:
   - Cuenta atrás visual de salto: reloj digital grande monospace, estado de preparación (`En espera` → `Bloqueo de salto` [últimos 5 min] → `Saltando` → `Enfriamiento` [5 min post-salto]).
   - Sistema destino con distancia y botón directo de copiar al portapapeles.
3. `CarrierFinancesPanel.svelte`:
   - Tarjeta métrica de mantenimiento semanal: coste base + servicios + saltos.
   - Semáforo de cobertura:
     - 🟢 `> 4 semanas`: Seguro.
     - 🟡 `2 – 4 semanas`: Advertencia.
     - 🔴 `< 2 semanas`: Crítico (alerta de riesgo de embargo/desguace).
   - Saldo bancario del carrier y botón para transferir fondos. Próximo tick: jueves 07:00 UTC.

#### Módulo Colonización:
1. `ColonisationProgressPanel.svelte`:
   - Nombre de la colonia / estación y porcentaje total de construcción completado (`ConstructionProgress`).
   - Lista de recursos con barras de llenado individuales: cantidad entregada / total requerida.
   - Cálculo automático de «Viajes restantes» con la nave actual.
2. `ColonisationSupplyPanel.svelte`:
   - Tarjetas de proveedores cercanos para los materiales incompletos con distancia en al, precio y botón de copia.

#### Módulo Powerplay:
1. `PowerplayPanel.svelte`:
   - Emblema / nombre de la potencia jurada y tiempo de lealtad.
   - Stepper de rango actual y barra de progreso hacia el siguiente rango de méritos.
   - Métricas de sesión: méritos ganados hoy, tasa méritos/h y tiempo restante para el tick semanal.

#### Módulo BGS:
1. `BgsPanel.svelte`:
   - Facciones del sistema: barras de influencia ordenadas, insignias de estado (Guerra, Boom, Expansión).
   - Indicador de tick galáctico del BGS: hora del último tick registrado y horas transcurridas.

---

## 4. Arquitectura de datos y eventos

### 4.1 Eventos del Diario procesados
- **Carrier:**
  - `CarrierStats`: ID, nombre, callsign, sistema, combustible, bodega ocupada, servicios activos, saldo bancario.
  - `CarrierJumpRequest`: sistema destino, timestamp de salida.
  - `CarrierJump`: salto completado, sistema alcanzado, consumo de tritio.
  - `CarrierDepositFuel`: recarga de tritio.
  - `CarrierFinance`: costes de mantenimiento, balance reservado/disponible.
  - `CarrierDecommission`: alerta de cancelación o desguace.
- **Colonización:**
  - `ColonisationConstructionDepot`: lista `ResourcesRequired`, progreso porcentual, tipo de instalación.
  - `ColonisationContribution`: recursos entregados en la sesión.
- **Powerplay 2.0:**
  - `Powerplay`: potencia, rango, méritos, tiempo de servicio.
  - `PowerplayMerits`: méritos obtenidos y origen.
  - `PowerplayVote`: votos de consolidación / preparación.
- **BGS:**
  - `FSDJump` / `Location`: lista `Factions[]` con influencia y estados.
- **Odyssey / A pie:**
  - `Status.json`: `Flags2` (OnFoot, etc.), `Oxygen`, `Health`, `Gravity`, `Temperature`.

### 4.2 Servicio de red para Tick BGS (`tmc-net`)
- Cliente HTTP asíncrono para `https://tick.edcd.io/api/ticks` con caché de 30 minutos y opt-out respetado.

---

## 5. Criterios de aceptación (F5)

1. **`carrier` topic y panel:** `CarrierStats` y `CarrierFinance` generan telemetría precisa; el semáforo de mantenimiento clasifica correctamente las semanas cubiertas con < 2 semanas en alerta crítica.
2. **Cuenta atrás de salto de Carrier:** `CarrierJumpRequest` inicia una cuenta atrás de salto sincronizada con el reloj del diario, mostrando los estados de bloqueo y enfriamiento.
3. **Colonización en tiempo real:** `ColonisationConstructionDepot` calcula con precisión las unidades faltantes y el número de viajes requeridos según la capacidad de carga de la nave actual.
4. **Powerplay 2.0:** Los méritos y rangos se calculan de acuerdo con las fórmulas oficiales de Ascendancy (`r >= 6: 15000 + 8000*(r-5)`).
5. **Tick de BGS:** `tick.edcd.io` reporta el último tick en < 1s o se degrada sin error si la red está apagada o sin conexión.
6. **Integración con diseño Glass HUD:** Todos los nuevos paneles utilizan estrictamente las clases y tokens de `docs/design-system-glass-hud.md`.
7. **i18n completa:** Todas las cadenas nuevas existen en `es`, `en`, `fr`, `de`, `pt` con tests en verde.
8. **Consumo y rendimiento:** La suite completa de tests de Rust y Vitest pasa al 100%, con memoria en reposo < 150 MB y CPU < 1%.
