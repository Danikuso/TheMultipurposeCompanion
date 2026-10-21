# TMC — Auto-companion: interruptor para fijar la disposición

**Estado:** ✅ aprobada por el propietario (2026-10-21)

## Intención

Dar al jugador control sobre los cambios automáticos de la interfaz: poder fijar la disposición y el panel de contexto y que TMC no los cambie por su cuenta.

## Comportamiento

- **Activado (por defecto, comportamiento previo):** la UI cambia de disposición según la relevancia de los módulos (`autoLayout`/`nextPin`) y cambia el panel de contexto de exploración (cuerpos/exobiología) automáticamente.
- **Desactivado:** nunca se cambia sola de disposición ni de panel de contexto (entrar en un sistema, atracar, fijar un objetivo, etc. no cambian nada). Los datos siguen actualizándose y las alertas/avisos se muestran igual. La navegación manual funciona con normalidad. Al reactivarlo, el modo automático se aplica de inmediato (saltar a la disposición automática es lo esperado).
- **Arranque:** hasta leer `auto_companion` de `/api/config` la UI no aplica ningún cambio automático (sin carrera); si la lectura falla o tarda, se usa el valor por defecto (activado). Con el interruptor apagado, la última disposición y el panel de contexto elegidos se guardan por cliente en `localStorage` (opcional, sin él no pasa nada) y se restauran al arrancar; activado ignora ese valor.
- **Escrituras:** los cambios del interruptor se serializan; si el PUT falla se vuelve al último valor confirmado.

## Interfaz y persistencia

- Interruptor accesible (`role="switch"`, etiqueta y tooltip) en la barra superior y casilla equivalente en Ajustes.
- Se guarda como `auto_companion: bool` en `config.toml` (`serde(default)` = `true`; las configuraciones antiguas conservan el comportamiento previo). Se cambia con el mismo `PUT /api/config`; si falla, el interruptor se revierte.
- Textos en los 5 idiomas (`auto-companion`, `auto-companion-tooltip`).
- Diagnóstico (solo debug): `auto_companion` en la instantánea de configuración y en `ui.app.layout`.

## Fuera de alcance

- Alertas, avisos y actualización de datos (no cambian).
- Reglas de relevancia de los módulos.
- Cambios por módulo (el interruptor es global).
