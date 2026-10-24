# AGENTS.md — Cómo se trabaja en TMC

Guía obligatoria para cualquier persona, modelo o agente que trabaje en **TMC — The Multipurpose Companion** (companion en tiempo real para Elite Dangerous). Léela entera antes de tocar nada.

## 0. Propósito y filosofía de TMC: El AYUDANTE (Companion) integral

TMC (**The Multipurpose Companion**) existe con un propósito fundamental e innegociable: servir como **AYUDANTE (COMPANION)** en **todo** lo que se pueda hacer en *Elite Dangerous*, siendo el copiloto indispensable tanto para los pilotos **más novatos** como para los comandantes **más experimentados**:

- **Nunca dejar al jugador bloqueado o en un callejón sin salida:** Si un piloto intenta realizar una acción o explorar una posibilidad (por ejemplo, ingenierizar un módulo de su nave, virgen o ya modificado) y todavía no dispone de acceso al ingeniero o ingenieros correspondientes (están bloqueados, no conocidos o no invitados), la aplicación **debe actuar de manera proactiva como un guía/ayudante**: mostrar primero de manera clara, pedagógica y secuencial los **pasos para desbloquearlo/s** (requisitos de descubrimiento, invitación y donación/desbloqueo), y **después** el cálculo de los materiales, recetas y soluciones de intercambio.
- **Claridad instantánea y comprensible:** La información nunca se presenta como un volcado crudo de tablas o datos sin procesar. Se emplean herramientas visuales de cabina intuitivas: steppers horizontales de progreso, semáforos de existencias, barras de llenado y tarjetas métricas de lectura rápida sin cálculos mentales.
- **Acción directa al servicio del piloto:** Cada pantalla debe ofrecer soluciones con un solo clic (copiar sistemas o rutas al portapapeles, calcular intercambios con los excedentes reales de la bodega, indicar el comerciante más cercano).
- **Sistema de diseño obligatorio:** Cualquier pantalla, módulo o componente actual y futuro debe seguir rigurosamente las pautas normativas del sistema de diseño [«TMC Glass HUD»](docs/design-system-glass-hud.md) (superficies de cabina, jerarquía métrica, medidores visuales, steppers y píldoras semánticas).

## 1. Primero, siempre

1. Lee **[`status.md`](status.md)**: dice en qué fase, plan, tarea y paso exacto está el desarrollo y si hay bloqueos.
2. Lee la **spec** y el **plan** que `status.md` indica como activos.
3. Continúa exactamente desde el punto que marca `status.md`. No empieces otra tarea ni rehagas trabajo marcado como hecho.

Si `status.md` y el repositorio no coinciden (p. ej. una tarea marcada como hecha sin su commit), **para**, documenta la discrepancia en `status.md` → «Bloqueos» y avisa al propietario.

## 2. Fuentes de verdad (en orden de prioridad)

| Documento | Ubicación | Para qué |
|---|---|---|
| Instrucciones del propietario (Danikuso) | conversación / issues | Mandan sobre todo lo demás |
| **Specs** | `docs/specs/AAAA-MM-DD-<tema>-design.md` | **Qué** se construye y por qué. Aprobadas por el propietario |
| **Plans** | `docs/plans/AAAA-MM-DD-<tema>.md` | **Cómo** se construye: tareas, interfaces, tests |
| **Estado** | `status.md` | **Dónde** estamos exactamente |
| Investigación | `docs/research/` | Contexto y fuentes (APIs, formato del diario, fórmulas) |

Un plan nunca contradice a su spec. Si al implementar descubres que la spec es incorrecta o incompleta, **no improvises**: anótalo en `status.md` → «Bloqueos», propón el cambio y espera aprobación.

## 3. Flujo de trabajo

```
idea/fase → SPEC (aprobada por el propietario) → PLAN (aprobado) → EJECUCIÓN tarea a tarea → REVISIÓN → status.md actualizado
```

1. **Spec.** Toda funcionalidad nueva o fase (ver hoja de ruta en la spec de arquitectura §13) empieza con una spec en `docs/specs/`. Sin spec aprobada no hay plan.
2. **Plan.** Se deriva de una spec aprobada. Vive en `docs/plans/`, con tareas pequeñas, interfaces exactas (nombres, firmas, tipos), tests con valores concretos y un commit por tarea. Sin plan aprobado no hay código.
3. **Ejecución.** Una tarea cada vez, en el orden del plan, siguiendo TDD:
   1. escribir el test → 2. verlo fallar → 3. implementar lo mínimo → 4. verlo pasar → 5. `fmt` + `clippy` + tests del workspace → 6. commit → 7. **actualizar `status.md`** (en el mismo commit o en uno inmediatamente posterior).
