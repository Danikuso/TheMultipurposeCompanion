# Spec: TMC Fase 10 — Asistencia e Integración de Voz (Zero-Friction Voice Assistant)

- **Fecha:** 2026-10-24
- **Autor:** Antigravity & Danikuso
- **Estado:** Propuesta para aprobación
- **Rama:** `fase/f10-asistencia-voz-integracion`
- **Ámbito:** `crates/tmc-model`, `crates/tmc-core`, `crates/tmc-server`, `crates/tmc-app`, `locales/`, `ui/`

---

## 1. Contexto y Filosofía de TMC (§0 AGENTS.md)

TMC es el **AYUDANTE (COMPANION)** integral para Elite Dangerous. En cabina, los comandantes tienen las manos ocupadas en los controles HOTAS/HOSAS o mando y no pueden estar tecleando ni cambiando de ventana en pleno vuelo, combate o aproximación planetaria.

La premisa innegociable de esta fase es:
> **«La app debe gestionarlo todo, el usuario no tiene que calentarse la cabeza para nada, solo activar la asistencia por voz.»**

Esto significa:
1. **Cero configuración compleja:** Nada de mapear variables manuales, instalar drivers raros, configurar puertos o pelearse con ficheros XML/VAP si no se desea.
2. **Asistente de voz autónomo incorporado:** TMC cuenta con su propio reconocimiento y procesamiento de voz directo. No depende obligatoriamente de software de terceros para responder preguntas y ejecutar acciones en cabina.
3. **Integración transparente con VoiceAttack:** Si el piloto ya usa VoiceAttack, TMC detecta su instalación automáticamente, mantiene sincronizado el perfil y responde a sus peticiones sin fricción.

---

## 2. Requisitos y Experiencia de Usuario

### 2.1 Activación con 1 Clic
En el encabezado del HUD (`TopBar.svelte`) y en Ajustes (`Settings.svelte`), existe un único control maestro:
- **«Activar asistencia por voz»**
Al encenderlo:
- El micrófono queda operativo para interactuar con TMC.
- Se activa el intérprete de intenciones de cabina.
- El Copiloto TTS (desarrollado en F7) vocaliza las respuestas.
- Se detecta en segundo plano si VoiceAttack está disponible para enlazarlo sin intervención del usuario.

### 2.2 Modos de Escucha
1. **Escucha con palabra clave (Wake Word):** El piloto dice *«Copiloto»*, *«TMC»* o *«Computadora»* seguido de su pregunta o instrucción.
2. **Pulsar para hablar (Push-to-Talk - PTT):** Botón o tecla rápida de cabina configurable para activar la escucha solo mientras se mantenga pulsada.
3. **Escucha directa en cabina:** Reconocimiento de frases de consulta frecuentes.

### 2.3 Repertorio de Consultas e Intenciones de Cabina (Voice Intents)
El piloto puede preguntar en lenguaje natural en cualquiera de los 5 idiomas soportados (`es`, `en`, `fr`, `de`, `pt`):

| Intención | Ejemplos de consulta hablada | Respuesta vocal del Copiloto | Acción de cabina |
|---|---|---|---|
| `ship_status` | *«¿Estado de la nave?»*, *«Informe de nave»* | *"Casco al 100%, escudos activos, combustible al 85%."* | Actualiza métricas HUD |
| `next_jump` | *«¿Cuál es el siguiente salto?»*, *«Siguiente sistema»* | *"Siguiente salto: Sol a 14 años luz, estrella repostable clase G."* | Ninguna / foco de ruta |
| `copy_next_system` | *«Copia el siguiente sistema»*, *«Copiar salto»* | *"Sistema Sol copiado al portapapeles."* | Copia al portapapeles |
| `nearest_factor` | *«¿Dónde está el factor interestelar más cercano?»* | *"Baird Gateway en LTT 1873 a 8,4 años luz. Sistema copiado."* | Copia sistema del factor |
| `nearest_black_market`| *«¿Dónde hay un mercado negro?»* | *"Black Hide en Wyrd a 12,5 años luz. Estación planetaria."* | Copia sistema de mercado |
| `crime_status` | *«¿Tengo multas?»*, *«¿Cuál es mi notoriedad?»* | *"Notoriedad limpia. Multas activas de 1.500 créditos con Sirius Corp."* | Abre panel crimen |
| `bio_signals` | *«¿Hay bio-señales?»*, *«Señales biológicas»* | *"Detectadas señales biológicas en 3 cuerpos del sistema."* | Ninguna |
| `materials_summary`| *«¿Qué materiales me faltan?»*, *«Materiales»* | *"Faltan 2 Manipuladores Químicos para tu receta fijada."* | Abre panel materiales |
| `nearest_engineer` | *«¿Dónde está Felicity Farseer?»* | *"Felicity Farseer se encuentra en Deciat, Farseer Inc."* | Copia sistema Deciat |
| `fuel_status` | *«¿Cuánto combustible me queda?»* | *"Combustible al 60%, 25 toneladas en tanque principal."* | Ninguna |

