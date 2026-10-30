# TMC — The Multipurpose Companion

<p align="center">
  <a href="README.md"><strong>English</strong></a> ·
  <a href="README.es.md"><strong>Español</strong></a> ·
  <a href="README.fr.md"><strong>Français</strong></a> ·
  <a href="README.de.md"><strong>Deutsch</strong></a> ·
  <a href="README.pt.md"><strong>Português</strong></a>
</p>

[![Version](https://img.shields.io/badge/version-0.1.0-orange.svg)](https://github.com/Danikuso/TheMultipurposeCompanion/releases)
[![License: PolyForm Noncommercial 1.0.0](https://img.shields.io/badge/license-PolyForm%20Noncommercial%201.0.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11%20%7C%20Linux%20%2F%20Steam%20Deck-green.svg)]()
[![Inara Whitelist](https://img.shields.io/badge/Inara%20InAPI%20v1-Official%20Whitelist-success.svg)]()

**TMC (*The Multipurpose Companion*)** is a free, open-source, real-time cockpit companion for ***Elite Dangerous*** (*Live 4.0 / Odyssey / Horizons* and *Legacy 3.8*), built with **Rust**, **Tauri 2**, and **Svelte 5**.

Its mission is to serve as an all-in-one copilot for both novice pilots and veteran commanders: bringing together **10 combined cockpit layouts**, **16 specialized modules**, **route plotting**, **step-by-step engineer unlock guides**, **audio copilot and voice queries**, and an **auxiliary touch screen for tablet or mobile** in a single ultra-lightweight application.

---

## Table of Contents

- **Player Resources:** [Commander's User Manual & Tutorials (PDF)](website/Manual_TMC_Comandante.pdf) · [Overview Website (`website/`)](website/index.html)
- **[Part I — Player and User Guide](#part-i--player-and-user-guide)**
  1. [Philosophy and Key Qualities](#1-philosophy-and-key-qualities)
  2. [Installation and Getting Started (Windows and Linux / Steam Deck)](#2-installation-and-getting-started)
  3. [How the Interface Works (`TopBar`, `Auto-companion`, and the 10 Layouts)](#3-how-the-interface-works)
  4. [All Features: The 16 Modules in Detail](#4-all-features-the-16-modules-in-detail)
  5. [Voice Copilot, Microphone Queries, VoiceAttack, and Tablet / Mobile (QR)](#5-voice-copilot-microphone-queries-voiceattack-and-tablet--mobile)
  6. [Settings Customization and Support / Feedback](#6-settings-customization-and-support--feedback)
- **[Part II — Developer Technical Documentation](#part-ii--developer-technical-documentation)**
  7. [Workspace Architecture (12 Crates + 2 Apps + Frontend)](#7-workspace-architecture)
  8. [Journal Ingestion, Storage, Security, and Performance](#8-journal-ingestion-storage-security-and-performance)
  9. [Local Paths, CLI Arguments, and Environment Variables](#9-local-paths-cli-arguments-and-environment-variables)
  10. [Development Tools, Replay, and Scientific Audit](#10-development-tools-replay-and-scientific-audit)
  11. [Compilation, Automated Tests, and NSIS Packaging](#11-compilation-automated-tests-and-nsis-packaging)
  12. [Contributing, Privacy, and Licensing](#12-contributing-privacy-and-licensing)

---

# Part I — Player and User Guide

## 1. Philosophy and Key Qualities

- **A companion that never leaves you stranded:** if you want to engineer a ship module but haven't yet discovered or unlocked the required engineer, TMC won't show you a blank warning: it first displays **step-by-step instructions on how to discover them, obtain their invite, and what workshop donation or task they require** (reading your real journal progress), and right below calculates the necessary materials and barter exchanges.
- **Visual clarity («TMC Glass HUD») and ship-synchronized colors:** every panel displays essential information at the top using gauges, status lights, and one-click actions, keeping large tables tucked away under **`View additional information`**. Furthermore, the **`Auto HUD`** theme automatically detects your game cockpit's custom color matrix to mirror your exact ship palette.
- **Lightweight and quiet in the background:** engineered not to impact game framerates or PC resources. When minimizing or closing the window (`✕`), TMC continues running in the system tray, freeing its WebView2 memory while keeping audio callouts and auto-copying of hyperspace jumps active.
- **100% respectful of your privacy and the Legacy galaxy:** works locally from the very first second with zero mandatory accounts. If it detects you are playing in the **Legacy 3.8** galaxy, it automatically blocks all transmissions to external community networks (EDDN, EDSM, Inara) to prevent contaminating live databases.
- **5 fully supported languages:** user interface, engineering data, and voice speech available in **English**, **Spanish**, **French**, **German**, and **Portuguese**.

---

## 2. Installation and Getting Started

### Windows 10 / 11 (64-bit)

1. Download the installer **`TMC_0.1.0_x64-setup.exe`** from the [Releases](https://github.com/Danikuso/TheMultipurposeCompanion/releases) section (installer wizard available in all 5 languages).
2. Run the installer and follow the on-screen steps.

> [!NOTE]
> **Windows Defender SmartScreen Notice:**  
> As a free, non-commercial open-source project without an expensive corporate signing certificate, Windows might show the blue *«Windows protected your PC»* dialog upon first run. Click on **«More info»** and then **«Run anyway»**.

3. Launch **TMC** before or after starting *Elite Dangerous*. It will automatically locate your saved games folder, import your history in the background (`Importing history: X of Y`), and restore its window size and position between sessions.
4. **System Tray:** clicking `✕` on the window minimizes TMC to the system tray next to the Windows clock. Left-click the tray icon (or right-click → **`Open TMC`**) to restore the window, or select **`Quit`** to exit completely.

### Linux and SteamOS (Steam Deck)

- TMC automatically detects the *Elite Dangerous* journal directory inside Steam **Proton** prefixes (both native and Flatpak installations, AppID `359320`).
- You can run the desktop GUI application (`tmc-desktop`) or start the lightweight headless server (`tmc-headless`) to view the interface from any browser or tablet on the same local Wi-Fi.

---

## 3. How the Interface Works

### 3.1 Top Bar (`TopBar`)
Always docked at the top of the screen:
- **Status and Galaxy:** live WebSocket connection indicator, **`LIVE 4.0`** or **`LEGACY`** badge, and contextual alerts (`Game not running` or `Legacy Galaxy: data remains strictly local`).
- **6 Instant Flight Metrics:** current **`System`**, **`MODE`** (across 10 flight/surface states: *Docked, Landed, Normal Space, Supercruise, Hyperspace, On Foot, In SRV, In Fighter, In Taxi, Multicrew*), **`Fuel`** (with amber warning below 25%), **`Hull`** (with red alarm below 30%), **`Cargo`** (`used / capacity`), and **`Credits`**.
- **Quick Controls on the Right:** tactical lock button **`🔓` / `🔒`** (with **`Alt + L`** shortcut to instantly freeze the screen), voice microphone button **`🎙️`**, and **`Auto-companion`** switch.

### 3.2 Auto-Companion Pro: Predictive Intelligence, Tactical Lock, and Priority Management
- **Intelligent, Continuous Context Switching:** with **`Auto-companion` enabled**, TMC smoothly transitions to the most relevant cockpit layout according to in-game actions (combat, asteroid prospecting, station docking, engineering, Fleet Carrier jump, colonization, or hyperspace navigation).
- **1-Click Tactical Lock (`Alt + L` / Lock Button):** during intense combat, route auditing, or recipe inspection, click the padlock button in the top bar or press **`Alt + L`** to freeze the current cockpit. The lock glows amber in *Glass HUD* style, preventing any automatic event from switching your view until you unlock it.
- **Hysteresis & Anti-Jerk Retention (`linger_seconds`):** when finishing a dogfight, mining an asteroid, or completing a trade transaction, the screen won't abruptly jump away. It retains the active cockpit for the configured duration (default 20 seconds, adjustable from 0 to 60s in Settings) so you can review loot, damage, and rewards peacefully.
- **Predictive In-Flight Transitions:** TMC anticipates events before they occur:
  - *Imminent Interdiction:* whether being interdicted or initiating an interdiction in supercruise, the cockpit switches to *Combat* immediately during the escape vector struggle.
  - *Ring Approach:* descending toward planetary rings or asteroid belts with mining gear equipped switches proactively to *Mining*.
  - *Docking Clearance Granted:* receiving docking permission at a starport or outpost pre-switches to *Trade*, preparing the local commodity market and station services.
- **Custom Cockpit Priority Order:** in `Settings`, you can customize the hierarchy order of all 9 roles using simple `▲` and `▼` buttons, or click `Reset to standard order` to restore the recommended tactical hierarchy.
- **Cockpit Audio Feedback:** enable the subtle *Glass HUD Chime* (synthesized natively via Web Audio API at 880 Hz ➔ 1320 Hz without heavy audio assets) or TTS copilot voice callouts to know when the screen switches without taking your eyes off your flight instruments.
- Within the **`Exploration`** layout, the **`Context Panel`** automatically flips between **`System Bodies`** and **`Exobiology`** when you touch down on a world with biological signals.
- If you manually click another layout while `Auto-companion` is active (without the lock engaged), TMC respects your temporary manual choice until in-game activity changes. Turning `Auto-companion` off fixes the screen wherever you leave it.

### 3.3 The 10 Combined Layouts vs. The 16 Modules
The left sidebar offers two complementary navigation modes (in addition to the pinned **`⚙️ Settings`** button):

| Combined Layout | Panels Grouped in a Single Screen |
|---|---|
| **1. `Exploration`** | `Route Strip`, `Context Panel` (`Bodies` / `Exobiology` toggle), `Route Planner`, and `Jump Range`. |
| **2. `Ship`** | `Ship` (outfitting and engineering), `Jump Range`, `Cargo Inventory`, and `Shipyard & Outfitting Search`. |
| **3. `Combat`** | `Target Info`, `Hunting Session`, `My Ship`, and automatic `Conflict Zone` and `Anti-Xeno (AX)` panels when active. |
| **4. `Mining`** | `Mining Target Finder` (triangular loop and best sell), `Prospector`, `Mining Session`, and `Where to Sell`. |
| **5. `Trade`** | `Current Market`, `Cargo Inventory`, `Trade Session`, `Trade Route Optimizer`, and `Find Commodity`. |
| **6. `Missions`** | `Active Missions`, `Notices`, `Session Rewards`, and automatic `Massacre Stack`, `Passengers`, `Tourism`, and `Community Goals` panels. |
| **7. `Engineering`** | `Materials Inventory`, `Engineers`, `Engineering Notices`, `Modified Ship`, `Engineer a Module`, `Shopping List`, `Barter Exchanges`, and `Where to Find`. |
| **8. `Fleet Carrier`** | `Carrier Status`, `Jump Sequence Countdown`, and `Finances & Upkeep`. |
| **9. `Colonization`** | `Construction Depot` and `Supply & Procurement`. |
| **10. `Powerplay 2.0`** | `Powerplay Status` and `System Factions` (Background Simulation - BGS). |

Below the layouts, you will find the list of **16 Individual Modules** (with a glowing activity dot when they hold active data) to open any tool in dedicated full-width view.

---

## 4. All Features: The 16 Modules in Detail

### 1. `Dashboard` (7 panels)
- **`Commander`:** name, credit balance, and progress bars with percentage for all 6 *Pilots Federation* ranks (*Combat, Trade, Exploration, Mercenary, Exobiologist, CQC*) and 2 *Superpowers* (*Federation, Empire*).
- **`Ship & Location`:** summary of active ship, star system, nearest celestial body, station, and current flight mode.
- **`Live Status`:** 7 instant cockpit flags (*Shields, Hardpoints, Landing Gear, Mass Lock, Silent Running, Cargo Scoop, Supercruise Overcharge SCO*) alongside power distributor pips for `SYS`, `ENG`, and `WEP` (0 to 4 pips).
- **`Route & Session`:** next system with **`Copy next system`** button, remaining jumps, session duration, completed jumps, light-years traveled, and total credits earned.
- **`Alerts`:** immediate notifications for *Low Fuel, Overheating, Interdiction in Progress, Under Attack, Hull Damage, Low Oxygen, Low Health*, or module malfunction.

### 2. `Ship` (2 panels: Technical Audit and Shipyard/Outfitting Finder)
- **Header & Meters (`Ship`):** canonical model, ship name, ship ID, unladen mass, rebuy cost, and gauges for hull integrity, shields, main + reservoir fuel tanks, and cargo.
- **3 Pre-Flight Checklist Tabs:**
  - **`🧭 Exploration`:** maximum jump range and verification checklist (`✅` / `⚪`) for Fuel Scoop, Guardian FSD Booster, Detailed Surface Scanner (DSS), SRV Hangar, and Auto Field-Maintenance Unit (AFMU).
  - **`⚔️ Combat`:** shield strength, thermal load, `SYS/ENG/WEP` distribution, hardpoints with ammunition `[clip / reserve]`, and defensive utility count (*Shield Boosters, Chaff Launchers, Point Defences, Heat Sinks*).
  - **`📦 Cargo & Mining`:** total capacity, installed cargo racks, refinery, limpet controllers, and onboard cargo.
- **`Module Breakdown`:** filterable by category (*All, Hardpoints, Core, Optional, Utility*) and search query. Displays class & rating `[5A]`, health `%`, power state `ON/OFF`, power priority `P1..P5`, and golden badge `G1..G5 ★` which opens blueprint recipe details, engineer, experimental effect, and an **exact attribute comparison (engineered value vs. stock value)**.
- **`Shipyard & Outfitting Search` (One-Stop Shop Finder):**
  - **Combined Basket from Current Location:** starts automatically from your current star system (even in unindexed deep space via 3D coordinates) and allows selecting **1 optional ship** (48 hulls) and/or **up to 10 outfitting modules** (153 families with category, class `0..8`, rating `A..I`, weapon mount *Fixed/Gimballed/Turret*, and armor hull type) to find the nearest stations that sell the **entire basket in a single stop**.
  - **Smart Fallback per Item:** if no single station sells all items at once, it automatically locates the closest station for each missing item individually, identifying any piece lacking known stock.
  - **Pilot Safeguards and Toggles:** **`−15% Li Yong-Rui discount`** toggle (which also permanently lowers your ship rebuy cost), dedicated **`Your Power's Stronghold Carriers`** switch to include friendly orbital carrier stations from Powerplay 2.0 (excluding rival power carriers and private carriers unless explicitly enabled), permit-locked system protection (`Shinrarta Dezhra`, `Sol`, `Achenar`, `Sirius`, `Alioth`…), **Rebuy Safety Traffic Light** (`Rebuy OK` / `Adjust Rebuy` / `Insufficient Balance`), itemized price breakdown, and one-click **`Copy`** and **`Route`** buttons.

### 3. `Navigation` (3 panels and dual Spansh router)
- **`Route Strip`:** displays your active Spansh route plan or the in-game galaxy map route. If you go off course, a **`Off Route`** alert appears with **`Recalculate`** and **`Clear Plan`** buttons. Details each jump across 4 columns: navigation (distance, star class, scoopable status **`⛽ Scoopable`**, estimated remaining fuel), exploration (discovered/undiscovered, prior visits, neutron/white dwarf), politics (allegiance, government, faction, and **`⚠ Danger State`** in war or outbreak), and economy/security/Powerplay.
- **`Jump Range`:** current jump range (with **`Supercharged ×N`** badge from neutron stars or synthesis), full-tank jump range, current fuel level, and maximum fuel per jump.
- **`Route Planner`:** system autocomplete, automatic 3D coordinate origin (even from remote systems not yet in public databases), and 2 calculation engines:
  - **`⚡ Neutron Highway`:** rapid neutron star hopping with configurable **`Efficiency (%)`**.
  - **`🗺️ Exact Route (Exact Plotter)`:** automatically reads unladen mass, fuel tanks, cargo load, engineered FSD, and Guardian Booster directly from your active ship without uploading data to third-party servers, offering **`Scoopable stars only`** and **`Use FSD injections`** options. Clicking **`Accept`** stores the route and auto-copies each subsequent waypoint to your clipboard jump after jump.

### 4. `Exploration` (`System Bodies`)
- **Top Summary:** system discovered/undiscovered status, all star classes present (highlighting scoopable *O, B, A, F, G, K, M* in green), **`FSS Discovery Progress`**, comparative value cards for **`System Value`** vs. **`Value with Recommended Maps`**, unmapped high-value bodies, and **`Unsold Data`** card with a red **`Data at Risk`** alert if your hull has taken damage.
- **Detailed Expandable Table:** 27 star and planet classifications, distance in light-seconds, scan and map values, `Bio` and `Geo` signals, flags (*Terraformable, Landable, Scoopable, First Discovery, Previously Mapped*), and **`Recommendation`** column (`★ Map` based on your credit threshold, `Land (Exobiology)`, or `Skip`, alongside the DSS probe efficiency target).

### 5. `Exobiology` (Odyssey Surface Sampling Assistant)
- **World Header:** surface gravity, surface temperature, atmospheric composition, biological signal count, and **`FIRST LOGGED · ×5 (estimated)`** badge when you are the first commander to log specimens on that planet.
- **`Live Sampling Tracker` (`1/3 → 2/3 → 3/3`):** progress bar and live counter for **`Distance to Previous Sample`** compared against the species' required clonal separation distance (`X m remaining`), **relative directional guidance and cardinal heading** based on where you are facing (e.g. *«previous sample is behind to your left (SW)»*), warning if you attempt to discard a partial sample, and **`Unsold Onboard Value`** card projecting total earnings upon completing the genus.
- **`Potential Species Table (Sorted by Value)`:** combines astrophysical criteria, personal flight logs, and *Canonn* telemetry to show base reward, `With ×5 Bonus` payout, minimum clonal distance, scan probability, and confirmed DSS surface signals.

### 6. `Combat` (5 tactical panels and Anti-Xeno)
- **`Target`:** selected ship with faction emblem, pilot name, combat rank, faction allegiance, high-visibility **`CLEAN · DO NOT FIRE`** (green) or **`WANTED · bounty`** (red) banner, shield and hull health bars, and percentage health of the targeted **`Subsystem`** (*Power Plant, Thrusters, FSD, Weapons…*), retaining data for 10 seconds if target lock is broken.
- **`My Ship`:** shield status, 10-block hull integrity bar, thermal level with countdown timer upon overheating, visual power distributor pip meters (`SYS`, `ENG`, `WEP`), and active cockpit warnings.
- **`Hunting Session`:** kills, credits earned, hourly rate (`cr/h`), combat rank progress, **`Reset Session`** button, breakdown of **`Unclaimed Bounties`** by faction (including net calculation with **`−25% at Interstellar Factors`**), and nearest *Interstellar Factors* location.
- **`Conflict Zone & Anti-Xeno (AX)` (Automatic):** selected war faction, kills, and combat bonds; Thargoid presence alert, AX bonds, and kill counter by variant (*Scout, Cyclops, Basilisk, Medusa, Hydra, Scythe, Glaive, Orthrus…*).

### 7. `Mining` (4 panels: Target Finder, Prospector, Session, and Where to Sell)
- **`Mining Target Finder & Optimal Sale (Miner's Companion)`:**
  - **Shortest Triangular Loop:** calculates the complete optimal circuit right in your cockpit: from your current system ➔ to the best pristine ring or belt ➔ to the closest station offering the highest sale price for your minerals.
  - **3 Specialized Mining Profiles & Instant Preset Chips:**
    - **Laser:** focused on Platinum in metallic rings, Painite, and Osmium. Includes combinations with *Hazardous RES* (+100% fragments per rock) and *High RES* (+75% fragments) extraction sites, plus verified double/triple overlaps.
    - **Core:** focused on Musgravite, Monazite, Alexandrite, Void Opals, and Grandidierite in rocky and icy rings.
    - **Missions & Engineers:** Bromellite (for unlocking engineer Bill Turner in Alioth), Tritium (for fueling *Fleet Carriers*), and Osmium.
  - **Demand Tax Risk Warning:** *Elite Dangerous* market mechanics drastically decrease payouts if your cargo exceeds 25% of a station's total commodity demand. TMC gives you an advance traffic light warning with the exact safe tonnage limit to avoid lost profits.
  - **1-Click Multi-Sell Optimizer:** reads the exact minerals and metals in your ship's hold and finds the optimal station that buys the whole mix for maximum aggregate profit.
  - **Offline Curated Catalog & Atomic Online Sync:** ships with over 40 community-verified high-density hotspots (*EDTools/Spansh*), working immediately offline and updating seamlessly in the background when an internet connection is available.
  - **Cockpit Actions:** **`📋 Copy System`** and **`🚀 Plot Route`** buttons on every target card and within the adaptive table with sticky horizontal scrolling.
- **`Prospector`:** mineral content level (*Low/Medium/High*), prominent **`◆ Core: Mineral`** banner for deep-core detonation, multi-color composition bar, mineral list highlighting favorites with `★`, automatic **`✔ Worth Mining`** or **`✘ Skip`** verdict based on your credit threshold, and a comparative graph of the **`Last 8 Asteroids`**.
- **`Mining Session`:** session duration, refined tonnage and `t/h` rate, estimated revenue and `cr/h`, remaining and expended **`Limpets`** by type, cargo hold fill percentage, breakdown per mineral, and real-time hopper status of the **`Refinery`**.
- **`Where to Sell`:** top recommended station for your entire cargo hold, plus expandable breakdowns per mineral showing station name, system (with copy button `⎘`), distance, market price, demand, pad size (`S/M/L` with warning if unconfirmed), and price age.

### 8. `Trade` (5 panels)
- **`Current Market`:** highlights top trading opportunities over the last 48 hours and displays the full station catalog, flagging **`RARE`** commodities (with optimal 160 ly sale distance), buy prices, supply, best nearby buyer, pad size, and profit per ton.
- **`My Cargo`:** hold utilization and lot-by-lot tracking of **`Average Price Paid`**, mission cargo or stolen tags, comparison between selling at current dock vs. the best nearby market, and direct **`SELL HERE`** or **`HAUL`** guidance.
- **`Trade Route`:** multi-stop route optimizer originating from your current dock, detailing what to buy and sell at each stop, jump count, total profit, and automatic clipboard copying of each destination.
- **`Trade Session & Find Commodity`:** net profit balance, `cr/h`, tonnage hauled, best trade run, trade rank progression, and autocomplete search to locate where to buy any commodity near your location.

### 9. `Missions` (7 panels: Contracts, Massacres, Passengers, Tourism, and CGs)
- **`Active Missions & Notices`:** contracts grouped by destination indicating `▶ Next Stop` with **`Add to Route`** button, mission type, faction, progress, expiration countdown (turns red if `< 1 h`), and alerts for route deviation or missing mission cargo in hold.
- **`Massacre Stack`:** groups pirate hunting contracts against the same target faction into parallel bars by issuing faction (where each kill counts simultaneously across all distinct factions), calculating remaining kills needed to complete the stack and total payout.
- **`Passengers`:** cabin occupancy by class (*Economy, Business, First, Luxury*), **`VIP`** / **`WANTED`** passenger tags, and critical red warning **`⚠ Wanted passenger onboard: scan risk in station security zones`**.
- **`Tourism`:** automatically reorders tourist beacons into the shortest **`Optimal TSP Route`** versus the mission board order (showing saved light-years and jumps), calculates `cr/ly` and `cr/h` against historical averages, and issues a **`✔ PROFITABLE`** or **`✖ NOT WORTH IT`** verdict.
- **`Session Rewards & Community Goals`:** credit earnings, engineering materials awarded, and faction reputation/influence shifts, plus live tracking of active *Community Goals* (*contribution, tier, percentile rank, time remaining, and projected reward*).

### 10. `Engineering` (8 panels for ship modules and Odyssey suits/weapons)
- **`Materials Inventory`:** 4 tabs (*Raw, Manufactured, Encoded*, and Odyssey *Locker* split into *Items, Components, Consumables, and Data*), instant search, **`Session History`** tracking provenance (*Mission, Scan, Mining, Trade, Engineering…*), and **`CAPACITY MAX`** warnings on full bins.
- **`Engineers & My Modified Ship`:** filterable catalog (*All, Unlocked, Pending*) of **ship and Odyssey ground engineers** with rank `★`, distance, and next unlock requirement progress; fast selector for any engineerable module installed on your active ship.
- **`Engineer a Module`:** select blueprint modification, experimental effect, grade `G1..G5`, and rolls per grade. If you don't yet have access to an engineer capable of crafting that grade, it first reveals the **`COMPANION GUIDE — ENGINEER UNLOCK`** with 3 sequential steps (*1. Discovery → 2. Invitation → 3. Workshop Unlock Donation*), system with quick-copy, and current progress, followed below by exact material costs and status lights (`OK`, **`Covered via Trade`**, or `Missing`).
- **`Shopping List, Material Exchanges, Where to Find, and Notices`:** manage multiple engineering projects, prioritize them (**`▲ Up` / `▼ Down`**) to allocate materials, compute exact barter conversions using unallocated surpluses (`Give X → Receive Y`), and locate the closest **Material Trader** or **Technology Broker** (*Human* and *Guardian*) along with gathering guides.

### 11. `Fleet Carrier` (3 panels)
- **`Carrier Status`:** callsign, carrier name, `ONBOARD` badge, current system, remaining jumps (500 ly/jump), docking access permissions, **`Tritium`** fuel level combining depot (`1,000 t`) + cargo reserve, and breakdown of free vs. allocated cargo capacity.
- **`Jump Sequence`:** operational status and live `MM:SS` timer for the 15-minute jump countdown and 5-minute post-jump cooldown, displaying destination system and **`Copy Destination`** button.
- **`Finances & Upkeep`:** financial health indicator (*Healthy, Warning, Critical*), **`Weeks Covered`**, carrier balance, weekly upkeep, maintenance reserve, and date of next weekly deduction.

### 12. `Colonization` (2 panels)
- **`Construction Depot`:** overall depot completion percentage, delivered vs. outstanding tonnage, dynamic **`Estimated Hauling Trips`** based on your active ship's cargo capacity, and an itemized resource-by-resource table.
- **`Supply & Procurement`:** outstanding material cards ordered by urgency with required tonnage, remaining trips, and one-click **`Copy`** button for rapid market purchasing.

### 13. `Powerplay 2.0` and 14. `BGS`
- **`Powerplay Status`:** power and superpower crests, loyalty weeks, current rank (`X / 100`), accumulated and remaining merits to next rank, session merits, **`Merit Rate per Hour`**, and countdown to weekly server tick (*Thursday 07:00 UTC*).
- **`System Factions (BGS)`:** controlling faction, active states, daily **`Galactic Tick Tracker`** (`X hours ago`, showing whether data is fresh or pending tick update), and a full local faction table with influence bars, states, allegiance, and government.

### 15. `Crime & Notoriety` (5 tactical cards)
- **`Legal Status & Notoriety`:** notoriety meter from 0 to 10 with cooldown decay timer (2 active flight hours per point) that detects and displays **`⏸️ Decay Paused (docked at station)`** while in port.
- **`Clean Slate Guide`:** warns if neutral contacts are locked due to active notoriety or, when notoriety is 0, displays the **`Nearest Interstellar Factors`** with distance, pad size, and **`Copy System`** button.
- **`Contraband & Black Markets`:** lists illegal goods in hold, locates the **`Nearest Black Market`**, and includes a 3-step **`Silent Running Infiltration Guide`** (8–10 km alignment, silent running / heat sink at `< 4 km`, and fast mail slot entry).
- **`Faction Fines & System Security`:** fines and bounties itemized by jurisdiction + local security level with estimated police response times.

### 16. `Accounts` (Optional Community Sync)
- **`Elite Dangerous Data Network (EDDN)`:** share market prices, outfitting, shipyard, and star jump telemetry anonymously with the community network, and receive local market updates within your configured light-year radius.
- **`Elite Dangerous Star Map (EDSM)`:** securely save your API key to sync flight logs, commander ranks, and credits.
- **`Inara`:** officially approved on the **Inara Whitelist** under the application name **`The Multipurpose Companion (TMC)`**, featuring smart batching (max 2 API requests per 60s) and a live connection test button.
- **`Frontier Companion API (CAPI)`:** secure OAuth2 login through Frontier's official authorization portal (with manual URL fallback if your browser prevents automatic callback) and interactive **`Stored Remote Fleet Viewer`** showing all ships parked in other systems, their station, value, and distance in light-years.

---

## 5. Voice Copilot, Microphone Queries, VoiceAttack, and Tablet / Mobile

### 5.1 Audio Copilot (Spoken Warnings / TTS)
In `Settings` → **Voice Copilot & Cockpit Alerts**, choose between:
- **`Operating System Native (Ultra-lightweight)`:** uses your installed OS voices with zero additional downloads.
- **`KokoroTTS Neural Engine (Cinematic AI Voice)`:** 10 natural offline neural voices across multiple languages, featuring automatic standby hibernation when idle to conserve RAM (with seamless fallback to the OS voice if unavailable).
- Includes **volume** and **speech rate** sliders, **`🔊 Test Voice Copilot`** button, and **5 independent channel toggles** (*Combat & Critical Alerts*, *Navigation & Fuel Scooping*, *Exploration & Exobiology*, *Mining & Prospecting*, and *Fleet Carrier*).

### 5.2 Microphone Voice Assistant (9 Spoken Queries in 5 Languages)
In `Settings` → **Voice Assistant & Integration**, enable speech recognition via **`Whisper AI Local`** (~39 MB, 100% offline and private, downloadable with one click from the panel) or the system engine, operating in **Continuous Listening** or **Push-to-Talk** (top bar `🎙️` button). Ask natural language questions for:
1. **Ship status** (hull, shields, and fuel).
2. **Fuel level** (percentage and tons).
3. **Next navigation jump** (announces next system).
4. **Copy next system** (announces and **copies system to clipboard** ready to paste into the galaxy map).
5. **Biological signals in system** (count of bio signals detected).
6. **Pinned materials summary** (progress on your active engineering shopping list).
7. **Legal status & fines** (notoriety, active fines, and bounties).
8. **Nearest Interstellar Factors** (announces station, system, distance, and **copies system to clipboard**).
9. **Nearest Black Market** (announces station, system, distance, and **copies system to clipboard**).

### 5.3 `VoiceAttack` Integration
TMC automatically detects if *VoiceAttack* is running on your machine, generates an updated **`TMC-Copiloto.vap`** profile pre-configured with all commands, and provides a **`📥 Download Profile (.vap)`** button in Settings ready for 1-click import.

### 5.4 Auxiliary Touch Screen on Tablet or Mobile (`QR Code`)
If you play with a single monitor or in VR, open `Settings`, toggle **`Local Network Access`**, and scan the on-screen **`Pairing QR Code`** with your phone or tablet camera connected to the same home Wi-Fi. The complete interactive cockpit opens in your mobile browser with zero app installation required; tapping «Copy System» on the tablet instantly places the destination into your PC clipboard.

---

## 6. Settings Customization and Support / Feedback

- **Language and Themes:** select among **5 languages** (`es`, `en`, `fr`, `de`, `pt`) and **3 visual themes** (**`Auto HUD`** synchronized with your in-game cockpit colors, **`Classic Orange`**, and **`High Contrast`**).
- **Auto-Companion Pro:** reorder cockpit priority for all 9 roles (`▲` / `▼`) with a reset button, hysteresis slider (0–60s with live counter), predictive transition switch, and sensory cues (*Glass HUD Chime* sound and TTS voice).
- **Operational Preferences:** auto-copy next hyperspace jump, master `Auto-companion` switch, local network LAN access, global privacy toggle **`Query online services (Spansh, EDSM, Canonn)`** (disable for 100% air-gapped offline use), **`Detailed Surface Map Threshold (cr)`**, and **`Default Route Planner Efficiency (%)`**.
- **Economy, Tourism, and Engineering:** customizable list of up to 30 **`Minerals of Interest`** (11 pre-selected), **`Mining Value Threshold (cr)`**, **`Trade Sell Search Radius (ly)`**, **`EDDN Market Data Radius (ly)`**, **`Tourism Benchmark (cr/ly)`**, and **`Engineering Rolls per Grade (G1–G5)`**.
- **Direct Support & Community Channel:** in `Settings` → **Support, Feedback & Community**, click **`🚀 Report issue or suggestion on GitHub`** to open a pre-filled ticket on [GitHub Issues](https://github.com/Danikuso/TheMultipurposeCompanion/issues) with app version, language, and OS pre-populated (with no private player data included).

---

# Part II — Developer Technical Documentation

## 7. Workspace Architecture

The project is structured as a modular workspace in **stable Rust (2024 edition)** with strictly decoupled responsibilities and zero circular dependencies, paired with a modern frontend in **Svelte 5 + TypeScript + Vite** and **Fluent** localization catalogs:

| Crate / Directory | Technical Responsibility |
|---|---|
| `crates/tmc-model` | Strongly-typed domain models, journal events, volatile game state, WebSocket subscription topics, strict diagnostic events (`app.toggle_lock`, `app.set_auto_priorities`), and settings schemas (`auto_companion_priorities`, `auto_companion_linger_seconds`, etc.). |
| `crates/tmc-journal` | Cross-platform directory discovery (Win32 `FOLDERID_SavedGames`, Proton/Flatpak on Linux), non-intrusive incremental *byte offset* tailing of `Journal.*.log`, and retry polling for `Status.json` + 9 companion `.json` files. |
| `crates/tmc-store` | Persistence in **SQLite (WAL mode)** via `rusqlite`, additive migrations, historical exploration/trade/shopping records, local credential vault, and self-healing database recovery. |
| `crates/tmc-i18n` | Loading and resolution of **Project Fluent** catalogs (`locales/{es,en,fr,de,pt}/*.ftl`) with automated tests enforcing 100% key parity against `es`. |
| `crates/tmc-net` | Asynchronous HTTP clients with rate limiters, caching, and offline fallback: *Spansh* (Neutron Plotter, Exact Plotter `/api/generic/route`, shipyard/outfitting search `POST /api/stations/search`, body/ring search `/api/bodies/search`, and nearest search `/api/nearest`), *Ardent*, *Canonn*, *tick.edcd.io*, *EDSM*, *Inara InAPI v1* (max 2 req/min batches), and *Frontier CAPI* (OAuth2 S256 PKCE). |
| `crates/tmc-eddn` | HTTP POST client submitting official schemas to *EDDN* (`journal/1`, `commodity/3`, `outfitting/2`, `shipyard/2`) with salted local pseudonyms, plus a ZeroMQ listener filtering local commodity markets by radius. |
| `crates/tmc-tts` | Prioritized speech queue with deduplication and cooldowns (`TtsQueue`), Windows native engine (SAPI5 / OneCore `< 2 MB` RAM), and decoupled neural worker process (`Kokoro-82M` ONNX with 60-second idle hibernation). |
| `crates/tmc-modules` | Domain logic for all 16 game modules (including `outfitting_catalog.json` in `ship` and curated `mining_hotspots.json` with in-memory atomic hot-swapping in `mining`), context relevance calculation with hysteresis (`linger_seconds`), predictive flight triggers, and UI projection. |
| `crates/tmc-core` | Central state actor, event routing bus, tactical `VoiceAdvisor`, and multilingual intent parser `VoiceIntentResolver` (9 voice intents). |
| `crates/tmc-server` | HTTP/REST and WebSocket (`/ws`) server built on **axum**, 100ms topic coalescing (max 10 updates/s), `.vap` profile generator for *VoiceAttack*, DNS-rebinding/CSRF protections, and 128-bit LAN token authentication (`HttpOnly; SameSite=Strict`). |
| `crates/tmc-app` | Service composition, application lifecycle, historical journal import with adaptive throttling, and supervisor loop. |
| `crates/tmc-replay` | CLI utility for time-accelerated journal playback (`play`) and deterministic PII anonymization (`anonymize`) for creating test fixtures. |
| `apps/desktop` | Native **Tauri 2** application (`TMC.exe`), single-instance mutex, window state persistence, system tray lifecycle, and WebView2 memory suspension. |
| `apps/headless` | Standalone CLI server without GUI and mathematical audit runner (`tmc-headless eval`). |
| `ui/` | Reactive UI in **Svelte 5** (runes), TypeScript, *TMC Glass HUD* design system, tactical screen lock (`Alt + L`), dynamic priority reorderer, Web Audio API synthesizer, dynamic virtual lists, and offline in-browser speech recognition in WebAssembly (`Whisper` via `@huggingface/transformers`). |
| `locales/` | Synchronized `.ftl` translation catalogs for `es` (reference), `en`, `fr`, `de`, and `pt`. |

---

## 8. Journal Ingestion, Storage, Security, and Performance

- **Non-Intrusive Byte-Offset Reading:** operating system file system notifications are only used to detect the creation of a new `Journal.*.log` file; line-by-line reading advances through byte offsets without locking file descriptors on `EliteDangerous64.exe`. Volatile files (`Status.json`, `Cargo.json`, `Market.json`, `Outfitting.json`, `Shipyard.json`, `ModulesInfo.json`, `NavRoute.json`, `Backpack.json`, `ShipLocker.json`, `FCMaterials.json`) apply up to 10 retries of 50ms to handle partial writes safely.
- **Resilience and Self-Healing:** malformed JSON lines or unknown future journal events never crash the application. If `tmc.db` suffers corruption on startup, it is automatically backed up as `tmc.db.corrupt-YYYYMMDDHHMMSS`, a clean schema is instantiated, and history is re-imported. If `config.toml` fails validation, it is preserved in `config.toml.bak` and safe defaults are restored.
- **Local Server Security:** binds by default exclusively to loopback `127.0.0.1:47320` (probing `47320..=47340` if occupied), validating `Host` and `Origin` headers. When `lan = true` (`0.0.0.0`), all non-loopback requests strictly require the random 128-bit session token provided in the QR code.
- **Strict Resource Budget (Hard Requirement):** `< 150 MB` total RAM with window open (~35–112 MB typical), `< 1%` idle CPU, zero continuous `requestAnimationFrame` loops, and maximum 10 UI updates/second over WebSocket. When minimized to the tray on Windows, invokes `COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW` and `SetIsVisible(false)`.

---

## 9. Local Paths, CLI Arguments, and Environment Variables

### Default File Paths
- **Game Journals:**
  - Windows: `%USERPROFILE%\Saved Games\Frontier Developments\Elite Dangerous\` (and custom HUD colors in `%LOCALAPPDATA%\Frontier Developments\Elite Dangerous\Options\Graphics\GraphicsConfigurationOverride.xml`).
  - Linux / SteamOS: Proton prefixes for AppID `359320` in `~/.steam/steam/...`, `~/.local/share/Steam/...` or `~/.var/app/com.valvesoftware.Steam/...`.
- **Configuration:** `%APPDATA%\TMC\config.toml` (Windows) / `~/.config/tmc/config.toml` (Linux).
- **SQLite Database and Rolling Logs (7-day max retention):** `%LOCALAPPDATA%\TMC\tmc.db` and `tmc*.log` (Windows) / `~/.local/share/tmc/tmc.db` (Linux).

### Command Line Arguments (`tmc-desktop` and `tmc-headless`)
- `--journal-dir <path>`: override the journal directory.
- `--config <path>`: use a custom `config.toml` file.
- `--ui-dir <path>`: serve precompiled web UI assets from a custom directory.

### Optional Environment Variables
- `TMC_CAPI_CLIENT_ID`: OAuth2 client ID registered on Frontier's developer portal to override default CAPI credentials (never commit secrets to git; use local `.env`).
- `VOICEATTACK_PATH` / `VOICEATTACK_DIR`: custom installation directory for *VoiceAttack*.

---

## 10. Development Tools, Replay, and Scientific Audit

```powershell
# 1. Run headless server + core without GUI (ideal for remote Linux or frontend dev)
cargo run -p tmc-headless -- --journal-dir tests/fixtures/journal

# 2. Replay a test journal in accelerated time (10x) to a temporary directory
cargo run -p tmc-replay -- play --from tests/fixtures/journal/basic_session.log --to $env:TEMP\tmc-sim --speed 10

# 3. Anonymize a real journal before committing it as a fixture (strips friends, redacts chat, substitutes CMDR/FID)
cargo run -p tmc-replay -- anonymize --from C:\Path\Journal.log --to tests\fixtures\journal\new_fixture.log

# 4. Run scientific evaluation against real journals without printing personal data (8 math oracles + SQLite timings)
cargo run -p tmc-headless -- eval --timings

# 5. Measure real memory footprint and idle CPU against budget (< 150 MB RAM, < 1% CPU)
pwsh scripts/measure-footprint.ps1 -Exe target/release/TMC.exe -JournalDir tests/fixtures/journal
```

The `tmc-headless eval` subcommand audits real journals without leaking PII to verify FSD jump range accuracy (`±0.05 ly`), Universal Cartographics payouts (`±5%`), exobiology predictions and `×5` multiplier, combat bond settlements, refinery yields, trade profit margins, massacre mission stack kill tracking, and engineering inventory bookkeeping.

---

## 11. Compilation, Automated Tests, and NSIS Packaging

### Prerequisites
- **Rust** stable (`1.90+`, 2024 edition) with native toolchain (MSVC Build Tools on Windows; `libwebkit2gtk-4.1-dev` on Linux for GUI desktop).
- **Node.js** `22+` and `npm`.
- **Tauri CLI v2** (`cargo install tauri-cli@^2`) for packaging the installer.

### Full Build & Verification Workflow

```powershell
# 1. Install frontend dependencies and build static assets (Svelte 5 + Vite)
npm.cmd --prefix ui ci
npm.cmd --prefix ui run build

# 2. Frontend quality checks & tests (TypeScript / svelte-check + Vitest)
npm.cmd --prefix ui run check
npm.cmd --prefix ui run test

# 3. Rust backend quality checks & tests (formatting, zero clippy warnings, and >1,400 workspace tests)
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# 4. Live UI development with hot-reload (Vite proxy -> 127.0.0.1:47320)
# Terminal 1: cargo run -p tmc-headless -- --journal-dir <path>
# Terminal 2: npm.cmd --prefix ui run dev

# 5. Build optimized Release binaries
cargo build -p tmc-desktop --release
cargo build -p tmc-headless --release

# 6. Build multilingual Windows NSIS installer
Push-Location apps/desktop
cargo tauri build
Pop-Location
# Output: target/release/bundle/nsis/TMC_0.1.0_x64-setup.exe
```

> **Note on disk hygiene:** always compile to the standard workspace `target/` directory (never define custom `CARGO_TARGET_DIR` variables to avoid duplicating dozens of gigabytes in cached dependencies).

---

## 12. Contributing, Privacy, and Licensing

- **License:** distributed under **[PolyForm Noncommercial 1.0.0](LICENSE)** (permits inspecting, auditing, modifying, building, and sharing the software for personal, educational, or non-profit community purposes). Incorporating code or datasets with incompatible licenses such as GPL/AGPL is strictly prohibited; all third-party dependencies are documented in [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
- **Contribution Guidelines:**
  - No user-visible strings or spoken cues may be hardcoded: all UI and voice strings must be defined in `locales/{es,en,fr,de,pt}/*.ftl` (`es` serves as the reference catalog validated by `cargo test -p tmc-i18n`).
  - Never commit API keys, private tokens, or real player journal files without first running them through `tmc-replay anonymize` and conducting manual inspection.
  - Adhere strictly to the cockpit design system in [`docs/design-system-glass-hud.md`](docs/design-system-glass-hud.md).
- **Privacy:** TMC does not include any telemetry, analytics, or remote tracking. Optional third-party API keys are stored solely in your local database, Frontier CAPI uses direct PKCE OAuth2 against Frontier Store, and EDDN submissions use locally salted pseudonyms.
- **Legal Notice:** *Elite Dangerous*, *Horizons*, and *Odyssey* are registered trademarks of Frontier Developments plc. TMC is an independent community project and is not affiliated with or officially endorsed by Frontier Developments.
