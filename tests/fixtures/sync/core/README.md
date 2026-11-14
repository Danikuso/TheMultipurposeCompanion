# Fixtures sintéticos del núcleo de sincronización (F17, T0)

Datos 100 % sintéticos (comandante `CMDR TESTER`, FID `F0000001`). No proceden de diarios reales.

| Archivo | Nombre destino en tests | Contenido | Uso |
|---|---|---|---|
| `session_a.01.log` | `Journal.2026-03-05T180001.01.log` | `Fileheader part=1`, termina en `Continued` (`Part`=2) | Sesión en dos partes, parte 1 |
| `session_a.02.log` | `Journal.2026-03-05T180001.02.log` | `Fileheader part=2`, continúa y termina en `Shutdown` | Sesión en dos partes, parte 2 |
| `other_session.01.log` | `Journal.2026-03-06T090001.01.log` | Lanzamiento distinto (otro día), `part=1`, sin `Continued` | `same_session(session_a.02, other_session.01) == false` |
| `legacy.01.log` | `Journal.2026-03-07T090001.01.log` | `gameversion` `3.8.0.407` (galaxia Legacy) | Nunca se sincroniza |
| `beta.01.log` | `Journal.2026-03-08T090001.01.log` | `gameversion` `4.1.0.0-Beta2` | Nunca se sincroniza (beta/alfa) |
| `two_cmdrs.01.log` | `Journal.2026-03-09T090001.01.log` | Dos `LoadGame`: `F0000001` y luego `F0000002` en el mismo archivo | Cursor y comandante por línea |

Notas: al usarlos en tests se copian a un directorio temporal con el nombre destino indicado (patrón `Journal.<fecha>.<part>.log`). `legacy.01.log` es de 3.8 (sin campo `Odyssey`, previo a Odyssey). Los `.log` no se normalizan (`.gitattributes`).
