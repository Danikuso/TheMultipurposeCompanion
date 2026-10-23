# TMC · Plan de ejecución de F6 — Cuentas y Sincronización Externa (EDDN, EDSM, Inara, Frontier CAPI)

- **Fecha:** 2026-10-23
- **Spec de referencia:** [`docs/specs/2026-10-23-tmc-f6-cuentas-sincronizacion-design.md`](../specs/2026-10-23-tmc-f6-cuentas-sincronizacion-design.md)
- **Rama de trabajo:** `fase/f6-cuentas-sincronizacion`
- **Estado:** 🔄 en propuesta / pendiente de aprobación del propietario

---

## Bloques de trabajo y tareas

### Bloque 0: Modelos de datos y configuración (`tmc-model`)

#### Tarea 1: Modelos de sincronización, credenciales y topic de estado
- **Archivos:** `crates/tmc-model/src/accounts.rs`, `crates/tmc-model/src/settings.rs`, `crates/tmc-model/src/lib.rs`
- **Cambios:**
  - Estructuras: `AccountSettings` (switches para cada servicio, commander_name, uploader_salt, etc.).
  - Topic: `AccountsStatusTopic` (estado de cada servicio: conectado, último sync, mensajes de sesión, errores, rate limiting).
  - Envelopes y tipos para EDDN, EDSM, Inara y Frontier CAPI.
- **Verificación:** `cargo test -p tmc-model`
- **Commit:** `feat(model): modelos y topics para cuentas y servicios de sincronización`

---

### Bloque 1: Clientes y adaptadores de red (`tmc-net`)

#### Tarea 2: Emisor de esquemas oficiales a gateway EDDN (`tmc-net`)
- **Archivos:** `crates/tmc-net/src/eddn_sender.rs`, `crates/tmc-net/src/lib.rs`, `crates/tmc-net/src/limiter.rs`
- **Cambios:**
  - Serialización de esquemas (`commodity/3`, `journal/1`, `outfitting/2`, `shipyard/2`) con envelope oficial y `uploaderID` seudonimizado determinista (HMAC/SHA256 con sal local).
  - Petición HTTP POST segura a `https://eddn.edcd.io:4430/upload/`.
  - Descarte estricto ante modo Legacy (`Galaxy != Live`).
- **Verificación:** `cargo test -p tmc-net eddn`
- **Commit:** `feat(net): cliente emisor de esquemas oficiales a gateway EDDN con seudonimización`

#### Tarea 3: Cliente de sincronización EDSM (`tmc-net`)
- **Archivos:** `crates/tmc-net/src/edsm_sync.rs`, `crates/tmc-net/src/lib.rs`
- **Cambios:**
  - Sincronización con `https://www.edsm.net/api-logs-v1/` (`set-logs`, `set-commander-ranks`, `set-commander-credits`, `set-commander-materials`).
  - Rate limiting (10 req / 10s) y manejo de errores con backoff.
- **Verificación:** `cargo test -p tmc-net edsm_sync`
- **Commit:** `feat(net): cliente para sincronización de bitácora y comandante con EDSM`

#### Tarea 4: Cliente Inara InAPI v1 con lotes y rate limit estricto (`tmc-net`)
- **Archivos:** `crates/tmc-net/src/inara.rs`, `crates/tmc-net/src/lib.rs`, `crates/tmc-net/src/limiter.rs`
- **Cambios:**
  - Implementación del protocolo InAPI v1 (`https://inara.cz/inapi/v1/`).
  - Límite innegociable de 2 req/min (`Service::Inara`). Agrupación en lotes (*batching*) de eventos (`setCommanderProfile`, `setCommanderShip`, `setCommanderInventory`, `setCommanderShipLocker`, `setCommanderFleetCarrier`, `setCommanderTravelLog`).
- **Verificación:** `cargo test -p tmc-net inara`
- **Commit:** `feat(net): cliente inara con protocolo inapi v1, lotes y límite estricto de 2 req/min`

#### Tarea 5: Flujo OAuth2 PKCE y cliente Frontier CAPI (`tmc-net`)
- **Archivos:** `crates/tmc-net/src/capi.rs`, `crates/tmc-net/src/lib.rs`
- **Cambios:**
  - Generación de `code_verifier` y `code_challenge` S256 (`sha256 + base64url`).
  - Canje de token (`/token`), refresco transparente (`refresh_token`) y consulta a `/profile` para inventario remoto de flota de naves.
- **Verificación:** `cargo test -p tmc-net capi`
- **Commit:** `feat(net): cliente frontier capi con oauth2 pkce y extracción de flota remota`

---

### Bloque 2: Persistencia segura y endpoints de autenticación (`tmc-store`, `tmc-app`)