4. **Revisión.** Al terminar cada tarea, revisión contra el plan y la spec (requisitos, interfaces, tests). Al terminar una fase, revisión completa de la rama y verificación de los criterios de aceptación de la spec, con evidencias (comando + salida).
5. **Cierre de fase.** Se marca en `status.md`, se anotan las desviaciones conscientes y se pasa a la spec de la siguiente fase.

**Desviaciones:** si una tarea no puede hacerse como dice el plan, documenta en el propio plan (sección «Desviaciones») qué cambió y por qué, y refléjalo en `status.md`. Los cambios de interfaz que afecten a tareas posteriores deben actualizarse también en sus bloques *Interfaces*.

## 4. `status.md`: reglas de actualización

`status.md` debe permitir que **otro agente sin contexto** continúe en el punto exacto. Actualízalo:

- al **empezar** una tarea (estado `🔄 en curso`, paso actual);
- al **terminar** cada paso relevante (último paso completado);
- al **terminar** una tarea (`✅ hecha`, hash del commit, tests que pasan);
- al **encontrar un bloqueo** o tomar una decisión que no está en el plan;
- antes de **terminar tu sesión**, aunque la tarea quede a medias (qué falta exactamente y qué archivos están tocados sin commit).

Formato fijo (no lo cambies sin motivo):

- **Cabecera:** fecha/hora de la última actualización, quién la hizo (modelo/agente/persona), fase, spec y plan activos.
- **Puntero actual:** tarea y paso exactos + siguiente acción concreta.
- **Tabla de tareas:** `⬜ pendiente` · `🔄 en curso` · `✅ hecha` · `⛔ bloqueada` · `⏭️ omitida (con motivo)`; con commit y notas.
- **Bloqueos y pendientes del propietario.**
- **Decisiones tomadas durante la ejecución** (con fecha).
- **Registro de sesiones** (una línea por sesión, la más reciente arriba).

Nunca marques algo como hecho sin haber ejecutado su verificación y visto la salida.

## 5. Reglas técnicas

- **Stack:** Rust estable (edición 2024) en workspace de Cargo; Tauri 2; Svelte 5 + TypeScript + Vite; SQLite (rusqlite); axum; Fluent para i18n. No cambies de tecnología sin una spec aprobada.
- **Arquitectura:** crates pequeños con una responsabilidad (`crates/tmc-*`), apps que solo componen (`apps/*`), UI en `ui/`, traducciones en `locales/`. Respeta la tabla de dependencias de la spec de arquitectura §3.1 (sin ciclos).
- **Módulos:** cada rol del juego es un módulo (`crates/tmc-modules/src/<id>/` + `ui/src/modules/<id>/` + una entrada en cada registro). No metas lógica de un rol en el núcleo.
- **Rendimiento (requisito, no deseo):** < 150 MB RAM, < 1 % CPU en reposo, sin animaciones infinitas ni `requestAnimationFrame` continuo, máx. 10 actualizaciones/s a la UI. Si un cambio puede afectar al consumo, pasa `scripts/measure-footprint.ps1`.
- **Robustez:** líneas corruptas, eventos desconocidos, archivos bloqueados o servicios caídos nunca tumban la app.
- **Lectura del diario:** por sondeo con offset de bytes; las notificaciones del SO solo para detectar archivos nuevos.
- **i18n:** ninguna cadena visible en el código; todo en `locales/{es,en,fr,de,pt}/*.ftl`. `es` es la referencia y todos los idiomas tienen las mismas claves (hay test que lo comprueba).
- **Calidad:** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`; en `ui/`: `npm run check`, `npm test`, `npm run build`. Todo en verde antes de cada commit.
- **Código en inglés**; documentación, commits y textos de referencia en **español**.

## 6. Licencia, datos y privacidad

- Licencia del proyecto: **PolyForm Noncommercial 1.0.0**. **Prohibido** incorporar código o datos con licencia GPL/AGPL u otras incompatibles (p. ej. datos de SrvSurvey). Usar APIs AGPL por red (Ardent) sí está permitido. Ante la duda, pregunta.
- Toda dependencia nueva se anota en `THIRD_PARTY_NOTICES.md` con su licencia.
- **Secretos nunca en el repo:** Client ID de Frontier (`TMC_CAPI_CLIENT_ID`), claves de Inara/EDSM, tokens. Van en `.env` local (ignorado) o en el almacén de credenciales del SO.
- **Datos del jugador:** los diarios reales solo se usan como fixtures tras pasar por `tmc-replay anonymize` y revisión manual. Nunca subas nombres de comandante, FID ni chats reales.
- No enviar datos a servicios externos salvo en las fases y con las opciones que la spec define (opt-out visible para el usuario). Nunca enviar datos de la galaxia Legacy.
- Respeta los límites de cada API (Inara 2 peticiones/min, EDSM 10/10 s, etc.; ver `docs/research/`).
- Marca registrada: «Elite Dangerous» es de Frontier Developments; no uses sus logotipos ni recursos gráficos.

## 7. Git

- Rama principal: `main`. Repositorio privado: `github.com/Danikuso/TheMultipurposeCompanion`.
- Para cada fase, trabaja en una rama `fase/fN-<nombre>` (p. ej. `fase/f0-cimientos`) y fusiona en `main` al cerrar la fase con la revisión completa hecha.
- Commits pequeños, uno por tarea como mínimo, con prefijo convencional en español: `feat(crate): …`, `fix: …`, `test: …`, `docs: …`, `chore: …`.
- Los commits hechos por IA terminan con la línea de coautoría correspondiente, p. ej. `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- No reescribas historia publicada (`push --force`) ni uses `--no-verify`.
- No hagas push a GitHub público ni cambies la visibilidad del repo sin orden explícita del propietario.

