# TMC · Especificación de F9 — Crimen, Notoriedad y Piratería (Law & Disorder)

- **Estado:** 🔄 en propuesta / pendiente de aprobación del propietario
- **Fecha:** 2026-10-24
- **Rama:** `fase/f9-crimen-pirateria`
- **Depende de:**
  - [`2026-10-17-tmc-arquitectura-y-f0-design.md`](2026-10-17-tmc-arquitectura-y-f0-design.md) (Arquitectura base, rendimiento < 150 MB y R12).
  - [`2026-10-18-tmc-f2-combate-mineria-design.md`](2026-10-18-tmc-f2-combate-mineria-design.md) (Combate y recompensas).
  - [`2026-10-24-tmc-f7-voz-copiloto-design.md`](2026-10-24-tmc-f7-voz-copiloto-design.md) (Copiloto auditivo y alertas de cabina).
  - Sistema de diseño normativo: [`docs/design-system-glass-hud.md`](../design-system-glass-hud.md) y filosofía Companion [`AGENTS.md` §0](../../AGENTS.md).

---

## 1. Intención y filosofía de Companion (El Ayudante inquebrantable)

En *Elite Dangerous*, las mecánicas de crimen, contrabando y notoriedad pueden ser abrumadoras o frustrantes si no se conocen al detalle:
- **El bloqueo por notoriedad:** Un piloto que abate accidentalmente a una nave limpia o comete un delito en combate acumula **Notoriedad**. Al intentar pagar sus multas en una estación o en un Interstellar Factor, el servicio se niega a tramitar el cobro sin darle explicaciones claras.
- **La cuenta atrás silenciosa:** Pocos jugadores saben que la notoriedad solo decae **1 punto por cada 2 horas de vuelo real en el espacio o supercrucero**, y que estar atracado en un hangar o desconectado detiene el temporizador.
- **El riesgo policial y las fuerzas de élite (ATR):** Volar con alta notoriedad en sistemas de Alta Seguridad provoca la interdicción y respuesta letal casi inmediata de las fuerzas policiales avanzadas (ATR).
- **Contrabando y mercancía robada:** La carga recogida en el espacio o robada a piratas/convoyes queda marcada como `Stolen` o `Illegal`. Ser escaneado por las patrullas policiales al entrar a una estación acarrea multas masivas e incluso la destrucción inmediata de la nave.

Fase 9 construye en TMC el **módulo integral de Crimen, Notoriedad y Piratería (`crime`)**, diseñado para actuar como un copiloto táctico y salvavidas legal:
1. **Nunca dejar al piloto en un callejón sin salida:**
   - Si el piloto tiene multas o es buscado (`Wanted`), TMC calcula y muestra de forma proactiva la **«Guía de Limpieza de Nombre del Ayudante»**:
     - Si Notoriedad > 0: Alerta visible con **cronómetro estimativo de decaimiento en vuelo** (p. ej. `Notoriedad 2: ~3 h 42 min de vuelo restantes para habilitar Interstellar Factors`) y advertencia de evasión de sistemas de alta seguridad.
     - Si Notoriedad = 0: Localización automática y visualización del **Interstellar Factor más cercano**, indicando distancia en años luz, tipo de puerto/estación y botón de 1 clic para copiar el sistema al portapapeles.
2. **Asistente de Contrabando y Mercados Negros:**
   - Detección reactiva de mercancías robadas o ilegales en la bodega (`Cargo`).
   - Localización del **Mercado Negro (Black Market) más cercano**.
   - Consejos tácticos de aproximación: recordatorio de *Silent Running* (Carrera Silenciosa), dispersores térmicos (Heat Sinks) o aproximación balística rápida (> 200 m/s) para evitar que las patrullas completen el escaneo de entrada.
3. **Termómetro de Seguridad del Sistema (Law & Disorder):**
   - Análisis del nivel de seguridad del sistema actual (Alta, Media, Baja o Anarquía).
   - Indicador de tiempo estimado de respuesta policial y riesgo de interdicción.
   - Detección de sistemas de **Anarquía** cercanos como refugios legales donde las leyes jurisdiccionales no aplican.
4. **Voz táctica de cabina:**
   - Avisos por voz breves y concisos («¡Alerta! Notoriedad incrementada a {nivel}», «Cargamento ilegal en bodega», «Multa emitida por {facción}»).

---

## 2. Objetivos y alcance de Fase 9

### Objetivos

1. **Modelos de datos y eventos en `tmc-model`:**
   - Modelos de delitos: `CrimeType` (`Assault`, `Murder`, `Piracy`, `RecklessWeaponsDischarge`, `IllegalCargo`, etc.), `CrimeEntry` y `FactionBountySummary`.
   - Eventos de diario:
     - `CommitCrime`: registro de delito, tipo, facción emisora, víctima, multa o recompensa.
     - `CrimeVictim`: cuando el propio comandante es víctima de un delito.
     - `PayFines`: pago de multas (con porcentaje de comisión de Interstellar Factor).
     - `PayBounties`: redención de recompensas sobre la nave activa.
   - Topic `crime.status`:
     - `legal_state`: estado legal en vivo (`Clean`, `IllegalCargo`, `Speeding`, `Wanted`, `Hostile`, `Warrant`).
     - `notoriety`: nivel actual (0 a 10).
     - `notoriety_decay_seconds`: segundos estimados de vuelo restantes para llegar a 0.
     - `total_fines_cr` y `total_bounties_cr`: balance económico pendiente.
     - `factions`: desglose por facción y sistema.
     - `stolen_cargo`: desglose de items y toneladas robadas en la bodega.
     - `nearest_interstellar_factor`: estación, sistema, distancia en al, plataformas (L/M/S).
     - `nearest_black_market`: estación, sistema, distancia en al, plataformas.
     - `system_security`: clasificación de seguridad del sistema actual y tiempo de respuesta policial.