---

## 3. Arquitectura y Componentes

```
              [ Micrófono del Piloto ]
                         │
                         ▼
             [ Web / Native STT Engine ]
                         │
                         ▼ (Texto transcrito)
           POST /api/voice/intent (REST)
                         │
        ┌────────────────┴────────────────┐
        ▼                                 ▼
[ VoiceIntentParser ] (tmc-core)   [ VoiceAttack Bridge ]
- Extracción de intenciones y     - Detección automática
  entidades en 5 idiomas           - Perfil TMC-Copiloto.vap
- Consulta al GameState en vivo
        │
        ├─────────────────────────────────┐
        ▼                                 ▼
[ Ejecuta Acción ]              [ Respuesta Hablada ]
- Portapapeles (copiar sistema)  - VoiceCue táctico
- Navegación / HUD               - Emisión vía TtsQueue (F7)
```

1. **`tmc-model`:**
   - Modelos `VoiceIntent`, `VoiceCommandRequest`, `VoiceCommandResponse`, `VoiceAttackStatus`.
2. **`tmc-core`:**
   - Motor `VoiceIntentResolver` que analiza el texto y extrae la intención contra el `GameState` (nave, ubicación, multas, ingenieros, materiales, ruta).
   - Generación determinista del texto hablado en el idioma configurado.
3. **`tmc-server` & `tmc-app`:**
   - Endpoint `POST /api/voice/intent`: procesa cualquier comando de voz (del motor nativo de TMC o de VoiceAttack) y devuelve la respuesta.
   - Endpoint `GET /api/voice/voiceattack`: detecta el estado de VoiceAttack y provee el archivo de perfil `.vap` autogenerado.
4. **`locales/`:**
   - Cadenas de reconocimiento de patrones léxicos y respuestas habladas en `es`, `en`, `fr`, `de`, `pt`.
5. **`ui/` (TMC Glass HUD):**
   - Indicador de micrófono en `TopBar.svelte` con feedback reactivo de estados: Inactivo, Escuchando, Procesando, Hablando.
   - Subsección en `Settings.svelte`: Asistencia por voz, control de micro, PTT opcional y monitor de estado de VoiceAttack.
   - Píldora flotante o notificación de última orden reconocida.

---

## 4. Criterios de Aceptación

1. **Cero fricción:** Con un único clic en «Activar asistencia por voz», el sistema escucha y responde a comandos por voz sin requerir software adicional.
2. **Comprensión semántica multilingüe:** Reconocimiento de consultas clave de nave, navegación, factores interestelares, mercados negros, multas y bio-señales en los 5 idiomas oficiales.
3. **Acciones automáticas:** Comandos como «Copia el siguiente salto» o «Factor interestelar más cercano» copian el sistema correspondiente al portapapeles y avisan verbalmente.
4. **Sincronización con VoiceAttack:** Si VoiceAttack está presente en el equipo, TMC detecta su ruta y exporta/sirve el perfil compatible de forma totalmente automática.
5. **Rendimiento e Higiene:** < 150 MB RAM, 0% CPU adicional en reposo, suite Rust y Vitest en verde, Clippy sin advertencias.