## 8. Comandos útiles

```bash
cargo test --workspace                       # tests Rust
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p tmc-headless -- --journal-dir <carpeta>   # núcleo + servidor sin ventana
cargo run -p tmc-replay -- play --from tests/fixtures/journal/basic_session.log --to <carpeta> --speed 10
npm --prefix ui run dev                      # UI en desarrollo (proxy a 127.0.0.1:47320)
cargo tauri build                            # instalador NSIS (desde apps/desktop)
pwsh scripts/measure-footprint.ps1 -Exe target/release/TMC.exe -JournalDir <carpeta>
```

(Los comandos existen a medida que se completan las tareas del plan de F0.)

## 9. Cuándo parar y preguntar al propietario

- La spec o el plan son ambiguos o contradictorios.
- Un cambio afecta al alcance, a la licencia, a la privacidad o a servicios externos.
- Se necesita una cuenta, clave o aprobación externa (Frontier, Inara, EDSM).
- Un criterio de aceptación no se puede cumplir.

Antes de parar, deja `status.md` actualizado con la pregunta concreta en «Bloqueos».

## 10. Higiene de disco, compilación y procesos (Obligatorio)

Para evitar el crecimiento descontrolado del espacio en disco (que llegó a alcanzar 75 GB por acumulación de artefactos obsoletos y duplicados) y garantizar una experiencia fluida con el usuario:

- **Directorio `target` único y canónico:**
  - Queda **estrictamente prohibido** definir o compilar en directorios de salida ad-hoc (como `target-f3/`, `target-f4/`, `target/diagnostic-*`, etc.) mediante `CARGO_TARGET_DIR` o configuraciones temporales. Todas las compilaciones de Rust deben dirigirse exclusivamente al `target/` estándar del workspace.
  - La duplicación de carpetas target multiplica por decenas de gigabytes las dependencias cacheadas innecesariamente.
- **Higiene periódica de Cargo y tests:**
  - El workspace cuenta con más de 1.400 tests. La ejecución de tests genera cientos de binarios independientes con símbolos completos de depuración (`.pdb`) en `target/debug/deps/` y árboles de compilación incremental en `target/debug/incremental/`.
  - Si la carpeta `target/` supera los 15-20 GB, o al finalizar una fase importante, se debe realizar un saneamiento mediante `cargo clean` o purgando los ejecutables antiguos de test.
- **Limpieza obligatoria e inmediata de directorios temporales:**
  - Prohibido dejar carpetas o volcados temporales huérfanos en la raíz del proyecto (p. ej. `.tmp/`, `debug_logs/`, volcados de footprint o trazas de diagnóstico).
  - Cualquier script, prueba o comando que genere archivos auxiliares temporales debe eliminarlos inmediatamente al finalizar la tarea, o utilizar el directorio temporal del sistema (`$env:TEMP`).
- **Prohibición estricta de lanzar procesos con interfaz gráfica (GUI) en segundo plano:**
  - **NUNCA** ejecutar la aplicación de escritorio (`TMC.exe`, `cargo run -p tmc-desktop`, `cargo tauri dev`, etc.) como tarea en segundo plano / daemon (`IsDaemon: true`).
  - *Causa:* Los comandos en segundo plano ejecutados por el agente corren en una sesión de Windows no interactiva (sin acceso a WinSta0), por lo que la ventana NO es visible para el usuario ni aparece en la barra de tareas. Esto genera procesos huérfanos y obliga al usuario a cancelar la ejecución manualmente.
  - *Protocolo obligatorio:* El agente compila los binarios (`cargo build -p tmc-desktop --release` y/o `cargo build -p tmc-desktop` para Debug), verifica que el binario exista y entrega las rutas exactas al usuario para que él lo ejecute directamente en su sesión interactiva.