2. **Módulo `crime` en `crates/tmc-modules`:**
   - Motor de seguimiento de estado criminal en el actor central:
     - Monitor de tiempo en vuelo: computa el tiempo real en supercrucero / espacio normal (no atracado en hangar) para decrementar la notoriedad a razón de 1 punto cada 2 horas (7.200 s).
     - Acumulador de multas y recompensas por facción.
     - Detección de cargamento robado desde `Cargo`.
     - Buscador de servicios cercanos (Interstellar Factors y Mercados Negros) utilizando la base de datos de estaciones de `tmc-store` y eventos `Docked`.
3. **Copiloto auditivo (`tmc-core` / `tmc-tts`):**
   - Integración de avisos de seguridad en `VoiceAdvisor`:
     - Incremento de notoriedad (`tts-crime-notoriety-gain`).
     - Notoriedad reducida a cero (`tts-crime-notoriety-cleared`).
     - Presencia de carga ilegal detectada en bodega al saltar a sistema gobernado (`tts-crime-illegal-cargo`).
     - Interdicción hostil por fuerzas de seguridad / ATR (`tts-crime-police-interdiction`).
4. **Catálogo de localización en los 5 idiomas (`locales/{es,en,fr,de,pt}/crime.ftl`):**
   - Todas las cadenas, explicaciones pedagógicas de la Guía del Ayudante y términos legales en español (referencia), inglés, francés, alemán y portugués.
5. **Interfaz de usuario («TMC Glass HUD»):**
   - Nuevo panel `CrimePanel.svelte` accesible en la barra lateral con icono táctico de balanza/justicia (`⚖️`).
   - Componentes modulares con tarjetas compactas y desplegables:
     - **Tarjeta de Estado Legal y Notoriedad:** Medidor horizontal de notoriedad 0..10, semáforo de estado legal (`LIMPIO`, `BUSCADO`, `CARGA ILÍCITA`) y contador de decaimiento en vuelo.
     - **Tarjeta «Guía de Limpieza de Nombre»:** Si notoriedad > 0, aviso pedagógico bloqueante y tiempo restante. Si notoriedad = 0, tarjeta del Interstellar Factor más cercano con botón de 1 clic para copiar sistema.
     - **Tarjeta de Contrabando y Mercados Negros:** Lista de mercancías robadas en bodega, consejos de aproximación sigilosa y Mercado Negro más cercano con botón de copia.
     - **Tarjeta de Desglose de Facciones y Penas:** Acordeón de multas y recompensas por facción.

### No objetivos
- Modificación en memoria del proceso de Elite Dangerous ni automatización de pilotaje.
- Rutas automáticas para eludir bloqueos (se ofrecen sugerencias informativas de vuelo).

---

## 3. Arquitectura y Flujo de Datos

```
[ Status.json (legal_state, flags) ]
[ Journal (CommitCrime, PayFines, PayBounties, Cargo, Docked) ]
                           │
                           ▼
                 [ CrimeModule (Rust) ]
                 - Notoriety tracker (decay timer)
                 - Fines & Bounties accumulator
                 - Stolen cargo evaluator
                 - Interstellar Factor / Black Market resolver
                           │
                           ├──────────────────────────────────┐
                           ▼                                  ▼
               Topic `crime.status`                [ VoiceAdvisor ] (tmc-core)
                           │                                  │
                           ▼                                  ▼
                 WebSocket / UI REST                    [ tmc-tts ]
                           │                                  │
                           ▼                                  ▼
                 `CrimePanel.svelte`                    Voz de cabina
                 (TMC Glass HUD)                     (Alertas tácticas)
```

---

## 4. Criterios de Aceptación (Verificación de Fase)

1. **Modelos y serialización:** Todos los eventos de crimen (`CommitCrime`, `PayFines`, `PayBounties`, etc.) deserializan de forma determinista y sin pánico ante esquemas antiguos o futuros.
2. **Lógica de Notoriedad:** El temporizador de notoriedad acumula tiempo de vuelo activo y descuenta 1 nivel cada 7.200 segundos; al atracar en una estación, el temporizador de decaimiento se congela correctamente.
3. **Guía de Limpieza de Nombre:** Con notoriedad > 0, la UI y el topic indican que los Interstellar Factors están bloqueados y muestran el tiempo de vuelo estimado. Con notoriedad 0 y multas/recompensas > 0, localizan el Interstellar Factor más cercano.
4. **Contrabando y Mercados Negros:** La presencia de carga con `Stolen > 0` genera una alerta de contrabando y localiza el Mercado Negro más cercano.
5. **Copiloto auditivo:** Al incrementarse la notoriedad o detectar carga robada en un sistema con autoridad, el copiloto emite la locución correspondiente.
6. **Localización:** 100% de cobertura en los 5 idiomas (`es`, `en`, `fr`, `de`, `pt`) verificado por los tests de `tmc-i18n`.
7. **Rendimiento e Higiene:** La suite completa de Rust y Vitest pasa en verde, `cargo clippy` limpio, y el consumo de memoria se mantiene dentro de los límites de < 150 MB y < 1% CPU en reposo.
