# Specialized third-party APIs and static reference datasets for Elite Dangerous tools (as of 2026-10-17)

Method note: most "live" statements below come from requests I made on 2026-10-17 to the services themselves (marked "live test"), from the services' own front-end source, or from GitHub metadata queried via `gh api` on the same day. Where a claim rests on my own test, the cited URL is the endpoint tested.

## 1. Spansh (spansh.co.uk): endpoints, async jobs, limits, terms, dumps, status

### Takeaway
Spansh is alive and heavily used (about 30M requests/month by the author's own figure); every plotter is an async job (submit -> `job` GUID -> poll `/api/results/{job}`), and system/body/station search is a JSON POST API. Only 4 GET endpoints are officially documented (OpenAPI at docs.spansh.co.uk); everything else is undocumented but stable enough that many third-party tools use it. No published rate limit or ToS was found.

### Cited Findings
**Status / scale / author statements**
- Site is live; Ember front-end app version string `0.1.903+d0f304f7`; API served under `https://spansh.co.uk/api`; downloads served from `https://downloads.spansh.co.uk` — [spansh.co.uk](https://spansh.co.uk/) (page `<meta>` config)
- Author (Patreon): the site offers "a powerful and fast API used by many of the third party software packages such as ED Market Connector, ED Discovery and Canonn Research"; site serves "roughly 30million requests a month" (~20,000 unique users); ad-free; hosted on 2 bare-metal backends at Hetzner — [Spansh Patreon](https://www.patreon.com/spansh/about)
- Author (Patreon): "These APIs are already used by a lot of the third party software and sites which help the players"; documentation was tied to funding goals ($300/month = JSON Schema for the dumps; $350/month = documentation of the major API calls) — [Spansh Patreon](https://www.patreon.com/spansh/about)
- Official OpenAPI 3.1 docs exist: "Spansh API" v3.1.1, server `https://spansh.co.uk/api`, but they document only four GET endpoints: `/dump/{id64}`, `/system/{id64}`, `/body/{id64}`, `/station/{marketId}` (200/404 responses) — [docs.spansh.co.uk](https://docs.spansh.co.uk/)
- JSON Schemas for dumps are published: `galaxy.schema.json` (also used for galaxy_populated/stations/1day/7days/1month), `systems.schema.json`, `factions.schema.json` — [galaxy.schema.json](https://docs.spansh.co.uk/galaxy.schema.json)
- Rate limits/terms: no `RateLimit-*` headers were returned by `/api/system/10477373803` (only nginx, HSTS, X-Frame-Options); I found no published ToS or rate-limit text on the site, docs or Patreon — live test [spansh.co.uk/api/system/10477373803](https://spansh.co.uk/api/system/10477373803)

**Async job pattern (verified live)**
- Submit: `POST https://spansh.co.uk/api/route` with form-encoded fields (the site uses jQuery `$.ajax` POST, `traditional:true`, so arrays repeat the key) returns HTTP 202 `{"job":"<GUID>","status":"queued"}` — [Spansh app JS](https://spansh.co.uk/assets/elite-dangerous-gui-4ce93861bd384bc982212db7e24f463c.js); live test of `POST /api/route` (`from=Sol&to=Colonia&range=50&efficiency=60&supercharge_multiplier=4`)
- Submit also works as `GET` with a query string (HTTP 202 same body): this is how the Observatory plugin helper calls it — [SpanshHelper.cs](https://github.com/fredjk-gh/ObservatoryPlugins/blob/main/PluginCommon/Data/Spansh/SpanshHelper.cs); live test `GET /api/route?efficiency=60&from=Sol&to=Alpha%20Centauri&range=50&supercharge_multiplier=4`
- Poll: `GET /api/results/{job}`. While pending the body has no `result`; when done it returns HTTP 200 with `status:"ok"`, `state:"completed"`, `created_at`, `updated_at`, `parameters`, `result` (my neutron Sol->Colonia job finished in ~4 s, 128 jumps, 22,000 ly). The site's own poller backs off (doubling timeout up to a max) and treats HTTP 404 as "job unknown, re-submit" — [Spansh app JS](https://spansh.co.uk/assets/elite-dangerous-gui-4ce93861bd384bc982212db7e24f463c.js); live test [/api/results/{job}](https://spansh.co.uk/api/results/8E9B73A8-C9C0-11F1-948B-A61E9996B4CE)
- Reference third-party client polls every 1 s, checks `result` first, treats `error`/`status=="failed"` as failure — [SpanshHelper.cs](https://github.com/fredjk-gh/ObservatoryPlugins/blob/main/PluginCommon/Data/Spansh/SpanshHelper.cs)
- Errors: invalid submit -> HTTP 400 `{"error":"from, to and range are required"}`; unknown job -> HTTP 404 `{"error":"Unable to find route"}` — live tests on [/api/route](https://spansh.co.uk/api/route) and [/api/results/0000...](https://spansh.co.uk/api/results/00000000-0000-0000-0000-000000000000)
- Neutron result shape: `result{source_system,destination_system,distance,range,efficiency,via,total_jumps,system_jumps[{system,id64,x,y,z,jumps,distance_jumped,distance_left,neutron_star}]}` — live test (above)

**Plotter endpoints and parameters (all extracted from the site's own client code, i.e. undocumented but in production)**
- Neutron router: `/api/route` — `from`, `to`, `range` (>20 per client validation), `efficiency` (1-100, default 60), `supercharge_multiplier` (4 or 6), `via[]` — [Spansh app JS](https://spansh.co.uk/assets/elite-dangerous-gui-4ce93861bd384bc982212db7e24f463c.js); [NeutronRouteJobResult.cs](https://github.com/fredjk-gh/ObservatoryPlugins/blob/main/PluginCommon/Data/Spansh/CommonGeneric/NeutronRouteJobResult.cs)
- Galaxy/"exact" plotter (fuel, refuel, injections): `/api/generic/route` — `source`/`destination` (id64), `is_supercharged`, `use_supercharge`, `use_injections`, `use_injections_when_required`, `exclude_secondary`, `refuel_every_scoopable`, `fuel_power`, `fuel_multiplier`, `optimal_mass`, `base_mass`, `tank_size`, `internal_tank_size`, `reserve_size`, `max_fuel_per_jump`, `range_boost`, `ship_build`, `max_time`, `cargo`, `algorithm` (options `fuel`,`fuel_jumps`,`guided`,`optimistic`,`pessimistic`; default `optimistic`), `supercharge_multiplier`, `injection_multiplier`. UI limits `max_time` to 120 s (240 s for Patreon users) — [Spansh app JS](https://spansh.co.uk/assets/elite-dangerous-gui-4ce93861bd384bc982212db7e24f463c.js)
- Road to Riches: `/api/riches/route` — `from`, `to` (optional), `radius`, `range`, `max_results`, `max_distance`, `min_value` (UI default 100000), `use_mapping_value`, `avoid_thargoids`, `loop` — same JS
- Ammonia / Earth-like / Rocky+HMC routes reuse `/api/riches/route` with `min_value=1` and `body_types` = `["Ammonia world"]`, `["Earth-like world"]`, `["Rocky body","High metal content world"]` respectively — same JS
- Exobiology: `/api/exobiology/route` — `from`,`to`,`radius`,`range`,`max_results`,`max_distance`,`min_value` (UI default 10,000,000),`avoid_thargoids`,`loop` — same JS
- Fleet carrier: `/api/fleetcarrier/route` — `source`, `destinations[]`, `capacity`, `mass`, `capacity_used`, `calculate_starting_fuel`, `fuel_loaded`, `tritium_stored`, `refuel_destinations[]` — same JS
- Trade: `/api/trade/route` — `system`,`station`,`max_hops`,`max_hop_distance`,`starting_capital`,`max_cargo`,`max_system_distance`,`max_price_age`,`requires_large_pad`,`allow_prohibited`,`allow_planetary`,`allow_player_owned`,`allow_restricted_access`,`unique`,`permit` — same JS
- Tourist: `/api/tourist/route` — `source`,`final_destination`,`destination[]`,`range`,`loop`; saved-search variant `/api/search/tourist/route/{guid}` — same JS
- Colonisation route plotter exists: `/api/colonisation/route` — `source_system`, `destination_system` — same JS
- Engineer route plotter: `/api/engineer/route` (JSON POST) — `{source, blueprints:[{slot,name,grade,effect?}], engineers?}`; helper lists `/api/engineers`, `/api/modules`, `/api/blueprints`, `/api/effects` — same JS
- User-bound variants exist (`/api/user/generic/route`, `/api/user/results/{id}`, `/api/user/routes`, `/api/user/tokens`...) and require a Spansh login (Frontier/Patreon OAuth) — same JS

**Search / lookup APIs**
- Lookups: `GET /api/system/{id64}` (response wrapped in `record`, includes per-body `estimated_scan_value` and `estimated_mapping_value`), `/api/body/{id64}`, `/api/station/{marketId}`, `/api/dump/{id64}`, `/api/nearest?x&y&z`, `/api/search?q=`, `/api/search/systems?q=`, `/api/commodity/buy/{system}/{commodity}/{amount}`, `/api/station/stock/...`, `/api/ship/{id}` — [Spansh app JS](https://spansh.co.uk/assets/elite-dangerous-gui-4ce93861bd384bc982212db7e24f463c.js); live test [/api/system/10477373803](https://spansh.co.uk/api/system/10477373803)
- Search: `POST /api/{systems|bodies|stations}/search` with JSON `{filters, sort, size, page(0-based), reference_system | reference_coords{x,y,z} | reference_route{source,destination}}`; discovery via `GET /api/{systems|bodies|stations}/searchable_fields`, `.../sortable_fields`, `.../field_values/{field}`; `.../search/save` and `.../search/recall/{guid}[/{page}]` — same JS
- Filter wire format (from the client code, validated by my live tests): numeric `{"min":..,"max":..}` for `distance`; groups `{"value":[...]}`; booleans `{"value":true}`; value-structs as list e.g. `"ring_signals":[{"comparison":"<=>","count":[1,99],"name":"Painite"}]` — same JS
- Live test, mining hotspots: bodies search with that `ring_signals` filter around Sol returned 308 hits; first hit "Barnard's Star 5" with per-ring `signals[{name,count}]`, `signals_updated_at`, `reserve_level` — [/api/bodies/search](https://spansh.co.uk/api/bodies/search)
- Live test, services: stations search `{"material_trader":{"value":["Raw"]}}` within 40 ly of Sol returned 19 stations with `market_updated_at` 2026-10-16 — [/api/stations/search](https://spansh.co.uk/api/stations/search)
- Searchable fields include (stations) `material_trader`, `technology_broker`, `services`, `has_large_pad`, `large_pads`, `is_dssa_carrier`, `carrier_docking_access`, `marketplace` (commodity supply/demand/buy/sell), `modules`, `ships`, `system_is_being_colonised`; (systems) `is_colonised`, `is_being_colonised`, `power_state*`, `thargoid_war_*`, `minor_faction_presences`, `landmarks`, `estimated_scan_value`/`estimated_mapping_value`; (bodies) `ring_signals`, `signals`, `genuses`, `landmarks`, `materials`, `atmosphere_composition`, `volcanism_type`, `is_landable`, `reserve_level`, `synthesis_recipes` — live test [/api/stations/searchable_fields](https://spansh.co.uk/api/stations/searchable_fields), [/api/bodies/searchable_fields](https://spansh.co.uk/api/bodies/searchable_fields), [/api/systems/searchable_fields](https://spansh.co.uk/api/systems/searchable_fields)

**Data dumps (HEAD requests 2026-10-17; sizes are compressed .gz bytes)**
- `galaxy.json.gz` 117,842,567,664 B (~118 GB), Last-Modified 2026-10-16 05:47 UTC — [download](https://downloads.spansh.co.uk/galaxy.json.gz)
- `galaxy_populated.json.gz` 4,566,758,037 B — [download](https://downloads.spansh.co.uk/galaxy_populated.json.gz); `galaxy_stations.json.gz` 4,537,087,294 B (modified 2026-10-17 00:18 UTC, i.e. refreshed more often) — [download](https://downloads.spansh.co.uk/galaxy_stations.json.gz)
- `galaxy_1day` 1.14 GB, `galaxy_7days` 3.01 GB, `galaxy_1month` 5.41 GB — [galaxy_1day](https://downloads.spansh.co.uk/galaxy_1day.json.gz)
- `systems.json.gz` 6.37 GB (systems without bodies), `systems_neutron` 181.5 MB, `systems_1day` 2.45 MB, `systems_1week` 19.3 MB, `systems_2weeks` 44.6 MB, `systems_1month` 90.2 MB, `systems_6months` 632 MB — [systems.json.gz](https://downloads.spansh.co.uk/systems.json.gz)
- `factions.json.gz` 17.3 MB — [download](https://downloads.spansh.co.uk/factions.json.gz)
- The directory listing at downloads.spansh.co.uk is 403; the file list comes from the site's `/dumps` route code — [Spansh app JS](https://spansh.co.uk/assets/elite-dangerous-gui-4ce93861bd384bc982212db7e24f463c.js)

**Third-party use in 2026 (evidence programmatic use is normal)**
- EDMC plugin NeutronDancer ("Advanced Neutron (and any Spansh) Jump Plotter", pushed 2026-10-14) — [dwomble/EDMC-NeutronDancer](https://github.com/dwomble/EDMC-NeutronDancer); EDMC-SpanshTools — [wuuthradd/EDMC-SpanshTools](https://github.com/wuuthradd/EDMC-SpanshTools); Elite Observatory plugins with typed Spansh job models — [fredjk-gh/ObservatoryPlugins](https://github.com/fredjk-gh/ObservatoryPlugins); Fuel Rats site uses a Spansh route proxy — [FuelRats/fuelrats.com](https://github.com/FuelRats/fuelrats.com); Canonn Signals credits "CMDR Spansh for use of his APIs and galaxy dumps" — [canonn-signals README](https://github.com/canonn-science/canonn-signals)
- EDMC core plugin `spansh_core.py` only builds links (`/system/{id64}`, `/station/{marketId}`, `/search/{name}`), not API calls — [spansh_core.py](https://github.com/EDCD/EDMarketConnector/blob/main/plugins/spansh_core.py)

### Inferences
- A desktop companion can safely: (a) submit plotter jobs and poll at >=1 s with an identifying User-Agent; (b) call `/api/system/{id64}` and `/api/dump/{id64}` for enrichment (estimated scan/mapping values included); (c) use `/api/bodies|stations|systems/search` for hotspots, trader/broker finding, colonisation candidates. Treat everything except the 4 documented endpoints as unstable and wrap in version-tolerant parsers (unknown keys ignored).
- Since no limits are published, adopt conservative client-side throttling (e.g. serialize jobs, cache results by id64 with the `updated_at` field, one poll/second max) — the author states a modest user base and he pays Hetzner hosting himself.
- For offline/bulk use, `galaxy_populated.json.gz` (4.6 GB) or `galaxy_stations.json.gz` are the only practical dumps for a desktop app; the 118 GB full galaxy is a server-side import job, not a desktop download.
- Exact plotter inputs (`fuel_power`, `fuel_multiplier`, `optimal_mass`, `base_mass`, `tank_size`, `max_fuel_per_jump`...) are the same quantities a Coriolis/EDSY-style build produces; the web client derives them from a pasted `ship_build` JSON (variable naming suggests a Coriolis-format config), so a companion would compute them from the journal Loadout itself. (Inference from client variable names; not verified against server behavior.)

### Gaps
- No ToS, acceptable-use or rate-limit statement found; whether Spansh blocks abusive clients is unknown.
- Exact server-side validation ranges for non-neutron plotters (e.g. min `range` for exact plotter, max `radius`) not verified; only taken from client code.
- Result JSON shapes for riches/exobiology/trade/fleetcarrier/tourist/exact plotter not captured by live test (only neutron). Observatory plugin repo has C# models for fleet carrier and tourist results.
- Job result retention time (how long a GUID stays valid) unknown.

---

## 2. EDAstro (edastro.com): data, maps, dumps, API

### Takeaway
EDAstro is live (205M systems in its DB on 2026-10-17) and offers a small rate-limited read API (`/api/starsystem`, GEC POI JSON endpoints) plus a large catalogue of daily/biweekly CSV and JSONL dumps (POIs, nebulae, regions, codex, fleet carriers, colonisation candidates). It also accepts journal events (EDMC has a built-in EDAstro uploader).

### Cited Findings
- Home page shows live DB counts (205,388,393 systems; 147,431,066 stars; 483,914,130 planets) and notes "EDDiscovery and EDMC support our API" — [edastro.com](https://edastro.com/)
- Data retrieval API: `https://edastro.com/api/starsystem?q={name|id64}` (single object) or comma list up to 10 (array); all read endpoints rate-limited to "100 requests in a 15 minute period" with counters in headers — [API Details](https://edastro.com/api-details.html); live test showed `RateLimit-Limit: 100`, `RateLimit-Remaining: 99`, `RateLimit-Reset: 306` — [api/starsystem?q=Sol](https://edastro.com/api/starsystem?q=Sol)
- Galactic Exploration Catalog (GEC) POI API: `/gec/json/all`, `/gec/json/combined` (GEC+GMP; `id`+`source` needed for uniqueness), `/gec/json/rare`, `/gec/json/single/{POI-ID}`, `/gec/json/id64/{id64}`, `/gec/json/nearest/{x}/{y}/{z}[/{minrating}]`, `/gec/json/favorites/{user}`, `/gec/json/categories`, `/gec/json/stats` — [API Details](https://edastro.com/api-details.html). Live tests: `categories`, `nearest/0/0/0`, `id64/10477373803`, `rare` returned 200 JSON; `stats` returned HTTP 500 at test time — [gec/json/categories](https://edastro.com/gec/json/categories)
- Submission API: EDMC core plugin POSTs `[ {appName, odyssey, system}, event ]` to `https://edastro.com/api/journal`, only for `CarrierStats`, `CarrierJumpRequest`, `ScanOrganic`; response `status` 200 or 401 both mean "OK"; `/api/eddn-upload` for manual upload — [edastro_core.py](https://github.com/EDCD/EDMarketConnector/blob/main/plugins/edastro_core.py); [API submissions info](https://edastro.com/api-info.html)
- EDAstro says the API's wanted event list changes over time and that it targets gaps EDDN misses (fleet carriers, codex, Odyssey organic scans) — [API submissions info](https://edastro.com/api-info.html)
- Data sources: EDDN (real-time), EDSM pull every few days, plus GMP, DSSA, IGAU codex, Inara, Spansh, nebula catalogue, Green Giants lists, Tritium Pathway Network — [Data Sources](https://edastro.com/datasources.html)
- Downloadable files page (sizes/dates as of 2026-10-17) — [Files and Data](https://edastro.com/mapcharts/files.html):
  - POIs: `edsmPOI.csv` (combined EDSM/GMP/DSSA/IGAU POIs, 11,282 rows, 1.3 MB, updated 2026-10-16; columns `POI Type,ID,Name,X,Y,Z,Reference System,Notes`) — [edsmPOI.csv](https://edastro.com/mapcharts/files/edsmPOI.csv)
  - Nebulae: `nebulae-coordinates.csv` (5,843 rows, 2026-10-10) — [file](https://edastro.com/mapcharts/files/nebulae-coordinates.csv)
  - Regions: `regionID.csv` (43 rows, 2025-04-13), `regionID_numsort.csv`, `regionMAP.zip` (10 ly resolution, 81M rows, last built 2020-09-09)
  - Codex: `codex-data.csv` (4,957,646 rows, 822 MB); Fleet carriers: `fleetcarriers.csv` (91,053 rows); Stations: `stations.csv` (445,311 rows, last 2025-04-30); DSSA deployments `DSSAdeployments.csv`
  - Colonisation: `colony-candidates.zip` (548,540 rows, 2026-10-16); Terraformables, Black holes, neutron stars, ELW/AW/WW lists, ring statistics, galactic records, etc.
  - JSONL 7-day dumps (updated every other day): `edastro_systems7days.jsonl.gz` (16 MB gz), `edastro_stars7days.jsonl.gz` (48 MB gz), `edastro_planets7days.jsonl.gz` (202 MB gz)
  - Large biweekly zipped CSV dumps by body class (e.g. `dump-systems.zip` 7.0 GB zipped, `dump-icy-body.zip` 24 GB zipped, `dump-surface-materials.zip` 11 GB zipped)
  - Page warns column order/names may change; scripts should detect headers
- Guardian/Thargoid sites are not separate EDAstro files; closest are GEC POIs (category list includes "Organic", "Mystery and Xenology", "Deep Space Outpost", etc.) and `codex-data.csv` — [gec/json/categories](https://edastro.com/gec/json/categories)
- Canonn Signals consumes GEC APIs, `nebulae-coordinates` and Green Gas Giant datasets from EDAstro with the author's credit — [canonn-signals README](https://github.com/canonn-science/canonn-signals)

### Inferences
- For a desktop companion: pull `edsmPOI.csv`, `nebulae-coordinates.csv`, `regionID.csv` once a day/week (small, static-ish) and use `/gec/json/nearest` or `/gec/json/id64` on demand within the 100 req/15 min budget (honor `RateLimit-Reset`).
- Sending `ScanOrganic`/`CarrierStats`/`CarrierJumpRequest` to `/api/journal` is optional reciprocity; follow EDMC plugin's field filter if implemented.

### Gaps
- No explicit license/terms for downloaded datasets found beyond the footer disclaimer ("not affiliated with Frontier"; data "supplied by players").
- GEC JSON schema (field list) was seen only as sample rows; "typos and all" format note refers to EDSM GMP-JSON compatibility.

---

## 3. Canonn (canonn.science): APIs, codex, bio/geo/Guardian/Thargoid data

### Takeaway
The old Strapi "CAPIv2" at `api.canonn.tech` did not respond from my test, but the Google Cloud Functions "query" API (`us-central1-canonn-api-236217.cloudfunctions.net/query`) and the daily "Dumpr" CSV dumps on Google Cloud Storage are live and return codex reference, Guardian ruin systems, bio statistics, Thargoid data. Canonn asks third-party software to make a "formal request" before using the query API.

### Cited Findings
- Usage rule in the repo README: "If you wish to access these functions ... in your own software we require that you make a formal request to Canonn and that if appropriate you also include code in your application to capture the data from your users"; use `Accept-Encoding: gzip, deflate` — [Canonn-GCloud query README](https://github.com/canonn-science/Canonn-GCloud/blob/main/query/README.md)
- Documented routes: `fleetCarrier/<serial>`, `fleetCarriers/...`, `challenge/next|status|missing|fastest_scans`, `codex/ref`, `nearest/codex`, `settlement/<name|id64>`, `get_gr_data` (Guardian ruins systems), `getSystemPoi`, `thargoid/nhss/systems|reports`, `thargoid/hyperdiction/reports`, `get_compres`, `gnosis`, `gnosis/schedule` — same README
- Live tests (2026-10-17, gzip responses, HTTP 200): `codex/ref` (dict keyed by entryid, fields `english_name`, `category`, `sub_category`, `hud_category`, `platform`, `reward`, `image_url`, and a per-entry `dump` CSV URL), `get_gr_data` (215 Guardian ruins systems with x,y,z), `biostats` (~1.7 MB; per-codex-entry histograms of body type, atmosphere, gravity, distance); `nearest/codex?x=0&y=0&z=0` returned HTTP 500 — [codex/ref](https://us-central1-canonn-api-236217.cloudfunctions.net/query/codex/ref), [get_gr_data](https://us-central1-canonn-api-236217.cloudfunctions.net/query/get_gr_data), [biostats](https://us-central1-canonn-api-236217.cloudfunctions.net/query/biostats)
- Dumpr generates CSV extracts of codex data daily to a CORS-enabled GCS bucket, explicitly "may also be used by third party developers"; per-entry files e.g. `https://storage.googleapis.com/canonn-downloads/dumpr/Biology/2101400.csv`, `.../hyperdictions.csv`; columns System Name, X, Y, Z, entryid, coordRegion, boxelRegion; notes that CodexEntry region names derive from boxel coordinates not x,y,z — [Dumpr README](https://github.com/canonn-science/Dumpr)
- Canonn's Strapi API hostnames: `https://api.canonn.tech` (EDMC-Canonn `servers.json`), docs at docs.canonn.tech ("CAPIv2 - Documentation") — [servers.json](https://raw.githubusercontent.com/canonn-science/EDMC-Canonn/master/data/servers.json), [docs.canonn.tech](https://docs.canonn.tech/). In my tests `api.canonn.tech` failed to connect (HTTP 000) while docs.canonn.tech, canonn.science, signals.canonn.tech, bioforge.canonn.tech returned 200. CAPIv2 repos last pushed 2021-11 (CAPIv2-Strapi/Docs) — [Canonn org repos](https://github.com/orgs/canonn-science/repositories)
- Active Canonn repos in Oct 2026: `canonn-signals` (2026-10-11, MIT; Angular "Canonn Signals" using Spansh API, GEC, Canonn), `bioforge` (2026-10-16, BSD-3), `canonn-colony-operations` (2026-10-15, MIT, BGS tracker for Canonn faction network), `Canonn-GCloud` (2026-10-11, GPL-3.0), `EDMC-Canonn` (2026-10-06, GPL-3.0), `GR-RuinsComboMap` (2026-10-07) — [Canonn org](https://github.com/canonn-science)
- Canonn Signals features: signals by category (Human, Other, Geology, Biology, Thargoid, Guardian), biological signal prediction, mining analysis, region map (klightspeed), data from Spansh, GEC, Canonn — [canonn-signals README](https://github.com/canonn-science/canonn-signals)
- EDMC-Canonn POI API lets third parties feed patrol POIs via TSV/JSON with placeholders ({CMDR},{LAT},{LON},{BODY}) — [POI_API.md](https://github.com/canonn-science/EDMC-Canonn/blob/master/POI_API.md)

### Inferences
- Safest usage for a closed-source/personal companion: consume the public GCS Dumpr CSVs and `codex/ref` (cache for days), avoid hammering query endpoints, and contact Canonn (Discord #software) if shipping a distributed app that calls the query API.
- `get_gr_data` + `biostats` give cheap Guardian ruin system lists and bio/geo spawn statistics for prediction.

### Gaps
- Exact current status/replacement of `api.canonn.tech` unknown (connect failure from my network could be local); no 2026 announcement found.
- Per-endpoint rate limits not documented. Some routes (`nearest/codex`) failed with 500 at test time.

---

## 4. Elite BGS (elitebgs.app) and BGS tick detection

### Takeaway
EliteBGS v5 has a clean REST API (systems/factions/stations/ticks) but in Oct 2026 it is unreliable: repeated multi-day outages with `ECONNREFUSED 127.0.0.1:27017`, an infra-limited sole maintainer, and no code release since Nov 2024. For tick detection use the EDCD tick detector (`tick.edcd.io`) rather than EliteBGS.

### Cited Findings
- API v5 route files: `systems`, `factions`, `stations`, `ticks`. Query params — systems: `id,eddbId,name,allegiance,government,state,primaryEconomy,secondaryEconomy,faction,factionId,factionControl,security,activeState,pendingState,recoveringState,influenceGT,influenceLT,referenceSystem,referenceSystemId,referenceDistance,referenceDistanceMin,sphere,beginsWith,minimal,factionDetails,factionHistory,timeMin,timeMax,...`; factions: `name,allegiance,government,system,systemId,activeState,pendingState,recoveringState,influenceGT/LT,minimal,systemDetails,timeMin,timeMax,count,page`; ticks: `timeMin,timeMax` — [systems.js](https://github.com/elite-kode/elitebgs/blob/master/backend/routes/elite_bgs_api/v5/systems.js), [factions.js](https://github.com/elite-kode/elitebgs/blob/master/backend/routes/elite_bgs_api/v5/factions.js), [tick_times.js](https://github.com/elite-kode/elitebgs/blob/master/backend/routes/elite_bgs_api/v5/tick_times.js)
- Live test 2026-10-17: `https://elitebgs.app/api/ebgs/v5/systems?name=Sol`, `/factions`, `/ticks` returned HTTP 500 `{"message":"connect ECONNREFUSED 127.0.0.1:27017","error":{}}` (MongoDB down); later retries timed out; `/api/ebgs/docs` and `/api/ebgs/v4/ticks` returned 404 — [elitebgs.app/api/ebgs/v5/ticks](https://elitebgs.app/api/ebgs/v5/ticks)
- GitHub issue #378 "DB connection error" (open, updated 2026-10-12): maintainer: "The issue is not with the code itself but I am struggling with some issues on the infra" (2026-03-08); outages reported 2026-04-27, 2026-07-27, 2026-10-09 ("both homepage and API return immediate 500 from nginx"), back 2026-10-10 for only hours, "up now" 2026-10-12; also notes errors can come back with HTTP 200 — [issue 378](https://github.com/elite-kode/elitebgs/issues/378)
- Earlier tick problems: issues #376/#377 (Feb 2025) — EliteBGS tick lagged a day behind EDCD's tick detector — [issue 376](https://github.com/elite-kode/elitebgs/issues/376), [issue 377](https://github.com/elite-kode/elitebgs/issues/377)
- Repo `elite-kode/elitebgs` (Apache-2.0): last commit 2024-11-14 ("add GitHub sponsor link"); successor `elite-kode/elitebgs-next` last push 2025-06-29 — [elitebgs](https://github.com/elite-kode/elitebgs), [elitebgs-next](https://github.com/elite-kode/elitebgs-next)
- Maintained tick source: EDCD `TickDetector` (pushed 2026-08-02); Python version "recommended - currently maintained", Node.js version "no longer maintained"; HTTP API `GET /api/tick` (latest tick as JSON string, ISO 8601 UTC) and `GET /api/ticks?start=yyyy-mm-dd&end=yyyy-mm-dd`; Socket.IO WebSocket pushes `message`/`tick` events on new tick; listens to EDDN `FSDJump/Location/CarrierJump` — [TickDetector README](https://github.com/EDCD/TickDetector)
- Live test: `https://tick.edcd.io/api/tick` -> `"2026-10-16T16:31:14+00:00"`, `Access-Control-Allow-Origin: *`; `/api/ticks` returns history from 2018-11-04 — [tick.edcd.io/api/tick](https://tick.edcd.io/api/tick)
- "Zoy" tick detector: `http://tick.infomancer.uk/galtick.json` returned `{"lastGalaxyTick":"2026-10-15T16:14:56.000Z"}` over HTTP; HTTPS connection timed out from my network. At the same moment EDCD reported 2026-10-16T16:31:14Z, i.e. Zoy's feed was ~1 day behind — live test [galtick.json](http://tick.infomancer.uk/galtick.json)
- Other BGS data: Spansh systems search exposes `minor_faction_presences` (name, influence, state, government, allegiance) and `controlling_minor_faction*` — [Spansh systems searchable_fields](https://spansh.co.uk/api/systems/searchable_fields); Spansh dump `factions.json.gz` — [download](https://downloads.spansh.co.uk/factions.json.gz); BGS-Tally (EDMC plugin) tracks BGS/colonisation/PP/TW activity per faction — [Raven Colonial About view](https://raw.githubusercontent.com/njthomson/RavenColonialWeb/main/src/views/About.tsx)

### Inferences
- Do not make EliteBGS a hard dependency. Preferred stack: tick time from `tick.edcd.io` (HTTP poll or Socket.IO) + own BGS state from journal/EDDN + Spansh systems/factions data; treat EliteBGS as optional history source with timeout/backoff and cache.
- Zoy's feed vs EDCD disagreement shows tick detection is heuristic; a companion should display "tick time (source)" and tolerate +/- hours.

### Gaps
- No official statement about EliteBGS' long-term future; only the GitHub issue thread.
- Zoy's detector source/maintainer status not verified (only one live reading).

---

## 5. Raven Colonial (ravencolonial.com) and other colonisation tools

### Takeaway
Raven Colonial is an active (changelog entry 2026-09-01) colonisation planner/tracker by CMDR Grinning2001 (author of SrvSurvey) with a GPL-3.0 open-source React front-end. Its REST backend (Azure) has no public documentation; integration is through SrvSurvey or EDMC plugins that call the same API with a per-commander API key.

### Cited Findings
- Front-end repo `njthomson/RavenColonialWeb` (GPL-3.0, pushed 2026-09-11); `lastEntry = 2026-09-01` in ChangeLogView; credits CMDR Grinning2001 — [RavenColonialWeb](https://github.com/njthomson/RavenColonialWeb), [ChangeLogView.tsx](https://raw.githubusercontent.com/njthomson/RavenColonialWeb/main/src/views/ChangeLogView.tsx), [About.tsx](https://raw.githubusercontent.com/njthomson/RavenColonialWeb/main/src/views/About.tsx)
- Features (About view): system planning/architect view (imports bodies/ports from Spansh; compares predicted vs real economies), project creation/tracking, assigning commodities to commanders, linking Fleet Carriers, finding markets, "nexus" chains; page auto-refreshes every 30 s — [About.tsx](https://raw.githubusercontent.com/njthomson/RavenColonialWeb/main/src/views/About.tsx)
- Client apps: SrvSurvey (original), EDMC plugins incl. BGS-Tally, a "current" dedicated plugin (author @Fenris159) and a legacy one; warns to disable colonisation features in the client app if using an EDMC plugin alongside — [About.tsx](https://raw.githubusercontent.com/njthomson/RavenColonialWeb/main/src/views/About.tsx)
- API base used by web and SrvSurvey: `https://ravencolonial100-awcbdvabgze4c5cq.canadacentral-01.azurewebsites.net`; authenticated calls send headers `rcc-cmdr0` (base64 CMDR name) and `rcc-key` (API key); login flow uses Frontier OAuth (`auth.frontierstore.net`) then `POST /api/login/` returns `{key, cmdr}` — [api-util.ts](https://raw.githubusercontent.com/njthomson/RavenColonialWeb/main/src/api/api-util.ts), [auth.ts](https://raw.githubusercontent.com/njthomson/RavenColonialWeb/main/src/api/auth.ts), [SrvSurvey RavenColonial.cs](https://raw.githubusercontent.com/njthomson/SrvSurvey/main/SrvSurvey/net/RavenColonial.cs)
- Endpoints seen in client code: `PUT /api/project/` (create), `POST /api/project/from/{id64}/{siteId}/{type}`, `GET|PATCH|DELETE /api/project/{buildId}`, `POST /api/project/{buildId}/supply/{cmdr}` (deliver cargo), `.../stats`, `.../fc/{marketId}`, `POST /api/project/markets` (find markets), `GET /api/system/{name}`, `GET /api/v2/system/{nameOrId64}` (+ `/bodies`, `/sites`, `/spanshEconomies`, `/snapshot`), `GET /api/cmdr/{cmdr}/active`, `/api/fc/{marketId}` (+ `/cargo`, `/spansh`), `/api/chain/{id}` (nexus), `/api/stats/`, `/api/ggg/json` — [project.ts](https://raw.githubusercontent.com/njthomson/RavenColonialWeb/main/src/api/project.ts), [v2-system.ts](https://raw.githubusercontent.com/njthomson/RavenColonialWeb/main/src/api/v2-system.ts), [cmdr.ts](https://raw.githubusercontent.com/njthomson/RavenColonialWeb/main/src/api/cmdr.ts), [fc.ts](https://raw.githubusercontent.com/njthomson/RavenColonialWeb/main/src/api/fc.ts)
- It pulls from EDSM (`api-system-v1/stations|bodies|factions`) and Ardent Insight (`https://api.ardent-insight.com/v2/search/system/name/...`) — [edsm.ts](https://raw.githubusercontent.com/njthomson/RavenColonialWeb/main/src/api/edsm.ts), [ardent.ts](https://raw.githubusercontent.com/njthomson/RavenColonialWeb/main/src/api/ardent.ts)
- Third-party EDMC integrations on GitHub: `toemaus313/ravencolonial_edmc` (pushed 2025-11-07; API key required only for Fleet Carrier tracking; "Stealth Mode"), `BBBBAface/EDMC-Raven-Colonial-Plugin` (2026-05-27), `mcjohnso/EDRavenColonialAgent` (2026-06-21), `pequalsnp/ed-colonization-reporter` (Linux-first desktop client, 2026-07-31) — [ravencolonial_edmc README](https://github.com/toemaus313/ravencolonial_edmc), [search listing](https://github.com/search?q=ravencolonial&type=repositories)
- Static colonisation data shipped with SrvSurvey: `colonization-costs2.json` (per-build-type commodity totals, e.g. Coriolis Starport tier 2: steel 14076, CMM composite 11261, aluminium 10055...), last commit 2026-03-20 (also copied in the web repo `src/assets/colonization-costs2.json`) — [colonization-costs2.json](https://raw.githubusercontent.com/njthomson/SrvSurvey/main/SrvSurvey/colonization-costs2.json)
- Other colonisation tooling available through APIs/data: Spansh colonisation route plotter (`/api/colonisation/route`) and search fields `is_colonised`/`is_being_colonised`; EDAstro `colony-candidates.zip` (systems without government/economy within range of a colonisation-capable station) — [Spansh app JS](https://spansh.co.uk/assets/elite-dangerous-gui-4ce93861bd384bc982212db7e24f463c.js), [EDAstro files](https://edastro.com/mapcharts/files.html); EDMC core `common_coreutils.py` recognises colonisation ship station names (`$EXT_PANEL_ColonisationShip;`) — [common_coreutils.py](https://raw.githubusercontent.com/EDCD/EDMarketConnector/main/plugins/common_coreutils.py)

### Inferences
- Because the REST API is undocumented, tied to an Azure host name and an API key tied to a Frontier login, a companion should integrate by (a) reading journal ColonisationConstructionDepot / market events itself, and (b) optionally offering "push to Raven Colonial" through the same endpoints as SrvSurvey — only after contacting the author (no stated third-party policy found). The static `colonization-costs2.json` is the lowest-risk reuse (GPL-3.0 caveat for redistribution).
- Do not run a second client alongside SrvSurvey/EDMC plugins for the same commander (double reporting; the About page warns about conflicts).

### Gaps
- No public API documentation, rate-limit or ToS for Raven Colonial found; ravencolonial.com serves an SPA shell for all paths (`/api`, `/swagger` just return the SPA), so no Swagger UI confirmed.
- Whether the Azure host name is stable long term is unknown.

---

## 6. edtools.cc, mining hotspot sources

### Takeaway
edtools.cc is a small server-rendered PHP tool site by CMDR VicTic (miner's tool, hotspot finder, RES/massacre PvE finder, Road to Riches, rares); it has no API and is not suited to programmatic use. Spansh body search (and Spansh/EDAstro dumps) is the right hotspot data source.

### Cited Findings
- Tool list: Rares, Distances, Sphere, List, Road to Riches, Massacre (PvE pirate-massacre mission source finder), Miner's Tool, Hotspots ("known hotspots ... around populated areas (Bubble, Pleiades, Colonia)"), Mining Missions, Multiple prices; RES search "no longer maintained", Selenium tool obsolete in Odyssey — [edtools.cc](https://edtools.cc/)
- Versions/dates in page footers: Miner's Tool v1.5 (2025-10-11), Hotspot finder v0.6 (2025-10-08; "Pristine systems only"; minerals incl. Alexandrite, Benitoite, Bromellite, Grandidierite, LTD, Monazite, Musgravite, Opal, Painite, Platinum, Rhodplumsite, Serendibite, Tritium), PvE v0.06 (2026-02-04) — [miner](https://edtools.cc/miner), [hotspot](https://edtools.cc/hotspot), [pve](https://edtools.cc/pve)
- Forms are plain GET/HTML (inputs `s,m,ms,pop` for hotspot; `s,md,lo,sc` for PvE); `/api`, `/about` return 404; no JSON endpoints or terms page found — [edtools.cc/api](https://edtools.cc/api)
- Spansh body search supports ring hotspot queries natively (`ring_signals` value-struct; results include per-ring `signals[{name,count}]`, ring `type`, `signals_updated_at`, body `reserve_level`) — live test [/api/bodies/search](https://spansh.co.uk/api/bodies/search)
- EDAstro offers ring statistics (`rings-statistics.csv`) and large ring/belt dumps (`dump-rings.zip`, `dump-belts.zip`) — [EDAstro files](https://edastro.com/mapcharts/files.html)
- Canonn Signals includes mining resource analysis (gemstones, metals, minerals, fuel) from system data — [canonn-signals README](https://github.com/canonn-science/canonn-signals)
- Hotspot ground truth for the current system comes from the journal (`SAASignalsFound` on rings, FSS `SAASignalsFound`/`Scan` ring data); Spansh `signals_updated_at` shows when community data was last refreshed (no citation needed beyond Spansh test above).

### Inferences
- Use Spansh `/api/bodies/search` with `ring_signals` + `distance` + `reference_system` for "nearest hotspot of X", cross-check with `reserve_level`; use Spansh `/api/stations/search` with `marketplace` demand filters (field exists) for selling stations. edtools.cc should only be linked-to in the UI (open in browser), not scraped.

### Gaps
- Could not determine edtools.cc's data source/update cadence (miner page contains a legacy `eddb.io` reference); no contact/terms found.

---

## 7. Ship building: Coriolis and EDSY (status, formats, converting a Loadout event)

### Takeaway
Both are live and current (Coriolis 4.1.13 built 2026-09-26; EDSY data files updated 2026-09-23), both support the 2025-26 ships and the July 2026 Operations update. A journal `Loadout` event is turned into a link by gzip-compressing its canonical JSON and urlsafe-base64 encoding it — EDMC ships the reference implementation.

### Cited Findings
- Algorithm (EDMC `shipyard_url_common`): `json.dumps(loadout, ensure_ascii=False, sort_keys=True, separators=(',',':'))` -> UTF-8 -> gzip -> `base64.urlsafe_b64encode` -> replace `=` with `%3D` — [common_coreutils.py](https://raw.githubusercontent.com/EDCD/EDMarketConnector/main/plugins/common_coreutils.py)
- Coriolis URL: `https://coriolis.io/import?data=<encoded>` (beta: `https://beta.coriolis.io/import?data=`) — [coriolis.py](https://raw.githubusercontent.com/EDCD/EDMarketConnector/main/plugins/coriolis.py)
- EDSY URL: `https://edsy.org/#/I=<encoded>` (beta: `https://edsy.org/beta/#/I=`) — [edsy.py](https://raw.githubusercontent.com/EDCD/EDMarketConnector/main/plugins/edsy.py)
- EDMC builds the input via `monitor.ship()`: a dict with `event:"Loadout"`, `Ship`, `ShipID`, optional `ShipName`/`ShipIdent`, and `Modules` sorted hardpoints/standard/internal (optional `timestamp`) — i.e. the journal Loadout event structure — [monitor.py](https://raw.githubusercontent.com/EDCD/EDMarketConnector/main/monitor.py)
- Coriolis: "Coriolis EDCD Edition", `CORIOLIS_VERSION 4.1.13`, date 2026-09-26; changelog 4.1.0 "Operations update, new modules, new blueprints", 4.1.5 "Operations Update live", 4.1.13 "Show Merc Coin Costs including engineering costs"; 4.0.10 "Fixed slot indexing for Lynx Highliner" — [coriolis.io](https://coriolis.io/), [main bundle](https://coriolis.io/main.3b0f63bd.js)
- Coriolis bundle contains `lynx_highliner` (edID 129041442, identical to FDevIDs shipyard.csv `MediumTransport01`), plus Kestrel, Caspian (`explorer_nx`), Type-11, Corsair, Panther Clipper — [main bundle](https://coriolis.io/main.3b0f63bd.js), [shipyard.csv](https://raw.githubusercontent.com/EDCD/FDevIDs/master/shipyard.csv)
- `EDCD/coriolis-data` `master` ships folder lacks `lynx_highliner.json`, `develop` has it — i.e. `develop` is the live branch; repo branches: develop, master, feature/3-3, special-keys — [master ships](https://github.com/EDCD/coriolis-data/tree/master/ships), [develop ships](https://github.com/EDCD/coriolis-data/tree/develop/ships)
- coriolis-data structure: `ships/*.json`, `modules/{hardpoints,internal,standard}`, `modifications/`, `spec/`, generator scripts; README says hosted compiled data is at `coriolis.io/data/` (my fetch of that path returned the SPA HTML); license: data is Frontier IP, Coriolis code MIT (GitHub shows NOASSERTION); issues go to the coriolis repo — [coriolis-data README](https://raw.githubusercontent.com/EDCD/coriolis-data/master/README.md), [LICENSE.md](https://raw.githubusercontent.com/EDCD/coriolis-data/master/LICENSE.md)
- Coriolis repos pushed 2026-09-26 (coriolis-data, coriolis); maintained by "Brighter-Applications" contributors under EDCD — [commits](https://github.com/EDCD/coriolis-data/commits/master)
- EDSY (edsy.org, by Taleden): live, `edsy.js` and `eddb.js` Last-Modified 2026-09-23; contain Lynx Highliner, Kestrel Mk II, Caspian Explorer, Type-11 Prospector, Cobra Mk V, Python Mk II entries; loads `pako.min.js` for gzip import; `EDCD/EDSY` repo is archived (last push 2018-11-14) so no current public source was found — [edsy.org](https://edsy.org/), [edsy.js](https://edsy.org/edsy.js), [EDCD org](https://github.com/EDCD)
- Observed data discrepancy: coriolis-data `kestrel.json` lists `edID: 129041381`, while FDevIDs shipyard.csv lists Kestrel Mk II (`SmallCombat01_NX`) as 129039241 — [kestrel.json](https://github.com/EDCD/coriolis-data/blob/master/ships/kestrel.json), [shipyard.csv](https://raw.githubusercontent.com/EDCD/FDevIDs/master/shipyard.csv)

### Inferences
- Implement link generation locally (no network call): copy EDMC's function; Coriolis accepts the Loadout JSON directly, EDSY same encoding. Strip nothing but keep key order canonical (sort_keys) to get stable URLs.
- Because the Operations/Rhino-era updates appear in FDevIDs within days and in Coriolis within weeks, a companion should handle unknown module names gracefully (show raw `Item`) rather than failing.

### Gaps
- Whether EDMC strips `timestamp` before building the link (`shipyard_url(loadout,...)` caller) was not verified.
- EDSY's own import-format documentation and license not found.
- Kestrel edID mismatch significance not investigated.

---

## 8. Static reference datasets (EDCD/FDevIDs, coriolis-data, engineers, materials, sites, exobiology, exploration value)

### Takeaway
EDCD/FDevIDs is the authoritative ID table (commodity/outfitting/shipyard updated within days of each 2026 game update) but several of its tables are stale (materials 2021, engineers 2021, rare commodities 2024-12). Coriolis-data covers modules/blueprints/ships (develop branch). Guardian, bio-criteria and colonisation datasets live in SrvSurvey (GPL-3.0); codex/bio statistics in Canonn; POIs/nebulae/regions in EDAstro. Exobiology payout values, clonal colony distances and the exploration value formula were not verified (see Gaps).

### Cited Findings
**EDCD/FDevIDs** — [repo](https://github.com/EDCD/FDevIDs) (pushed 2026-09-05; no license shown; description "Collection of IDs returned from the Companion API"; README: collections "incomplete - most notably some PowerPlay-specific items are missing from outfitting.csv", player-specific items omitted, names are in-game English)
- Files: `commodity.csv` (13.9 KB), `outfitting.csv` (112 KB), `shipyard.csv`, `rare_commodity.csv`, `material.csv`, `microresources.csv`, `engineers.csv`, `bundles.csv`, `sku.csv`, `economy.csv`, `government.csv`, `factionstate.csv`, `factionids.csv`, `systemallegiance.csv`, `security.csv`, `happiness.csv`, `crimes.csv`, `passengers.csv`, `rings.csv`, `terraformingstate.csv`, rank tables (`combatrank`, `TradeRank`, `ExplorationRank`, `CQCRank`, `EmpireRank`, `FederationRank`), `dockingdeniedreasons.csv`, plus a "Frontier API" folder and "How to determine MatTrader and Broker type" — [contents](https://github.com/EDCD/FDevIDs)
- Last-commit dates per file (via GitHub API, 2026-10-17): commodity 2026-09-02 ("new commodities and modules with Rhino SRV Update"), outfitting 2026-09-02, sku 2026-09-02, shipyard 2026-04-30 ("new ship with modules: Lynx Highliner"), government 2025-11-11, rare_commodity 2024-12-20, bundles 2024-05-12, factionids 2024-02-11, crimes 2024-01-22, engineers 2021-10-28 ("four new engineers in the Colonia area"), microresources 2021-07-18, material 2021-01-13, economy 2022-08-09 — [commits](https://github.com/EDCD/FDevIDs/commits/master)
- Recent commits: 2026-09-05, 2026-09-02, 2026-08-24, 2026-07-13 ("new system allegiance with Operations update"), 2026-07-03 ("new commodity and modules for Operations & Nomad update"), 2026-05-13, 2026-04-30, 2026-03-24, 2026-02-25 (Kestrel Mk II), 2025-12-02 (Caspian), 2025-09-29 (Type-11 Prospector) — [commit log](https://github.com/EDCD/FDevIDs/commits/master)
- shipyard.csv columns: `id,symbol,name,sku`; newest rows: Kestrel Mk II `SmallCombat01_NX` 129039241, Lynx Highliner `MediumTransport01` 129041442, Caspian Explorer `Explorer_NX` 129036569 — [shipyard.csv](https://raw.githubusercontent.com/EDCD/FDevIDs/master/shipyard.csv)

**Coriolis data** — [EDCD/coriolis-data](https://github.com/EDCD/coriolis-data): ships (47 JSON files on develop incl. `lynx_highliner`, `kestrel`, `explorer_nx`, `python_nx`, `type_11_prospector`), modules (hardpoints/internal/standard), `modifications/` (engineering blueprints/specials); package.json `3.0.0`, `license: MIT`; `ChangeLog.md` only goes to 2.5.1 (stale); recent commits 2026-02 to 2026-04 on master, repo pushed 2026-09-26 — [package.json](https://raw.githubusercontent.com/EDCD/coriolis-data/master/package.json), [ChangeLog.md](https://raw.githubusercontent.com/EDCD/coriolis-data/master/ChangeLog.md)
- Spansh's own engineering plotter embeds blueprint data (per grade `components`, `features`, `uuid`, `fdname`, module names) in a ~6 MB bundle chunk and serves `/api/blueprints`, `/api/modules`, `/api/effects`, `/api/engineers` — [chunk.574](https://spansh.co.uk/assets/chunk.574.f6c0cf7568815769c6d4.js)

**EDEngineer (blueprints/entry data)** — `msarilar/EDEngineer`: MIT, **archived**, last push 2024-03-22; data files `EDEngineer/Resources/Data/{blueprints,entryData,equipment,localization,releaseNotes}.json`; last data commits 2024-03-07 ("Sirius AX Weapon Update") — [repo](https://github.com/msarilar/EDEngineer)

**Odyssey Materials Helper** — `jixxed/ed-odyssey-materials-helper`: MIT, pushed 2026-10-12, site edomh.nl; its blueprint/engineer/material definitions are in Java source and `locale/*.csv` (e.g. `blueprint/odyssey/names.csv`, `broker/broker.csv`), not a standalone JSON dataset; test resources contain sample journal/EDDN payloads — [repo](https://github.com/jixxed/ed-odyssey-materials-helper)

**Guardian / Thargoid / settlement datasets** — SrvSurvey (GPL-3.0, pushed 2026-09-05): `allRuins.json`, `allStructures.json`, `allBeacons.json`, `guardianSiteTemplates.json` (last 2026-08-16), `data/guardian/*.json` (~729 per-site files), `settlementTemplates.json`, `settlements/humanSiteTemplates.json`, `ggg.json`, `Boxel.Names.txt` — [SrvSurvey](https://github.com/njthomson/SrvSurvey); Canonn `get_gr_data` (215 ruin systems) and `thargoid/nhss/*` routes — [Canonn query README](https://github.com/canonn-science/Canonn-GCloud/blob/main/query/README.md); `canonn-science/GR-RuinsComboMap` (GPL-3.0, 2026-10-07) — [repo](https://github.com/canonn-science/GR-RuinsComboMap)

**Exobiology species spawn criteria and statistics** — SrvSurvey `bio-criteria/<genus>.json` (21 genus files; per species/variant query rules on body type, gravity, temperature, pressure, atmosphere, volcanism, star class, with "hit count" annotations; last commit 2026-07-31) — [aleoida.json](https://raw.githubusercontent.com/njthomson/SrvSurvey/main/SrvSurvey/bio-criteria/aleoida.json); Canonn `biostats` histograms (live) — [biostats](https://us-central1-canonn-api-236217.cloudfunctions.net/query/biostats); Canonn `codex/ref` — [codex/ref](https://us-central1-canonn-api-236217.cloudfunctions.net/query/codex/ref); EDMC-BioScan (GPL-2.0, pushed 2026-09-23, "species deduction engine") — [Silarn/EDMC-BioScan](https://github.com/Silarn/EDMC-BioScan)

**Exploration value** — Spansh already returns `estimated_scan_value` and `estimated_mapping_value` per body in `/api/system/{id64}` and exposes them as search/sort fields — [/api/system/10477373803](https://spansh.co.uk/api/system/10477373803), [bodies sortable_fields](https://spansh.co.uk/api/bodies/sortable_fields). Candidate open-source implementations of the formula (not inspected): EDDiscovery (Apache-2.0, pushed 2026-10-16) — [repo](https://github.com/EDDiscovery/EDDiscovery); Elite Observatory core (MIT, 2026-10-01) — [repo](https://github.com/Xjph/ObservatoryCore)

**Regions / boxel naming** — `klightspeed/EliteDangerousRegionMap` (MIT, 2026-05-12) used by Canonn and EDAstro-style tools — [repo](https://github.com/klightspeed/EliteDangerousRegionMap); EDAstro `regionID.csv`/`regionMAP.zip` — [files](https://edastro.com/mapcharts/files.html)

**Material trader / tech broker / interstellar factors locations** — no static list needed: Spansh stations search exposes `material_trader` (Raw/Encoded/Manufactured), `technology_broker` and `services` filters (verified for material_trader) — [stations searchable_fields](https://spansh.co.uk/api/stations/searchable_fields); FDevIDs documents how to determine MatTrader/Broker type — [FDevIDs](https://github.com/EDCD/FDevIDs)

**Other EDCD infrastructure** — EDDN (BSD-3-Clause, pushed 2026-07-12), EDMarketConnector (GPL-2.0, 2026-10-01) with core plugins `coriolis`, `edsy`, `edastro_core`, `spansh_core`, `eddn`, `edsm`, `inara`; `EDCD/EDMC-Plugin-Registry` (2026-10-16) — [EDCD org](https://github.com/EDCD)

### Inferences
- Build the companion's lookup layer from FDevIDs CSVs (update monthly or on game-update days) + Coriolis `develop` JSON for module stats/blueprints; bundle snapshot files and refresh via raw.githubusercontent.com.
- Stale tables (material, engineers, rare commodities) need a manual overlay or fallback to journal-provided `*_Localised` names; journal gives localised names for most events, so IDs are mostly needed for CAPI/market/outfitting joins.
- For exobiology value/clonal distance, prefer extracting constants from BioScan/Observatory/SrvSurvey source (check licenses: GPL-2.0/GPL-3.0 are copyleft; MIT sources safer) or use Spansh/Canonn data, rather than hard-coding unverified numbers.

### Gaps
- Exobiology per-species credit values and clonal colony minimum distances: I did not locate/verify a citable dataset (EDMC-BioScan data path not found within budget; no GitHub code-search hits).
- Body exploration value formula source: not inspected; only candidate repos listed.
- Engineer locations/unlock requirements as a structured current dataset: not located (FDevIDs `engineers.csv` is IDs only and stale since 2021-10; EDEngineer archived; EDOMH data embedded in Java). Spansh `/api/engineers` content not fetched.
- Materials: FDevIDs `material.csv` stale (2021-01); no verified current alternative dataset.
- Odyssey suits/weapons engineering data (EDOMH Java enums/CSV) not extracted.
- Licenses: FDevIDs shows no license; coriolis-data data files are Frontier IP per LICENSE.md; redistribution terms should be checked before bundling.
- ICARUS Terminal, EDDiscovery/EDDI static data not investigated.
