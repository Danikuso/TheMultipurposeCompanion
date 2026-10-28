# Spike de red: Búsqueda de Astilleros (Naves) y Equipamiento (Módulos) en Spansh

Pruebas reales realizadas el 2026-10-28 contra `https://spansh.co.uk/api/stations/*` con `User-Agent: TMC/0.1.0 (spike)`, intervalo >= 1,2 s entre peticiones y `Sol` (`x=0, y=0, z=0`) como referencia pública. Ningún dato de comandante enviado.

## 1. Esquema de filtros (`GET /api/stations/searchable_fields`)

- **`ships`** (`type: "combined"`, `multiple: true`):
  - Subcampos: `name` (`type: "group"`, primario), `price` (`type: "numeric"`, entero).
  - Formato válido verificado en `POST /api/stations/search`:
    ```json
    "ships": [{ "name": { "value": ["Mandalay"] } }]
    ```
- **`modules`** (`type: "combined"`, `multiple: true`):
  - Subcampos: `name` (`type: "group"`, primario), `class` (`type: "group"`, enteros `[0..8]`), `rating` (`type: "group"`, `["A".."I"]`), `price` (`type: "numeric"`).
  - Formato válido verificado para múltiples componentes simultáneos (semántica **AND** en servidor: devuelve estaciones que tienen **todos** los elementos a la vez):
    ```json
    "modules": [
      { "name": { "value": ["Frame Shift Drive (SCO)"] }, "class": { "value": [5] }, "rating": { "value": ["A"] } },
      { "name": { "value": ["Fuel Scoop"] }, "class": { "value": [6] }, "rating": { "value": ["A"] } }
    ]
    ```
  - **Importante:** Enviar `ships: { "value": [...] }` (en vez de array `[{ "name": { "value": [...] } }]`) provoca HTTP 500 `{"error":"Could not perform search"}`. Tanto `ships` como `modules` deben serializarse siempre como arrays de objetos con subcampos `{ "value": [...] }`.
- **Referencia espacial:**
  - Admite indistintamente `"reference_system": "Sol"` o `"reference_coords": { "x": 0.0, "y": 0.0, "z": 0.0 }`, con ordenación `"sort": [{ "distance": { "direction": "asc" } }]`. Esto garantiza funcionamiento incluso si el comandante consulta desde un sistema procedimental profundo aún no indexado en Spansh.
- **Filtros auxiliares en servidor:**
  - `type` (`group`): `["Asteroid base", "Coriolis Starport", "Dockable Planet Station", "Dodec Starport", "Drake-Class Carrier", "Mega ship", "Ocellus Starport", "Orbis Starport", "Outpost", "Planetary Construction Depot", "Planetary Outpost", "Planetary Port", "Settlement", "Space Construction Depot", "Surface Settlement"]`. Filtrar en servidor excluyendo `"Drake-Class Carrier"`, obras de construcción y asentamientos evita que los portaaviones agoten el cupo `size` cerca de sistemas concurridos.
  - `has_large_pad` (`boolean`): `{"value": true}` filtra en servidor estaciones con plataforma Grande (`L`).
  - `is_planetary` (`boolean`): `{"value": false}` permite excluir puertos planetarios si el usuario solicita solo estaciones orbitales.
  - `system_controlling_power` (`group`): `{"value": ["Li Yong-Rui"]}` filtra en servidor sistemas con el descuento del **15 %** en naves y equipamiento de *Li Yong-Rui*.

## 2. Catálogo vivo (`field_values/ships` y `field_values/modules`)

- **Naves (`48` modelos):** Incluye todas las naves clásicas y las incorporadas en 2024–2026 (`Python MkII`, `Type-8 Transporter`, `Mandalay`, `Cobra MkV`, `Corsair`, `Panther Clipper MkII`, `Caspian Explorer`, `Type-11 Prospector`, `Lynx Highliner`, `Kestrel Mk II`, etc.).
- **Módulos (`153` familias):** Clases `[0, 1, 2, 3, 4, 5, 6, 7, 8]` y calificaciones `["A", "B", "C", "D", "E", "F", "G", "H", "I"]`.

## 3. Forma de la respuesta y matices de dominio

Cada estación en `results[]` incluye:
- Metadatos de estación: `name`, `market_id`, `type`, `system_name`, `system_id64`, `distance` (al), `distance_to_arrival` (sl), `is_planetary`, `large_pads`, `medium_pads`, `small_pads`, `allegiance`, `system_controlling_power`, `shipyard_updated_at`, `outfitting_updated_at`, `updated_at`.
- **`ships[]`**: `[{ "name": "Mandalay", "price": 17639221, "symbol": "Mandalay" }, ...]`.
- **`modules[]`**:
  - Componentes internos/utilidades: `{ "category": "internal"|"standard"|"utility", "class": 5, "rating": "A", "name": "Frame Shift Drive (SCO)", "ed_symbol": "Int_Hyperdrive_Overcharge_Size5_Class5", "price": 6124743 }`.
  - Armas (`category: "hardpoint"`): añade `"weapon_mode": "Fixed" | "Gimbal" | "Turret"` (no filtrable como subcampo en la petición, pero verificable localmente sobre `st.modules`).
  - Blindajes (`Lightweight Alloy`, `Reinforced Alloy`, `Military Grade Composite`, `Mirrored Surface Composite`, `Reactive Surface Composite`): añade `"ship": "Diamondback Explorer"` (específico de modelo de nave; verificable localmente sobre `st.modules`).
- **Casos especiales detectados:**
  1. **`Stronghold Carrier` (Powerplay 2.0, 771 en la galaxia):** Aparece con `name: "Stronghold Carrier"`, `type: "Planetary Outpost"` y `market_id >= 3_900_000_000`, plataformas `S=4, M=4, L=2` y `system_controlling_power` con el nombre canónico de la potencia (`"Aisling Duval"`, `"A. Lavigny-Duval"`, `"Li Yong-Rui"`, etc.). Además, Spansh marca erróneamente `is_planetary: true` en algunos *Stronghold Carriers* orbitales: por tanto, cuando `!include_planetary`, el filtro planetario debe aplicarse **localmente solo a estaciones reales no-carrier**, permitiendo que el interruptor opcional **«Stronghold Carriers de tu potencia»** (`include_own_stronghold`) recupere exclusivamente aquellos cuyo `system_controlling_power` coincida con la potencia afiliada del comandante (`pledged_power`).
  2. **Sistemas con permiso (`Shinrarta Dezhra`, `Sol`, etc.):** Spansh **no** incluye bandera booleana `needs_permit` en la respuesta de estación (`Jameson Memorial` verificado). Debe cruzarse localmente con una tabla curada de sistemas poblados con permiso y con los rangos Élite del comandante en `GameState`.
  3. **Tamaño de carga útil:** Cada estación con astillero y equipamiento completo ocupa ~80–120 KB JSON. Conviene pedir `size: 15`–`20`, parsear únicamente los metadatos y los precios de los artículos solicitados, y descartar de inmediato el resto del JSON sin almacenarlo crudo en memoria.