#### Tarea 6: Almacenamiento seguro de credenciales y tokens en SQLite (`tmc-store`)
- **Archivos:** `crates/tmc-store/src/accounts.rs`, `crates/tmc-store/src/lib.rs`
- **Cambios:**
  - Tabla `accounts_credentials` (servicio, api_key, cmdr_name, token_json, salt, updated_at).
  - Tests de persistencia, aislamiento y migración aditiva.
- **Verificación:** `cargo test -p tmc-store`
- **Commit:** `feat(store): persistencia de credenciales, sal de anonimización y tokens oauth2`

#### Tarea 7: Endpoint callback axum para OAuth2 de Frontier (`tmc-app`)
- **Archivos:** `crates/tmc-app/src/server.rs`, `crates/tmc-app/src/capi_service.rs`
- **Cambios:**
  - Rutas axum `/api/auth/frontier/login` y `/api/auth/frontier/callback` (captura del código y finalización del canje PKCE, redirigiendo con mensaje amigable de éxito).
- **Verificación:** `cargo test -p tmc-app capi`
- **Commit:** `feat(app): servidor axum con endpoints de login y callback loopback para frontier oauth2`

---

### Bloque 3: Módulo de cuentas y orquestador en el núcleo (`tmc-modules`)

#### Tarea 8: Módulo `accounts` en Rust (`tmc-modules`)
- **Archivos:** `crates/tmc-modules/src/accounts/mod.rs`, `crates/tmc-modules/src/accounts/tests.rs`, `crates/tmc-modules/src/registry.rs`
- **Cambios:**
  - Suscripción a eventos del diario (`FSDJump`, `Market`, `Outfitting`, `Shipyard`, `Rank`, `Loadout`, `ShipLocker`, `Materials`).
  - Despacho seguro: descarte absoluto en galaxia Legacy o si `online_services == false`.
  - Publicación periódica y reactiva del topic `accounts.status`.
- **Verificación:** `cargo test -p tmc-modules accounts`
- **Commit:** `feat(modules): módulo accounts con orquestación de envíos y filtrado estricto de galaxia`

---

### Bloque 4: Localización e i18n (`locales/`)

#### Tarea 9: Localización completa en 5 idiomas (`locales/`)
- **Archivos:** `locales/{es,en,fr,de,pt}/accounts.ftl`, `locales/{es,en,fr,de,pt}/main.ftl`
- **Cambios:**
  - Textos de estado, switches, cuotas de peticiones, explicaciones del Ayudante (Companion) y privacidad.
- **Verificación:** `cargo test -p tmc-i18n` (18 tests PASS simétricos en los 5 idiomas).
- **Commit:** `feat(i18n): claves y traducciones de cuentas y sincronización en 5 idiomas`

---

### Bloque 5: Interfaz de usuario («TMC Glass HUD»)

#### Tarea 10: Componentes y panel `AccountsPanel.svelte` (`ui/`)
- **Archivos:** `ui/src/modules/accounts/AccountsPanel.svelte`, `ui/src/modules/accounts/types.ts`, `ui/src/modules/accounts/index.ts`
- **Cambios:**
  - 4 tarjetas Glass HUD modulares: EDDN (métrica de sesión, uploaderID, terminal de envíos), EDSM (credenciales, 1.º descubrimientos, botón sync), Inara (medidor de 2 req/min, buffer de lote, botón enviar), Frontier CAPI (flota remota en astilleros con valor y ubicación, botón vincular/desvincular).
  - Tests unitarios en `ui/tests/accounts.test.ts`.
- **Verificación:** `npm test`
- **Commit:** `feat(ui): panel glass hud de cuentas y sincronización externa`

#### Tarea 11: Integración en el Shell y navegación (`ui/`)
- **Archivos:** `ui/src/shell/Sidebar.svelte`, `ui/src/shell/layouts.ts`, `crates/tmc-model/src/diagnostics.rs`
- **Cambios:**
  - Acceso desde la barra lateral (`🌐 Cuentas / Accounts`), registro en el catálogo de módulos y sincronización con `UI_PANELS` en diagnostics.
- **Verificación:** `npm test`, `cargo test -p tmc-model`
- **Commit:** `feat(ui): integración de navegación y disposición para cuentas y sincronización`

---

### Bloque 6: Verificación integral, calidad y build (`workspace`)

#### Tarea 12: Verificación integral, calidad, footprint y build
- **Archivos:** Todo el workspace
- **Verificación:**
  - Vitest en `ui/`
  - Svelte check en `ui/`
  - Tests completos de Rust en workspace
  - Clippy con `-D warnings`
  - Formato con `cargo fmt --all --check`
  - Bundle `ui/dist/`
  - Compilación de `target/debug/TMC.exe`
- **Commit:** `chore: verificación y cierre de F6 (Cuentas y Sincronización Externa)`
