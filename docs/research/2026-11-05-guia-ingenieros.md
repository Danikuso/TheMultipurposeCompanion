# Investigaci?n: gu?a de los 38 ingenieros

Fecha: 2026-11-05. F16 T0.

## Fuentes y uso

- Base incorporada: EDDiscovery/EliteDangerousCore, Engineers.cs, revisi?n 7d30920e6822d4836e0d6dd5c91026ec7e2ab4c7, Apache-2.0; snapshot y licencia ya presentes en data/import/engineering/snapshot. La matriz resume sus campos discovery/meeting/unlock/reputation, conservados en source_text del cat?logo. Se trata de datos ya autorizados por el propietario, sin incorporar conjuntos GPL/AGPL.
- Correcciones Live: [notas oficiales Frontier 18.08 publicadas en Steam](https://store.steampowered.com/news/posts/?appids=359320&enddate=1726826562&feed=steam_community_announcements), 2024-08-06, consultadas 2026-11-05: Hero 5 planes, Kit 5 encuestas, Wellington 15 datos de ocio en total, Yarden 5 campa?as. No aplicar las cantidades antiguas del snapshot a Live.
- Contraste actual: [Inara, directorio](https://inara.cz/elite/engineers/), consultado 2026-11-05; las cantidades reducidas siguen presentes. Se distingue desbloqueo de requisito de referencia en Odyssey. Consulta documental; no a?adir llamadas de la aplicaci?n ni importar un dataset externo.
- [Bill Turner](https://inara.cz/elite/engineer/19/), [Colonel Bris Dekker](https://inara.cz/elite/engineer/14/), [Marsha Hicks](https://inara.cz/elite/engineer/21/), [Tiana Fortune](https://inara.cz/elite/engineer/16/) y [Zacariah Nemo](https://inara.cz/elite/engineer/9/): contraste individual de los requisitos compuestos, donaciones y reputaci?n.
- [Yi Shen](https://inara.cz/elite/engineer/36/): se necesitan las tres referencias, no una donaci?n ?nica a Yi Shen.
- [Descripci?n de EngineerProgress](https://elite-journal.readthedocs.io/en/latest/Station%20Services.html): transcripci?n del manual de Frontier; RankProgress es porcentaje hacia el siguiente rango, Rank se recibe cuando hay acceso. El enum enumera estados pero no garantiza que Acquainted confirme todos los pasos. No se infiere una escala ordinal de la enumeraci?n. Los intentos de abrir el PDF oficial y las notas directamente en Frontier no fueron accesibles (403/URL no accesible); se usa la publicaci?n oficial de Frontier en Steam para las correcciones.

## Matriz del cat?logo (38 IDs ?nicos)

Los textos de esta tabla proceden de la base Apache-2.0 ya incorporada, con las cuatro correcciones Live se?aladas arriba. Las entregas Odyssey en la cuarta columna son **para referencias posteriores**, no pagos para desbloquear al ingeniero de la fila. Los textos visibles se redactar?n y traducir?n de nuevo en T1.

| ID / ingeniero | Descubrimiento | Requisito de acceso | Entrega o referencia | Reputaci?n |
|---|---|---|---|---|
| 300000 Didi Vatermann | From Selene Jean (grade 3-4). | Gain trade rank Merchant or higher. | Provide 50 units of Lavian Brandy. | Craft modules for a major increase.Sell commodities to Vatermann LLC. |
| 300010 Bill Turner | Referencia de Selene Jean (grado 3 y un tercio hacia 4). | Gain Friendly status with Alliance. You will also need Allied status with Alioth Independents to get a permit to access the Alioth starsystem. | Provide 50 units of Bromellite. | Craft modules for a major increase. Sell commodities to Turner Metallics Inc. |
| 300030 Broo Tarquin | From Hera Tani (grade 3-4). | Gain combat rank Competent or higher. | Provide 50 units of Fujin Tea. | Craft modules for a major increase. Hand in bounty vouchers to Broo's Legacy. |
| 300040 The Sarge | From Juri Ishmaak (grade 3-4). | Gain rank Midshipman or higher with the Federal Navy. | Provide 50 units of Aberrant Shield Pattern Analysis. | Craft modules for a major increase. Sell exploration data and hand in bounty vouchers to The Beach. |
| 300050 Zacariah Nemo | From Elvira Martuuk (grade 3-4). | Gain invitation from Party of Yoru. | Provide 25 units of Xihe Companions. | Craft modules for a major increase. Sell commodities to Nemo Cyber Party Base. |
| 300080 Liz Ryder | Public sources. | Gain Cordial or Friendly status with Eurybia Blue Mafia. | Provide 200 units of Landmines. | Craft modules for a major increase. Sell commodities to Demolition Unlimited. |
| 300090 Hera Tani | From Liz Ryder (grade 3-4). | Gain rank Outsider or higher with the Empire. | Provide 50 units of Kamitra Cigars. | Craft modules for a major increase. Sell commodities to The Jet's Hole. |
| 300100 Felicity Farseer | Public data sources. | Gain exploration rank Scout or higher. | Provide 1 unit of Meta Alloys. | Craft modules for a major increase. Sell exploration data at Farseer Inc. |
| 300110 Ram Tah | From Lei Cheung (grade 3-4). | Gain exploration rank Surveyor or higher. | Provide 50 units of Classified Scan Databanks. | Craft modules for a major increase. Sell exploration data to Phoenix Base. |
| 300120 Lei Cheung | From The Dweller (grade 3-4). | You have traded in over 50 markets. | Provide 200 units of Gold. | Craft modules for a major increase. Sell commodities to Trader's Rest. |
| 300130 Petra Olmanova | From Tod McQuinn (grade 3-4). | Gain combat rank Expert or higher. | Provide 200 units of Progenitor Cells. | Craft modules for a major increase. |
| 300140 Colonel Bris Dekker | From Juri Ishmaak (grade 3-4). | Friendly with the Federation. | Provide 1,000,000 or 10,000,000 credits worth of federal combat bonds. | Craft modules for a major increase. Hand in bounty vouchers to Dekker's Yard. |
| 300150 Marsha Hicks | From The Dweller (grade 3-4). | Gain exploration rank Surveyor or higher. | Mine 10 units of Osmium. | Craft modules for a major increase. |
| 300160 Elvira Martuuk | Public knowledge | Attain a maximum distance from your career start location of at least 300 light years. | Provide 3 units of Soontill Relics. | Craft modules for a major increase. Sell exploration data at Long Sight Base. |
| 300180 The Dweller | Common knowledge | Deal with at least 5 black markets. | Pay 500,000 credits. | Craft modules for a major increase. Sell commodities to Black Hide. |
| 300200 Marco Qwent | From Elvira Martuuk (grade 3-4). | Gain invitation from Sirius Corporation. | Provide 25 units of Modular Terminals. | Craft modules for a major increase. Sell commodities to Qwent Research Base. |
| 300210 Selene Jean | From Tod McQuinn (grade 3-4). | Mine at least 500 tons of ore. | Provide 10 units of Painite. | Craft modules for a major increase. Sell commodities and exploration data to Prospector's Rest. |
| 300220 Professor Palin | From Marco Qwent (grade 3-4). | Attain a maximum distance from your career start location of at least 5,000 light years. | Provide 25 units of Sensor Fragments. | Craft modules for a major increase. Sell exploration data to Abel Laboratory. |
| 300230 Lori Jameson | From Marco Qwent (grade 3-4). | Gain combat rank Dangerous or higher. | Provide 25 units of Kongga Ale. | Craft modules for a major increase. Sell exploration data to Jameson Base. |
| 300250 Juri Ishmaak | From Felicity Farseer (grade 3-4). | Earn more than 50 combat bonds. | Provide 100,000 or 1,000,000 credits worth of combat bonds. | Craft modules for a major increase. Hand in bounty vouchers or combat bonds to Pater's Memorial. |
| 300260 Tod 'The Blaster' McQuinn | Common knowledge. | Earn more than 15 bounty vouchers. | Provide 100,000 credits worth of bounty vouchers. | Craft modules for a major increase. Hand in Alliance bounty vouchers to Trophy Camp. |
| 300270 Tiana Fortune | From Hera Tani (grade 3-4). | Friendly with the Empire. | Provide 50 units of Decoded Emission Data. | Craft modules for a major increase. Sell commodities to Fortune's Loss. |
| 300280 Mel Brandon | From Elvira Martuuk (grade 3-4). | Gain invitation from Colonia Council. | Provide 100,000 credits worth of bounty vouchers. | Craft modules for a major increase. |
| 300290 Etienne Dorn | From Liz Ryder (grade 3-4). | Gain trade rank Dealer or higher. | Provide 25 units of Occupied Escape Pods. | Craft modules for a major increase. |
| 300300 Chloe Sedesi | From Marco Qwent (grade 3-4). | Attain a maximum distance from your career start location of at least 5,000 light years. | Provide 25 units of Sensor Fragments. | Craft modules for a major increase. Sell exploration data to Cinder Dock. |
| 400001 Jude Navarro | Common knowledge. | Complete 10 Restore or Reactivation missions. | Provide 5 units of Genetic Repair Meds. | Sin niveles de reputaci?n G1?G5; completar referencias a otros contactos. |
| 400002 Domino Green | Common knowledge. | Travel at least 100 light years in shuttles. | Provide 5 doses of Push. | Sin niveles de reputaci?n G1?G5; completar referencias a otros contactos. |
| 400003 Hero Ferrari | Common knowledge. | Complete 10 surface conflict zones. | Provide 5 Settlement Defence Plans (referral to Wellington Beck). | Sin niveles de reputaci?n G1?G5; completar referencias a otros contactos. |
| 400004 Kit Fowler | From Domino Green | Sell 5 Opinion Polls to bartenders. | Provide 5 units of Surveillance equipment. | Sin niveles de reputaci?n G1?G5; completar referencias a otros contactos. |
| 400005 Wellington Beck | From Hero Ferrari | Sell 15 total Multimedia Entertainment, Classic Entertainment and Cat Media to bartenders. | Provide 5 units of Insight Entertainment suites. | Sin niveles de reputaci?n G1?G5; completar referencias a otros contactos. |
| 400006 Terra Velasquez | From Jude Navarro | Complete 6 Covert theft and Covert heist missions. | Provide 15 Financial projections. | Sin niveles de reputaci?n G1?G5; completar referencias a otros contactos. |
| 400007 Uma Laszlo | From Wellington Beck | Reach Unfriendly reputation or lower with Sirius Corporation. | Sin entrega adicional documentada. | Sin niveles de reputaci?n G1?G5; completar referencias a otros contactos. |
| 400008 Oden Geiger | From Terra Velasquez | Sell a total of 20 Biological sample, Employee genetic data and Genetic research to bartenders. | Sin entrega adicional documentada. | Sin niveles de reputaci?n G1?G5; completar referencias a otros contactos. |
| 400009 Yarden Bond | From Kit Fowler | Sell 5 Smear Campaign Plans to bartenders. | Sin entrega adicional documentada. | Sin niveles de reputaci?n G1?G5; completar referencias a otros contactos. |
| 400010 Baltanos | Common knowledge | Reach Friendly reputation with the Colonia Council. | Provide 10 Faction Associates data. | Sin niveles de reputaci?n G1?G5; completar referencias a otros contactos. |
| 400011 Eleanor Bresa | Common knowledge. | Visit 5 settlements in the Colonia system. | Provide 10 Digital Designs. | Sin niveles de reputaci?n G1?G5; completar referencias a otros contactos. |
| 400012 Rosa Dayette | Common knowledge. | Sell a total of 10 Culinary Recipes or Cocktail Recipes to stations in Colonia. | Provide 10 units of Manufacturing Instructions data. | Sin niveles de reputaci?n G1?G5; completar referencias a otros contactos. |
| 400013 Yi Shen | From Baltanos, Eleanor Bresa and Rosa Dayette | Complete referral tasks for Baltanos, Eleanor Bresa and Rosa Dayette. | Sin entrega adicional documentada. | Sin niveles de reputaci?n G1?G5; completar referencias a otros contactos. |

## Interpretaci?n para la gu?a

| Estado del diario | Descubrimiento | Invitaci?n | Desbloqueo |
|---|---|---|---|
| Known | Confirmado | Paso actual: cumplir condici?n de acceso | Pendiente |
| Invited | Confirmado | Confirmado | Paso actual: visitar taller y completar tarea |
| Acquainted | Sin confirmaci?n adicional inferida | Sin confirmaci?n adicional inferida | Consultar tarea y estado del juego; no afirmar desbloqueo |
| Unlocked | Confirmado | Confirmado | Confirmado |
| Barred | Sin inferencias | Sin inferencias | Acceso vetado; instrucciones de referencia visibles |
| Unknown / futuro / sin diario | Desconocido: consultar descubrimiento | Desconocido | Desconocido |

Los ingenieros a pie no usan reputaci?n G1?G5. En su resumen desbloqueado debe mostrarse orientaci?n para obtener referencias o instalar modificaciones. No confundir el grado del traje/arma con reputaci?n del ingeniero.

Para nave, una referencia de grado 3?4 requiere llegar a grado 3 y aproximadamente un tercio hacia el 4; la gu?a explica esa condici?n, pero no declara cumplimiento autom?tico. Se conserva el estado comunicado por el diario.

## Disponibilidad y l?mites de la telemetr?a

- Bodega actual, materiales y saldo confirmado permiten calcular disponible/requerido/faltante. No permiten confirmar donaciones ni ventas hist?ricas.
- Una mercanc?a comprada no prueba extracci?n minera: no modelar las entregas de Selene Jean o Marsha Hicks como una contribuci?n simple de bodega.
- Bonos de combate y recompensas no equivalen al saldo de cr?ditos. No representarlos como contribuci?n credits ni derivarlos del saldo.
- Bris Dekker y Juri tienen cantidades alternativas en la fuente (1/10 M y 0,1/1 M respectivamente); mostrar ambas y pedir consultar la cantidad del taller, sin fabricar un umbral ?nico ni barra porcentual. La variaci?n queda documentada dentro de la instrucci?n concreta.
- Permisos: Alioth requiere Allied con Alioth Independents; Sirius requiere invitaci?n de Sirius Corporation y permiso; Shinrarta Dezhra exige permiso (normalmente primer Elite); Sol y Achenar exigen sus permisos. No basta con la referencia del ingeniero anterior.
- Odyssey: acceso al taller por actividad/reputaci?n; la entrega de referencia al siguiente contacto se explica despu?s del acceso. Las tres entregas de Colonia se muestran completas en la ruta de Yi Shen.

## Verificaci?n T0

Conteo automatizado: 38 entradas, 38 IDs ?nicos. La matriz incluye cada ID exactamente una vez. Se conservan las condiciones compuestas y los casos de cuant?a variable expl?citos. Ninguna comprobaci?n en vuelo realizada; guide_verified significar? contraste documental, separado del verified del cat?logo de capacidades.
