# Fixtures sintéticos del núcleo de sincronización (F17, T0)

Datos 100 % sintéticos (comandante `CMDR TESTER`, FID `F0000001`). No proceden de diarios reales.

| Archivo | Contenido | Uso |
|---|---|---|
| `session_a.01.log` | `Fileheader part=1`, termina en `Continued` (`Part`=2) | Sesión en dos partes, parte 1 |
| `session_a.02.log` | `Fileheader part=2`, continúa y termina en `Shutdown` | Sesión en dos partes, parte 2 |
| `other_session.01.log` | Lanzamiento distinto (otro día), `part=1`, sin `Continued` | `same_session(session_a.02, other_session.01) == false` |
| `legacy.01.log` | `gameversion` `3.8.0.407` (galaxia Legacy) | Nunca se sincroniza |
| `beta.01.log` | `gameversion` `4.1.0.0-Beta2` | Nunca se sincroniza (beta/alfa) |
| `two_cmdrs.01.log` | Dos `LoadGame`: `F0000001` y luego `F0000002` en el mismo archivo | Cursor y comandante por línea |

Notas: los nombres siguen el patrón `Journal.<fecha>.<part>.log` solo en tests que los copien con ese nombre
a un directorio temporal; aquí se conservan con nombre corto. Los `.log` no se normalizan (`.gitattributes`).
