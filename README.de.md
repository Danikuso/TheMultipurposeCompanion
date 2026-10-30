# TMC — The Multipurpose Companion

<p align="center">
  <a href="README.md"><strong>English</strong></a> ·
  <a href="README.es.md"><strong>Español</strong></a> ·
  <a href="README.fr.md"><strong>Français</strong></a> ·
  <a href="README.de.md"><strong>Deutsch</strong></a> ·
  <a href="README.pt.md"><strong>Português</strong></a>
</p>

[![Version](https://img.shields.io/badge/version-0.1.0-orange.svg)](https://github.com/Danikuso/TheMultipurposeCompanion/releases)
[![Lizenz: PolyForm Noncommercial 1.0.0](https://img.shields.io/badge/lizenz-PolyForm%20Noncommercial%201.0.0-blue.svg)](LICENSE)
[![Plattform](https://img.shields.io/badge/plattform-Windows%2010%2F11%20%7C%20Linux%20%2F%20Steam%20Deck-green.svg)]()
[![Inara Whitelist](https://img.shields.io/badge/Inara%20InAPI%20v1-Offizielle%20Whitelist-success.svg)]()

**TMC (*The Multipurpose Companion*)** ist ein kostenloser, quelloffener Echtzeit-Cockpit-Begleiter für ***Elite Dangerous*** (*Live 4.0 / Odyssey / Horizons* und *Legacy 3.8*), entwickelt mit **Rust**, **Tauri 2** und **Svelte 5**.

Sein Ziel ist es, sowohl Fluganfängern als auch erfahrenen Kommandanten als umfassender Copilot zur Seite zu stehen: Er vereint in einer einzigen ultraleichten Anwendung **10 kombinierte Cockpit-Layouts**, **16 spezialisierte Module**, **Routenplanung**, **Schritt-für-Schritt-Anleitungen zur Ingenieursfreischaltung**, **akustische Sprachansagen und Mikrofonabfragen** sowie einen **Touchscreen-Zweitbildschirm für Tablet oder Smartphone**.

---

## Inhaltsverzeichnis

- **Ressourcen für Spieler:** [Kommandanten-Handbuch & Anleitungen (PDF)](website/Manual_TMC_Comandante.pdf) · [Übersichts-Website (`website/`)](website/index.html)
- **[Teil I — Spieler- und Benutzerhandbuch](#teil-i--spieler--und-benutzerhandbuch)**
  1. [Philosophie und Kernfunktionen](#1-philosophie-und-kernfunktionen)
  2. [Installation und Erste Schritte (Windows und Linux / Steam Deck)](#2-installation-und-erste-schritte)
  3. [Funktionsweise der Benutzeroberfläche (`TopBar`, `Auto-companion` und die 10 Layouts)](#3-funktionsweise-der-benutzeroberfläche)
  4. [Alle Funktionen: Die 16 Module im Detail](#4-alle-funktionen-die-16-module-im-detail)
  5. [Sprach-Copilot, Mikrofonabfragen, VoiceAttack und Tablet / Smartphone (QR)](#5-sprach-copilot-mikrofonabfragen-voiceattack-und-tablet--smartphone)
  6. [Einstellungen, Anpassung und Feedback / Community-Support](#6-einstellungen-anpassung-und-feedback--community-support)
- **[Teil II — Technische Entwicklerdokumentation](#teil-ii--technische-entwicklerdokumentation)**
  7. [Workspace-Architektur (12 Crates + 2 Apps + Frontend)](#7-workspace-architektur)
  8. [Journal-Erfassung, Speicherung, Sicherheit und Performance](#8-journal-erfassung-speicherung-sicherheit-und-performance)
  9. [Lokale Pfade, CLI-Argumente und Umgebungsvariablen](#9-lokale-pfade-cli-argumente-und-umgebungsvariablen)
  10. [Entwickler-Werkzeuge, Replay und Wissenschaftliches Audit](#10-entwickler-werkzeuge-replay-und-wissenschaftliches-audit)
  11. [Kompilierung, Automatisierte Tests und NSIS-Paketierung](#11-kompilierung-automatisierte-tests-und-nsis-paketierung)
  12. [Mitwirken, Datenschutz und Lizenzierung](#12-mitwirken-datenschutz-und-lizenzierung)

---

# Teil I — Spieler- und Benutzerhandbuch

## 1. Philosophie und Kernfunktionen

- **Ein Begleiter, der Sie nie im Stich lässt:** Wenn Sie ein Schiffsmodul modifizieren möchten, aber den erforderlichen Ingenieur noch nicht kennen oder freigeschaltet haben, zeigt TMC keine leere Fehlermeldung an: Es erklärt zunächst **Schritt für Schritt, wie Sie ihn entdecken, seine Einladung erhalten und welche Freischaltaufgabe oder Spende seine Werkstatt verlangt** (anhand Ihres tatsächlichen Journal-Fortschritts), und berechnet direkt darunter die erforderlichen Materialien und Tauschgeschäfte.
- **Visuelle Klarheit («TMC Glass HUD») und Schiffsfarben-Synchronisation:** Jedes Panel zeigt oben die wichtigsten Informationen mit Messinstrumenten, Ampelsystemen und 1-Klick-Aktionen an, während umfangreiche Tabellen unter **`Zusätzliche Informationen anzeigen`** übersichtlich eingeklappt bleiben. Darüber hinaus erkennt das Theme **`Auto-HUD`** automatisch die benutzerdefinierte Farbmatrix Ihres Schiffscockpits im Spiel und passt die Farbpalette exakt an.
- **Ressourcenschonend und leise im Hintergrund:** Entwickelt, um weder Framerate noch PC-Ressourcen zu beeinträchtigen. Beim Schließen des Fensters (`✕`) bleibt TMC im Infobereich der Taskleiste (System Tray) aktiv, gibt seinen Grafikspeicher frei und führt Sprachwarnungen sowie das automatische Kopieren von Sprungzielen nahtlos weiter.
- **100 % respektvoll gegenüber Privatsphäre und Legacy-Galaxie:** Funktioniert vom ersten Augenblick an vollständig lokal ohne Registrierungszwang. Erkennt TMC, dass Sie in der **Legacy 3.8**-Galaxie spielen, blockiert es automatisch jegliche Übertragung an externe Community-Server (EDDN, EDSM, Inara), um die Live-Datenbanken sauber zu halten.
- **5 vollständige Sprachen:** Benutzeroberfläche, Ingenieursdaten und Sprachausgabe verfügbar auf **Deutsch**, **Englisch**, **Spanisch**, **Französisch** und **Portugiesisch**.

---

## 2. Installation und Erste Schritte

### Windows 10 / 11 (64-Bit)

1. Laden Sie das Installationsprogramm **`TMC_0.1.0_x64-setup.exe`** aus dem Bereich [Releases](https://github.com/Danikuso/TheMultipurposeCompanion/releases) herunter (Assistent in allen 5 Sprachen verfügbar).
2. Führen Sie das Setup aus und folgen Sie den Schritten.

> [!NOTE]
> **Hinweis zu Windows Defender SmartScreen:**  
> Da es sich um ein kostenloses Open-Source-Projekt ohne teures kommerzielles Unternehmenszertifikat handelt, zeigt Windows beim ersten Start möglicherweise den blauen Dialog *«Der Computer wurde durch Windows geschützt»* an. Klicken Sie auf **«Weitere Informationen»** und anschließend auf **«Trotzdem ausführen»**.

3. Starten Sie **TMC** vor oder nach *Elite Dangerous*. Das Programm erkennt automatisch Ihren Spielstände-Ordner, importiert den Verlauf im Hintergrund (`Verlauf wird importiert: X von Y`) und merkt sich Fenstergröße und -position für künftige Sitzungen.
4. **Infobereich (System Tray):** Beim Klick auf `✕` minimiert sich TMC neben die Windows-Uhr. Ein Linksklick (oder Rechtsklick → **`TMC öffnen`**) stellt das Fenster wieder her; über **`Beenden`** wird das Programm vollständig geschlossen.

### Linux und SteamOS (Steam Deck)

- TMC erkennt automatisch das Journal-Verzeichnis von *Elite Dangerous* innerhalb der Steam-**Proton**-Präfixe (sowohl native als auch Flatpak-Installationen, AppID `359320`).
- Sie können die Desktop-GUI-Anwendung (`tmc-desktop`) oder den ressourcenschonenden Headless-Server (`tmc-headless`) starten, um das Cockpit im Webbrowser oder auf einem Tablet im selben WLAN-Netzwerk aufzurufen.

---

## 3. Funktionsweise der Benutzeroberfläche

### 3.1 Obere Statusleiste (`TopBar`)
Stets am oberen Bildschirmrand fixiert:
- **Verbindungsstatus & Galaxie:** Live-Verbindungsanzeige, **`LIVE 4.0`**- oder **`LEGACY`**-Badge und Kontextwarnungen (`Spiel läuft nicht` oder `Legacy-Galaxie: Daten bleiben lokal`).
- **6 Sofort-Flugmetriken:** Aktuelles **`System`**, **`MODUS`** (aus 10 Flug- und Oberflächenzuständen: *Angedockt, Gelandet, Normalraum, Supercruise, Hyperraum, Zu Fuß, Im SRV, Im Ship-Launched Fighter, Im Apex-Taxi, Multicrew*), **`Treibstoff`** (mit gelber Warnung unter 25 %), **`Rumpf`** (mit rotem Alarm unter 30 %), **`Fracht`** (`belegt / Kapazität`) und **`Credits`**.
- **Schnellsteuerungen rechts:** Taktische Sperrtaste **`🔓` / `🔒`** (mit Tastaturkürzel **`Alt + L`** zum sofortigen Einfrieren der Anzeige), Mikrofon-Taste **`🎙️`** und Schalter für **`Auto-companion`**.

### 3.2 Auto-Companion Pro: Vorausschauende Intelligenz, Taktische Sperre und Prioritäten
- **Intelligente, kontinuierliche Cockpit-Umschaltung:** Bei aktiviertem **`Auto-companion`** schaltet TMC anhand der Geschehnisse im Spiel automatisch auf die passendste Cockpit-Ansicht um (Kampf, Asteroidenprospektion, Stationslandung, Ingenieure, Flottenträgersprung, Kolonisierung oder Raumreise).
- **1-Klick-Taktiksperre (`Alt + L` / Vorhängeschloss):** In brenzligen Kämpfen, bei der Routenprüfung oder beim Rezeptvergleich klicken Sie auf das Schloss in der oberen Leiste oder drücken **`Alt + L`**, um die aktuelle Ansicht einzufrieren. Das Schloss leuchtet bernsteinfarben im *Glass HUD*-Stil und verhindert automatische Umschaltungen, bis Sie es wieder lösen.
- **Hysterese & Ruckelfreie Verweildauer (`linger_seconds`):** Nach Beendigung eines Gefechts, einer Bergbauprospektion oder eines Warenverkaufs springt das Bild nicht abrupt weg. Es verbleibt für die eingestellte Zeitspanne (Standard: 20 Sekunden, einstellbar von 0 bis 60 s in den Einstellungen) im aktiven Cockpit, damit Sie Schäden, Beute und Statistiken in Ruhe prüfen können.
- **Vorausschauende Flugübergänge:** TMC antizipiert Ereignisse, bevor sie eintreffen:
  - *Drohende Interdiktion:* Bei einem feindlichen Abfangversuch im Supercruise schaltet das Cockpit bereits während des Fluchtvektorkampfs sofort auf *Kampf* um.
  - *Ringannäherung:* Beim Sinkflug auf Planetenringe oder Asteroidengürtel mit montierter Bergbauausrüstung wechselt TMC proaktiv auf *Bergbau*.
  - *Landeerlaubnis erteilt:* Sobald eine Station oder ein Außenposten die Landeerlaubnis erteilt, schaltet TMC auf *Handel* um und bereitet Markt sowie Stationsdienste vor.
- **Benutzerdefinierte Prioritätenreihenfolge:** In den `Einstellungen` können Sie die Hierarchie aller 9 Rollen mit einfachen Pfeiltasten `▲` und `▼` festlegen oder über `Standardreihenfolge wiederherstellen` zur empfohlenen Priorisierung zurückkehren.
- **Akustisches Cockpit-Feedback:** Aktivieren Sie den dezenten Cockpit-Ton (*Glass HUD Chime*, per Web Audio API synthetisiert von 880 Hz auf 1320 Hz ohne Zusatzdateien) oder TTS-Sprachansagen des Copiloten, um Umschaltungen wahrzunehmen, ohne den Blick vom Fluggeschehen abzuwenden.
- Im Layout **`Erkundung`** schaltet das **`Kontext-Panel`** automatisch zwischen **`Himmelskörper`** und **`Exobiologie`** um, sobald Sie auf einer Welt mit Biosignalen landen.
- Wenn Sie bei aktivem `Auto-companion` manuell ein anderes Layout anklicken (ohne aktive Sperre), respektiert TMC Ihre Wahl, bis sich die Spielaktivität ändert. Das Ausschalten von `Auto-companion` fixiert die Anzeige dauerhaft.

### 3.3 Die 10 Kombinierten Layouts vs. Die 16 Module
Die linke Seitenleiste bietet zwei Navigationsmodi (neben der fest verankerten Schaltfläche **`⚙️ Einstellungen`**):

| Kombiniertes Layout | Auf einem Bildschirm vereinte Panels |
|---|---|
| **1. `Erkundung`** | `Routenband`, `Kontext-Panel` (Umschalter `Himmelskörper` / `Exobiologie`), `Routenplaner` und `Sprungreichweite`. |
| **2. `Schiff`** | `Schiff` (Ausrüstung und Ingenieure), `Sprungreichweite`, `Frachtraum` und `Werft- & Ausrüstungssuche`. |
| **3. `Kampf`** | `Zielinformationen`, `Jagdsitzung`, `Mein Schiff` sowie automatische `Konfliktzonen`- und `Anti-Xeno (AX)`-Panels bei Aktivität. |
| **4. `Bergbau`** | `Bergbau-Zielfinder` (Dreiecksroute und Bestpreis-Verkauf), `Prospektor`, `Bergbau-Sitzung` und `Verkaufsorte`. |
| **5. `Handel`** | `Aktueller Markt`, `Frachtraum`, `Handelssitzung`, `Handelsroute` und `Ware suchen`. |
| **6. `Missionen`** | `Aktive Missionen`, `Hinweise`, `Sitzungsbelohnungen` sowie automatische `Massaker-Stapel`-, `Passagiere`-, `Tourismus`- und `Community-Ziele`-Panels. |
| **7. `Ingenieure`** | `Materialinventar`, `Ingenieure`, `Ingenieurshinweise`, `Modifiziertes Schiff`, `Modul modifizieren`, `Einkaufsliste`, `Materialtausch` und `Fundorte`. |
| **8. `Flottenträger`** | `Trägerstatus`, `Sprungsequenz-Countdown` und `Finanzen & Unterhalt`. |
| **9. `Kolonisierung`** | `Baudepot` und `Versorgung & Beschaffung`. |
| **10. `Powerplay 2.0`** | `Powerplay-Status` und `Systemfraktionen` (Hintergrundsimulation - BGS). |

Unterhalb der Layouts finden Sie die Liste der **16 Einzelmodule** (mit Aktivitätsanzeige bei aktuellen Daten), um jedes Werkzeug dediziert in voller Breite zu öffnen.

---

## 4. Alle Funktionen: Die 16 Module im Detail

### 1. `Dashboard` (7 Panels)
- **`Kommandant`:** Name, Credit-Guthaben und Prozent-Fortschrittsbalken für alle 6 Ränge der *Pilots Federation* (*Kampf, Handel, Erkundung, Söldner, Exobiologe, CQC*) und 2 *Großmächte* (*Föderation, Imperium*).
- **`Schiff & Standort`:** Übersicht über aktives Schiff, Sternsystem, nächstgelegenen Himmelskörper, Station und aktuellen Modus.
- **`Live-Status`:** 7 Cockpit-Statusindikatoren (*Schilde, Waffenaufhängungen, Fahrwerk, Massensperre, Schleichfahrt, Frachtgreifer, Supercruise-Überladung SCO*) sowie Energieverteiler-Pips für `SYS`, `ENG` und `WEP` (0 bis 4 Pips).
- **`Route & Sitzung`:** Nächstes System mit Schaltfläche **`Nächstes System kopieren`**, verbleibende Sprünge, Sitzungsdauer, absolvierte Sprünge, zurückgelegte Lichtjahre und verdiente Credits.
- **`Warnungen`:** Sofortige Alarmierung bei *Treibstoffmangel, Überhitzung, laufender Interdiktion, Feindbeschuss, Rumpfschaden, Sauerstoffmangel, niedriger Gesundheit* oder Modulfehlfunktion.

### 2. `Schiff` (2 Panels: Technische Prüfung und Werft-/Ausrüstungssuche)
- **Kopfzeile & Messanzeigen (`Schiff`):** Modellbezeichnung, Schiffsname, Kennung, Leermasse, Versicherungskosten (*Rebuy*) sowie Füllstände für Rumpf, Schilde, Haupt- + Reservetank und Fracht.
- **3 Vorflug-Checklisten:**
  - **`🧭 Erkundung`:** Maximale Sprungreichweite und Kontrollliste (`✅` / `⚪`) für Treibstoffsammler, Guardian-FSD-Booster, Detaillierten Oberflächenscanner (DSS), SRV-Hangar und Automatische Feld-Wartungseinheit (AFMU).
  - **`⚔️ Kampf`:** Schildstärke, Wärmelast, `SYS/ENG/WEP`-Aufteilung, installierte Waffen mit Munition `[Magazin / Reserve]` und defensive Hilfssysteme (*Schildbooster, Chaff-Werfer, Punktverteidigung und Kühlkörper*).
  - **`📦 Fracht & Bergbau`:** Gesamtkapazität, Frachtgestelle, Raffinerie, Drohnensteuerungen und Fracht an Bord.
- **`Modulaufschlüsselung`:** Filterbar nach Kategorie (*Alle, Aufhängungen, Primär, Optional, Hilfssysteme*) und Volltextsuche. Zeigt Klasse & Wertung `[5A]`, Zustand `%`, Schaltzustand `ON/OFF`, Energiepriorität `P1..P5` und goldenes Emblem `G1..G5 ★`, das beim Anklicken Bauplan, Ingenieur, Experimentaleffekt und den **präzisen Attributvergleich (modifizierter Wert vs. Originalwert)** öffnet.
- **`Werft- und Ausrüstungssuche` (Kaufassistent aus einer Hand):**
  - **Kombinierter Warenkorb ab Standort:** Geht automatisch von Ihrem aktuellen Sternsystem aus (auch im unkartierten Tiefraum über 3D-Koordinaten) und erlaubt die Auswahl von **1 optionalem Schiff** (48 Typen) und/oder **bis zu 10 Modulen** (153 Familien mit Kategorie, Klasse `0..8`, Wertung `A..I`, Waffenmontage *Fest/Kardanisch/Turm* und Panzerungstyp), um die nächstgelegenen Stationen zu ermitteln, die den **gesamten Korb auf einmal anbieten**.
  - **Intelligente Einzelsuche als Rückfallebene:** Bietet keine Station alle Teile gemeinsam an, findet das System automatisch die nächste Station für jeden fehlenden Artikel einzeln und markiert Komponenten ohne bekannten Lagerbestand.
  - **Schutzfunktionen und Schalter:** **`−15 % Li Yong-Rui-Rabatt`** (der auch künftige Versicherungskosten dauerhaft senkt), Schalter **`Bastionsträger Ihrer Macht`** für Flottenträger-Raumstationen Ihrer Powerplay-2.0-Fraktion (unter Ausschluss rivalisierender und privater Träger), Systemerlaubnis-Schutz (`Shinrarta Dezhra`, `Sol`, `Achenar`, `Sirius`, `Alioth`…), **Versicherungs-Ampel** (`Versicherung OK` / `Anpassen` / `Guthaben unzureichend`), Einzelpreisauflistung und 1-Klick-Schaltflächen **`Kopieren`** und **`Route`**.

### 3. `Navigation` (3 Panels und Spansh-Doppelroutenplaner)
- **`Routenband`:** Zeigt Ihren aktiven Spansh-Routenplan oder die im Spiel berechnete Route. Bei Kursabweichung erscheint der Hinweis **`Abseits der Route`** mit **`Neu berechnen`** und **`Plan löschen`**. Beschreibt jeden Sprung in 4 Spalten: Navigation (Distanz, Spektralklasse, Treibstoffsammelbarkeit **`⛽ Sammelbar`**, geschätzter Resttreibstoff), Erkundung (entdeckt/unentdeckt, Vorbesuche, Neutronenstern/Weißer Zwerg), Politik (Machtzugehörigkeit, Regierung, Fraktion und **`⚠ Gefahrenstatus`** bei Krieg oder Ausbrüchen) sowie Wirtschaft/Sicherheit/Powerplay.
- **`Sprungreichweite`:** Aktuelle Reichweite (mit **`Aufgeladen ×N`**-Kennzeichnung bei Neutronen-Boost oder FSD-Einspritzung), Reichweite mit vollem Tank, aktueller Treibstoffstand und Maximalverbrauch pro Sprung.
- **`Routenplaner`:** System-Autovervollständigung, automatische 3D-Koordinaten (selbst aus tiefen Systemen ohne öffentlichen Datenbankeintrag) und 2 Berechnungsmodi:
  - **`⚡ Neutronen-Highway`:** Schnelles Reisen über Neutronensterne mit anpassbarem **`Effizienzgrad (%)`**.
  - **`🗺️ Exakte Route (Exact Plotter)`:** Liest Masse, Tanks, Fracht, verbesserten FSD-Antrieb und Guardian-Booster direkt aus Ihrem Schiff ohne externe Uploads, mit Optionen für **`Nur betankbare Sterne`** und **`FSD-Einspritzungen nutzen`**. Durch Klick auf **`Übernehmen`** wird die Route gespeichert und jedes nächste Ziel Sprung für Sprung in die Zwischenablage kopiert.

### 4. `Erkundung` (`Himmelskörper des Systems`)
- **Oberer Überblick:** Entdeckungsstatus des Systems, alle vorhandenen Sternklassen (Betankbare Sterne *O, B, A, F, G, K, M* grün hervorgehoben), **`VSS-Scanfortschritt`**, Vergleichskarten **`Systemwert`** vs. **`Wert mit empfohlenen Kartierungen`**, unkartierte Himmelskörper und Karte **`Nicht verkaufte Daten`** mit roter Warnung **`Daten in Gefahr`** bei Rumpfschäden.
- **Erweiterbare Detailtabelle:** 27 Stern- und Planetenklassen, Distanz in Lichtsekunden, Scan- und Kartierungswert, `Bio`- und `Geo`-Signale, Markierungen (*Terraformierbar, Landbar, Rohstoffe, Erstentdeckung, Bereits kartiert*) und Spalte **`Empfehlung`** (`★ Kartieren` basierend auf Ihrem Credit-Schwellenwert, `Landen (Exobiologie)` oder `Überspringen`, inkl. DSS-Sonden-Effizienzziel).

### 5. `Exobiologie` (Odyssey-Oberflächenproben-Assistent)
- **Himmelskörper-Kopfzeile:** Oberflächengravitation, Temperatur, Atmosphärentyp, Anzahl biologischer Signale und Emblem **`UNREGISTRIERT · ×5 (geschätzt)`**, wenn Sie der erste Kommandant sind, der dort Daten erfasst.
- **`Laufende Probennahme` (`1/3 → 2/3 → 3/3`):** Fortschrittsbalken und Live-Zähler für **`Distanz zur vorherigen Probe`** im Abgleich mit der Mindestklondistanz der Spezies (`Noch X m erforderlich`), **relative Richtungs- und Kompassanzeige** passend zu Ihrer Blickrichtung (z. B. *«vorherige Probe liegt hinten links (SW)»*), Warnung beim Verwerfen unvollständiger Proben und Wertkarte **`Nicht verkauft an Bord`** mit Gewinnschätzung.
- **Tabelle `Mögliche Spezies (nach Wert sortiert)`:** Verbindet astrophysikalische Bedingungen, Ihren Flugverlauf und *Canonn*-Daten zur Anzeige von Basisauszahlung, Payout `Mit ×5 Bonus`, Mindestabstand, Wahrscheinlichkeit und durch DSS-Sonden bestätigten Biosignalen.

### 6. `Kampf` (5 taktische Panels und Anti-Xeno)
- **`Ziel`:** Ausgewähltes Schiff mit Fraktionswappen, Pilot, Kampfrang, Fraktion, gut sichtbarem Banner **`SAUBER · FEUER EINSTELLEN`** (grün) oder **`GESUCHT · Kopfgeld`** (rot), Schild- und Rumpfbalken sowie prozentuale Gesundheit des anvisierten **`Subsystems`** (*Kraftwerk, Antriebe, FSD, Waffen…*), wobei Daten 10 Sekunden nach Zielverlust erhalten bleiben.
- **`Mein Schiff`:** Schildstatus, 10-Segment-Rumpfleiste, Temperaturanzeige mit Überhitzungs-Countdown, Energieverteiler-Anzeige (`SYS`, `ENG`, `WEP`) und aktive Cockpit-Alarme.
- **`Jagdsitzung`:** Abschüsse, verdiente Credits, Stundensatz (`cr/h`), Kampfrang-Fortschritt, Taste **`Sitzung zurücksetzen`**, Aufschlüsselung der **`Offenen Kopfgelder`** nach Fraktion (inkl. Nettoberechnung mit **`−25 % bei Interstellar Factors`**) und nächstgelegener *Interstellar Factors*-Standort.
- **`Konfliktzone & Anti-Xeno (AX)` (Automatisch):** Gewählte Kriegspartei, Abschüsse und Kampfbelohnungen; Thargoiden-Warnung, AX-Bonds und Abschusszähler nach Variante (*Scout, Cyclops, Basilisk, Medusa, Hydra, Scythe, Glaive, Orthrus…*).

### 7. `Bergbau` (4 Panels: Zielfinder, Prospektor, Sitzung und Verkaufsorte)
- **`Bergbau-Zielfinder & Bestpreis-Verkauf (Miner's Companion)`:**
  - **Kürzeste Dreiecksroute:** Berechnet im Cockpit den optimalen Rundkurs: von Ihrem aktuellen System ➔ zum ertragreichsten Ring oder Gürtel ➔ zur nächstgelegenen Station mit dem höchsten Ankaufspreis für Ihre Erze.
  - **3 Spezialisierte Abbauprofile & Sofort-Presets:**
    - **Laser:** Fokussiert auf Platin in metallischen Ringen, Painit und Osmium. Beinhaltet Kombinationen mit *Hazardous RES* (+100 % Fragmente pro Gesteinsbrocken) und *High RES* (+75 % Fragmente) sowie geprüfte Mehrfach-Überlappungen (*Overlaps*).
    - **Tiefenkern (*Core*):** Fokussiert auf Musgravit, Monazit, Alexandrit, Tieftöner-Opale und Grandidierit in eisigen und felsigen Ringen.
    - **Missionen & Ingenieure:** Bromellit (zur Freischaltung von Ingenieur Bill Turner in Alioth), Tritium (zur Betankung von *Flottenträgern*) und Osmium.
  - **Warnung vor Nachfragesteuer-Risiko (*Demand Tax Risk*):** Die Wirtschaft von *Elite Dangerous* senkt Ankaufspreise drastisch, wenn Ihre Ladung 25 % der Gesamtnachfrage der Station übersteigt. TMC warnt präventiv per Ampelsystem und nennt die exakte Tonnagegrenze, um Gewinneinbußen zu vermeiden.
  - **1-Klick-Frachtraum-Verkaufsoptimierer (*Multi-Sell*):** Analysiert die tatsächlich im Schiff gelagerten Mineralien und Metalle und ermittelt die optimale Station für den gewinnmaximierten Gesamterlös.
  - **Kuratierter Offline-Katalog & Atomare Online-Synchronisation:** Bringt ab Werk über 40 geprüfte ertragreiche Vorkommen mit (*EDTools/Spansh*), funktioniert sofort ohne Internetverbindung und synchronisiert sich bei bestehender Verbindung im Hintergrund.
  - **Cockpit-Aktionen:** Schaltflächen **`📋 System kopieren`** und **`🚀 Route berechnen`** direkt auf jeder Karte und in der adaptiven Tabelle mit fixierter horizontaler Scrollleiste.
- **`Prospektor`:** Erzgehalt (*Niedrig/Mittel/Hoch*), deutliches Banner **`◆ Kern: Mineral`** für Tiefensprengungen, mehrfarbiger Zusammensetzungsbalken, Mineralliste mit Favoriten-Hervorhebung `★`, automatisches Urteil **`✔ Lohnt sich`** oder **`✘ Überspringen`** basierend auf Ihrem Credit-Schwellenwert und Vergleichsgrafik der **`Letzten 8 Asteroiden`**.
- **`Bergbau-Sitzung`:** Dauer, raffinierte Tonnage und Rate in `t/h`, geschätzte Einnahmen und `cr/h`, verbleibende und verbrauchte **`Limpets`** nach Typ, Frachtraumauslastung, Aufschlüsselung nach Mineral und Füllstände der **`Raffinerie`**.
- **`Verkaufsorte`:** Beste empfohlene Gesamtstation für Ihre Ladung sowie aufklappbare Details pro Mineral mit Station, System (mit Kopiertaste `⎘`), Distanz, Marktpreis, Nachfrage, Landepad-Größe (`S/M/L`) und Datenalter.

### 8. `Handel` (5 Panels)
- **`Aktueller Markt`:** Hebt die besten Handelsmöglichkeiten der letzten 48 Stunden hervor und zeigt das Stationsangebot mit Kennzeichnung seltener Waren **`SELTEN`** (inkl. optimaler Verkaufsdistanz bei 160 Lj), Einkaufspreisen, Angebot, bestem regionalen Abnehmer und Gewinn pro Tonne.
- **`Meine Fracht`:** Frachtraumbelegung und chargengenaue Verfolgung des **`Durchschnittlichen Einkaufspreises`**, Kennzeichnung von Missions- oder Schmuggelware, Vergleich zwischen lokalem Verkauf und dem besten Nachbarmarkt sowie Empfehlung **`HIER VERKAUFEN`** oder **`WEITERTRANSPORTIEREN`**.
- **`Handelsroute`:** Multi-Stopp-Routenoptimierer ab Ihrer aktuellen Station mit genauer Frachtaufteilung pro Zwischenlandung, Sprunganzahl, Gesamtgewinn und automatischer Zielkopie.
- **`Handelssitzung & Ware suchen`:** Nettogewinn, `cr/h`, transportierte Tonnage, profitabelste Tour, Handelsrang-Fortschritt und Suchmaske mit Autovervollständigung für jeden gesuchten Rohstoff in Systemnähe.

### 9. `Missionen` (7 Panels: Verträge, Massaker, Passagiere, Tourismus und CGs)
- **`Aktive Missionen & Hinweise`:** Verträge nach Ziel gruppiert mit Kennzeichnung `▶ Nächster Halt` und Schaltfläche **`Zur Route hinzufügen`**, Typ, Fraktion, Fortschritt, Ablauf-Countdown (rot bei `< 1 h`) und Alarmierung bei Kursabweichung oder fehlender Missionsware im Frachtraum.
- **`Massaker-Stapel`:** Fasst Piratenjagd-Aufträge gegen dieselbe Zielfraktion in parallelen Balken nach Auftraggeberfraktion zusammen (wobei jeder Abschuss gleichzeitig für alle verschiedenen Fraktionen gewertet wird), berechnet verbleibende Kills und Gesamtauszahlung.
- **`Passagiere`:** Kabinenbelegung nach Klasse (*Economy, Business, First, Luxus*), Kennzeichnungen **`VIP`** / **`GESUCHT`** und rote Sicherheitswarnung **`⚠ Gesuchter Passagier an Bord: Scan-Gefahr in Stationssicherheitszonen`**.
- **`Tourismus`:** Ordnet Touristenbojen automatisch zur kürzesten **`Optimalen Rundreise (TSP)`** gegenüber der Auftragsreihenfolge an (mit Einsparung an Lichtjahren und Sprüngen), berechnet `cr/ly` und `cr/h` und fällt ein Urteil **`✔ RENTABEL`** oder **`✖ UNRENTABEL`**.
- **`Sitzungsbelohnungen & Community-Ziele`:** Übersicht über Credits, Ingenieursmaterialien und Reputations-/Einflussveränderungen pro Fraktion sowie Live-Verfolgung aktiver *Community Goals* (*Beitrag, Stufe, Perzentil, Restzeit und erwartete Prämie*).

### 10. `Ingenieure` (8 Panels für Schiffsmodule und Odyssey-Ausrüstung)
- **`Materialien`:** 4 Reiter (*Rohmaterialien, Gefertigt, Verschlüsselt* und Odyssey-*Spind* unterteilt in *Gegenstände, Komponenten, Verbrauchsgüter und Daten*), Schnellsuche, **`Sitzungsverlauf`** nach Herkunft (*Mission, Scan, Bergbau, Tausch, Ingenieur…*) und **`KAPAZITÄT VOLL`**-Warnungen.
- **`Ingenieure & Mein modifiziertes Schiff`:** Filterbarer Katalog (*Alle, Freigeschaltet, Ausstehend*) aller **Schiffs- und Odyssey-Boden-Ingenieure** mit Rang `★`, Entfernung und Fortschritt der Freischaltbedingungen; Schnellauswahl für jedes modifizierbare Modul auf Ihrem Schiff.
- **`Modul modifizieren`:** Wählen Sie Bauplan, Experimentaleffekt, Stufe `G1..G5` und Versuche pro Stufe. Fehlt Ihnen der Zugang zu einem passenden Ingenieur, blendet das System den **`BEGLEITER-LEITFADEN — INGENIEURSFREISCHALTUNG`** mit den 3 Schritten (*1. Entdeckung → 2. Einladung → 3. Werkstatt-Spende*), System mit Schnellkopie und aktuellem Fortschritt ein, gefolgt von exakten Kosten und Materialampel (`OK`, **`Durch Tausch abgedeckt`** oder `Fehlt`).
- **`Einkaufsliste, Materialtausch, Fundorte und Hinweise`:** Verwalten Sie mehrere Projekte, ordnen Sie diese nach Priorität (**`▲ Nach oben` / `▼ Nach unten`**) zur Materialreservierung, ermitteln Sie Tauschkurse mit Überschüssen (`Gebe X → Erhalte Y`) und finden Sie den nächsten **Materialhändler** oder **Technologie-Broker** (*Menschlich* und *Guardian*) samt Sammelanleitungen.

### 11. `Flottenträger` (3 Panels)
- **`Trägerstatus`:** Kennung, Name, `AN BORD`-Badge, aktuelles System, verbleibende Sprünge (500 Lj/Sprung), Landeberechtigungen, **`Tritium`**-Tankstand (`1.000 t`) + Frachtraumreserven und Kapazitätsverteilung.
- **`Sprungsequenz`:** Betriebsstatus und Live-Timer `MM:SS` für den 15-Minuten-Sprung-Countdown und die 5-minütige Abkühlphase samt Zielsystem und Schaltfläche **`Ziel kopieren`**.
- **`Finanzen & Unterhalt`:** Finanzstatus (*Gesund, Warnung, Kritisch*), **`Abgedeckte Wochen`**, Trägerguthaben, wöchentlicher Unterhalt, Reserve und Datum der nächsten Abbuchung.

### 12. `Kolonisierung` (2 Panels)
- **`Baudepot`:** Gesamter prozentualer Baufortschritt, gelieferte vs. ausstehende Tonnage, dynamische Berechnung der **`Geschätzten Frachtflüge`** anhand der Kapazität Ihres aktiven Schiffes und tabellarische Ressourcenaufschlüsselung.
- **`Versorgung & Beschaffung`:** Karten ausstehender Materialien nach Dringlichkeit mit Tonnagen, verbleibenden Flügen und **`Kopieren`**-Taste für den schnellen Markteinkauf.

### 13. `Powerplay 2.0` und 14. `BGS`
- **`Powerplay-Status`:** Wappen von Macht und Großmacht, Treuewochen, aktueller Rang (`X / 100`), gesammelte und verbleibende Meriten bis zur nächsten Stufe, Sitzungsmeriten, **`Meriten-Stundensatz`** und Countdown zum wöchentlichen Server-Tick (*Donnerstag 07:00 UTC*).
- **`Systemfraktionen (BGS)`:** Kontrollierende Fraktion, aktive Zustände, täglicher **`Galaktischer Tick-Tracker`** (`Vor X h`, Kennzeichnung frischer oder anstehender Daten) und Fraktionstabelle mit Einflussbalken, Zuständen, Treue und Regierungsform.

### 15. `Kriminalität & Notorietät` (5 Taktikkarten)
- **`Rechtsstatus & Notorietät`:** Anzeige von 0 bis 10 mit Abkling-Timer (2 Stunden aktive Flugzeit pro Punkt), der erkennt und anzeigt: **`⏸️ Abbau pausiert (an Station angedockt)`**.
- **`Leitfaden zur Weste-Reinigung`:** Warnt bei Sperrung neutraler Kontakte durch Notorietät oder zeigt bei 0 direkt den **`Nächstgelegenen Interstellar Factors`** mit Distanz, Landepad-Größe und **`System kopieren`**-Taste.
- **`Schmuggel & Schwarzmärkte`:** Führt illegale Fracht auf, lokalisiert den **`Nächsten Schwarzmarkt`** und enthält einen 3-Schritte-Leitfaden für **`Lautlose Infiltration`** (8–10 km Ausrichtung, Schleichfahrt / Kühlkörper bei `< 4 km` und schneller Einflug).
- **`Strafenübersicht & Systemsicherheit`:** Strafen und Kopfgelder nach Zuständigkeit + Sicherheitsstufe mit geschätzten Polizeireaktionszeiten.

### 16. `Konten` (Optionale Community-Synchronisation)
- **`Elite Dangerous Data Network (EDDN)`:** Teilen Sie Marktpreise, Werftdaten und Sternsprünge anonym mit der Community und empfangen Sie lokale Marktaktualisierungen in Ihrem konfigurierten Radius.
- **`Elite Dangerous Star Map (EDSM)`:** Speichern Sie Ihren API-Schlüssel zur Synchronisation von Flugbuch, Rängen und Credits.
- **`Inara`:** Offiziell freigegeben auf der **Inara-Whitelist** unter dem Namen **`The Multipurpose Companion (TMC)`**, mit intelligenter Stapelübertragung (max. 2 Anfragen alle 60 s) und Verbindungstest-Taste.
- **`Frontier Companion API (CAPI)`:** Sichere OAuth2-Anmeldung über das offizielle Frontier-Portal und interaktive **`Fernflotten-Übersicht`** mit allen in anderen Systemen geparkten Schiffen samt Station, Wert und Entfernung in Lichtjahren.

---

## 5. Sprach-Copilot, Mikrofonabfragen, VoiceAttack und Tablet / Smartphone

### 5.1 Sprach-Copilot (Akustische Warnungen / TTS)
Unter `Einstellungen` → **Sprach-Copilot & Cockpit-Warnungen** haben Sie die Wahl zwischen:
- **`Betriebssystem-Nativ (Ultraleicht)`:** Verwendet sofort die in Ihrem Betriebssystem installierten Stimmen ohne zusätzliche Downloads.
- **`KokoroTTS Neural Engine (Kinomäßige KI-Stimme)`:** 10 natürliche lokale neuronale Stimmen in verschiedenen Sprachen mit automatischer Standby-Abschaltung bei Inaktivität zur RAM-Schonung (samt automatischem Fallback auf die Systemstimme).
- Beinhaltet Regler für **Lautstärke**, **Sprechgeschwindigkeit**, Testtaste **`🔊 Sprach-Copiloten testen`** und **5 unabhängige Kanäle** (*Kampf & kritische Warnungen*, *Navigation & Sternbetankung*, *Erkundung & Exobiologie*, *Bergbau & Prospektion* und *Flottenträger*).

### 5.2 Mikrofon-Sprachassistent (9 Gesprochene Abfragen in 5 Sprachen)
Unter `Einstellungen` → **Sprachassistenz & Integration** aktivieren Sie die Spracherkennung über **`Whisper AI Local`** (~39 MB, 100 % offline und privat, mit einem Klick installierbar) oder die System-Engine, wahlweise im Modus **Dauerhaftes Zuhören** oder **Push-to-Talk** (Mikrofon-Taste `🎙️` in der oberen Leiste). Fragen Sie in natürlicher Sprache nach:
1. **Schiffszustand** (Rumpf, Schilde und Treibstoff).
2. **Treibstoffstand** (Prozent und Tonnen).
3. **Nächster Sternsprung** (sagt Ihr nächstes Reiseziel an).
4. **Nächstes System kopieren** (sagt das Ziel an und **kopiert das System in die Zwischenablage**, bereit zum Einfügen in die Galaxiekarte).
5. **Biosignale im System** (Anzahl erkannter Biosignale).
6. **Materialübersicht** (Fortschritt der aktiven Ingenieurseinkaufsliste).
7. **Rechtsstatus & Strafen** (Notorietät, offene Strafgelder und Kopfgelder).
8. **Nächstgelegener Interstellar Factors** (nennt Station, System, Entfernung und **kopiert das System in die Zwischenablage**).
9. **Nächstgelegener Schwarzmarkt** (nennt Station, System, Entfernung und **kopiert das System in die Zwischenablage**).

### 5.3 `VoiceAttack`-Integration
TMC erkennt automatisch, wenn *VoiceAttack* auf Ihrem System läuft, generiert das vorkonfigurierte Profil **`TMC-Copiloto.vap`** und stellt in den Einstellungen die Taste **`📥 Profil herunterladen (.vap)`** zum direkten Import bereit.

### 5.4 Zusätzlicher Touchscreen auf Tablet oder Smartphone (`QR-Code`)
Wenn Sie mit nur einem Monitor oder in VR spielen, aktivieren Sie in den `Einstellungen` den Schalter **`Lokaler Netzwerkzugriff`** und scannen Sie den angezeigten **`Kopplungs-QR-Code`** mit der Kamera Ihres Mobilgeräts im selben WLAN. Das gesamte Cockpit öffnet sich ohne App-Installation im Browser; ein Fingertipp auf «System kopieren» auf dem Tablet überträgt das Ziel direkt in die Zwischenablage Ihres PCs.

---

## 6. Einstellungen, Anpassung und Feedback / Community-Support

- **Sprache und Themes:** Auswahl aus **5 Sprachen** (`es`, `en`, `fr`, `de`, `pt`) und **3 Designs** (**`Auto-HUD`** synchronisiert mit Ihren Cockpit-Farben im Spiel, **`Klassisches Orange`** und **`Hoher Kontrast`**).
- **Auto-Companion Pro:** Taktische Prioritätenreihenfolge für alle 9 Rollen (`▲` / `▼`) mit Rücksetztaste, Hysterese-Schieberegler (0 bis 60 s mit Live-Anzeige), Prädiktionsschalter und sensorischem Feedback (*Glass HUD Chime*-Ton und TTS-Stimme).
- **Betriebliche Einstellungen:** Automatisches Kopieren des nächsten Sprungs, Hauptschalter für `Auto-companion`, LAN-Netzwerkzugriff, globaler Datenschalter **`Online-Dienste abfragen (Spansh, EDSM, Canonn)`** (deaktivierbar für 100 % autarken Offline-Betrieb), **`DSS-Kartierungsschwelle (cr)`** und **`Standard-Routenplaner-Effizienz (%)`**.
- **Wirtschaft, Tourismus und Ingenieure:** Anpassbare Liste von bis zu 30 **`Mineralien von Interesse`** (11 vorkonfiguriert), **`Bergbau-Mindestwert (cr)`**, **`Handels-Suchradius (Lj)`**, **`EDDN-Marktdatenradius (Lj)`**, **`Tourismus-Referenz (cr/Lj)`** und **`Ingenieursversuche pro Stufe (G1–G5)`**.
- **Direkter Support- und Feedbackkanal:** Klicken Sie unter `Einstellungen` → **Support, Feedback & Community** auf **`🚀 Problem oder Vorschlag auf GitHub melden`**, um ein vorausgefülltes Ticket auf [GitHub Issues](https://github.com/Danikuso/TheMultipurposeCompanion/issues) zu öffnen (Version, Sprache und Betriebssystem vorausgefüllt, ohne private Spielerdaten).

---

# Teil II — Technische Entwicklerdokumentation

## 7. Workspace-Architektur

Das Projekt ist als modularer Workspace in **stabilem Rust (Edition 2024)** strukturiert, mit strikt getrennten Verantwortlichkeiten ohne zirkuläre Abhängigkeiten, kombiniert mit einem modernen Frontend in **Svelte 5 + TypeScript + Vite** und **Fluent**-Lokalisierungskatalogen:

| Crate / Verzeichnis | Technische Verantwortung |
|---|---|
| `crates/tmc-model` | Streng typisierte Domänenmodelle, Journal-Ereignisse, flüchtiger Spielzustand, WebSocket-Topics, strikte Diagnoseereignisse (`app.toggle_lock`, `app.set_auto_priorities`) und Konfigurationsschemata (`auto_companion_priorities`, `auto_companion_linger_seconds` usw.). |
| `crates/tmc-journal` | Plattformübergreifende Verzeichniserkennung (Win32 `FOLDERID_SavedGames`, Proton/Flatpak unter Linux), nicht-blockierendes inkrementelles Einlesen von `Journal.*.log` per *Byte-Offset* und abfragesichere Erfassung von `Status.json` + 9 Begleitdateien. |
| `crates/tmc-store` | Persistenz in **SQLite (WAL-Modus)** über `rusqlite`, additive Migrationen, Historie von Erkundung/Handel/Einkaufslisten, lokaler Tresor für Zugangsdaten und automatische Reparatur bei Beschädigung. |
| `crates/tmc-i18n` | Laden und Auflösen von **Project Fluent**-Katalogen (`locales/{es,en,fr,de,pt}/*.ftl`) mit Tests zur Gewährleistung einer 100 %igen Schlüsselparität gegenüber `es`. |
| `crates/tmc-net` | Asynchrone HTTP-Clients mit Ratenbegrenzung, Caching und Offline-Rückfallebene: *Spansh* (Neutron Plotter, Exact Plotter `/api/generic/route`, Werft-/Ausrüstungssuche `POST /api/stations/search`, Ring-/Körpersuche `/api/bodies/search` und Umkreissuche `/api/nearest`), *Ardent*, *Canonn*, *tick.edcd.io*, *EDSM*, *Inara InAPI v1* (max. 2 Req/min im Batch) und *Frontier CAPI* (OAuth2 S256 PKCE). |
| `crates/tmc-eddn` | HTTP-POST-Sender für offizielle *EDDN*-Schemata (`journal/1`, `commodity/3`, `outfitting/2`, `shipyard/2`) mit lokal gesalzenen Pseudonymen, plus ZeroMQ-Listener zur regionalen Marktfilterung. |
| `crates/tmc-tts` | Priorisierte Sprachwarteschlange mit Deduplizierung und Cooldowns (`TtsQueue`), Windows-native Sprach-Engine (SAPI5 / OneCore `< 2 MB` RAM) und entkoppelter neuronaler Worker-Prozess (`Kokoro-82M` ONNX mit Standby nach 60 s Leerlauf). |
| `crates/tmc-modules` | Fachlogik für alle 16 Spielmodule (einschließlich `outfitting_catalog.json` in `ship` und kuratiertem `mining_hotspots.json` mit atomarem In-Memory-Neuladen in `mining`), Relevanzberechnung mit Hysterese (`linger_seconds`), prädiktive Trigger und UI-Projektionen. |
| `crates/tmc-core` | Zentraler Zustandsakteur, Ereignis-Routing-Bus, taktischer `VoiceAdvisor` und mehrsprachiger Absichts-Parser `VoiceIntentResolver` (9 Sprachbefehle). |
| `crates/tmc-server` | HTTP/REST- und WebSocket-Server (`/ws`) auf Basis von **axum**, 100ms Topic-Coalescing (max. 10 Updates/s), `.vap`-Profilgenerator für *VoiceAttack*, DNS-Rebinding-/CSRF-Schutz und 128-Bit-LAN-Token-Authentifizierung (`HttpOnly; SameSite=Strict`). |
| `crates/tmc-app` | Dienstkomposition, Anwendungslebenszyklus, historischer Journal-Import mit adaptiver Drosselung und Überwachungsschleife. |
| `crates/tmc-replay` | CLI-Werkzeug für zeitbeschleunigtes Journal-Replay (`play`) und deterministische PII-Anonymisierung (`anonymize`) zur Erstellung von Test-Fixtures. |
| `apps/desktop` | Native **Tauri 2**-Anwendung (`TMC.exe`), Einzelinstanz-Mutex, Fensterzustands-Persistenz, System-Tray-Lebenszyklus und WebView2-Speicheraussetzung. |
| `apps/headless` | Eigenständiger Server ohne GUI und mathematischer Audit-Runner (`tmc-headless eval`). |
| `ui/` | Reaktive Oberfläche in **Svelte 5** (Runes), TypeScript, *TMC Glass HUD*-Designsystem, Bildschirmsperre (`Alt + L`), dynamische Prioritätsverwaltung, Web Audio API-Synthesizer, dynamische virtuelle Listen und lokale Offline-Spracherkennung per WebAssembly (`Whisper` via `@huggingface/transformers`). |
| `locales/` | Synchronisierte `.ftl`-Übersetzungskataloge für `es` (Referenz), `en`, `fr`, `de` und `pt`. |

---

## 8. Journal-Erfassung, Speicherung, Sicherheit und Performance

- **Nicht-intrusives Einlesen per Byte-Offset:** Benachrichtigungen des Betriebssystems dienen ausschließlich dazu, das Anlegen einer neuen `Journal.*.log`-Datei zu erkennen; das zeilenweise Lesen schreitet per Byte-Offset voran, ohne jemals Dateideskriptoren von `EliteDangerous64.exe` zu sperren. Flüchtige Dateien (`Status.json`, `Cargo.json`, `Market.json`, `Outfitting.json`, `Shipyard.json`, `ModulesInfo.json`, `NavRoute.json`, `Backpack.json`, `ShipLocker.json`, `FCMaterials.json`) tolerieren über bis zu 10 Wiederholungsversuche à 50 ms unvollständige Schreibzugriffe des Spiels.
- **Widerstandsfähigkeit und Selbstreparatur:** Beschädigte JSON-Zeilen oder unbekannte künftige Journal-Events bringen die Anwendung niemals zum Absturz. Ist `tmc.db` beim Start fehlerhaft, wird sie als `tmc.db.corrupt-YYYYMMDDHHMMSS` gesichert, ein sauberes Schema aufgesetzt und der Verlauf neu importiert. Eine ungültige `config.toml` wird in `config.toml.bak` gesichert und auf sichere Standardwerte zurückgesetzt.
- **Lokale Serversicherheit:** Bindet standardmäßig ausschließlich an Loopback `127.0.0.1:47320` (mit Portsuche in `47320..=47340` bei Belegung) und validiert `Host`- sowie `Origin`-Header. Bei aktiviertem LAN-Modus `lan = true` (`0.0.0.0`) erfordert jede externe Anfrage das im QR-Code übertragene zufällige 128-Bit-Sitzungstoken.
- **Striktes Ressourcenbudget (Harte Anforderung):** `< 150 MB` Gesamtarbeitsspeicher bei geöffnetem Fenster (~35–112 MB in der Praxis), `< 1 %` CPU im Leerlauf, keine kontinuierlichen `requestAnimationFrame`-Schleifen und maximal 10 UI-Aktualisierungen pro Sekunde über WebSocket. Beim Minimieren in den Infobereich unter Windows ruft TMC `COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW` und `SetIsVisible(false)` auf.

---

## 9. Lokale Pfade, CLI-Argumente und Umgebungsvariablen

### Standard-Pfade
- **Journale des Spiels:**
  - Windows: `%USERPROFILE%\Saved Games\Frontier Developments\Elite Dangerous\` (und eigene HUD-Farben unter `%LOCALAPPDATA%\Frontier Developments\Elite Dangerous\Options\Graphics\GraphicsConfigurationOverride.xml`).
  - Linux / SteamOS: Proton-Präfixe für AppID `359320` unter `~/.steam/steam/...`, `~/.local/share/Steam/...` oder `~/.var/app/com.valvesoftware.Steam/...`.
- **Konfiguration:** `%APPDATA%\TMC\config.toml` (Windows) / `~/.config/tmc/config.toml` (Linux).
- **SQLite-Datenbank und rotierende Protokolle (max. 7 Tage):** `%LOCALAPPDATA%\TMC\tmc.db` und `tmc*.log` (Windows) / `~/.local/share/tmc/tmc.db` (Linux).

### Befehlszeilenargumente (`tmc-desktop` und `tmc-headless`)
- `--journal-dir <pfad>`: Erzwingt das Einlesen der Journale aus einem bestimmten Verzeichnis.
- `--config <pfad>`: Verwendet eine alternative `config.toml`-Datei.
- `--ui-dir <pfad>`: Liefert die kompilierten Web-UI-Dateien aus einem benutzerdefinierten Pfad aus.

### Optionale Umgebungsvariablen
- `TMC_CAPI_CLIENT_ID`: Auf dem Frontier-Entwicklerportal registrierte OAuth2-Client-ID zum Überschreiben lokaler CAPI-Einstellungen (niemals Geheimnisse in Git committen; lokale Datei `.env` nutzen).
- `VOICEATTACK_PATH` / `VOICEATTACK_DIR`: Benutzerdefinierter Installationspfad für *VoiceAttack*.

---

## 10. Entwickler-Werkzeuge, Replay und Wissenschaftliches Audit

```powershell
# 1. Server + Kern ohne grafische Oberfläche ausführen (ideal für Remote-Linux oder Frontend-Dev)
cargo run -p tmc-headless -- --journal-dir tests/fixtures/journal

# 2. Test-Journal im Zeitraffer (10x) in ein temporäres Verzeichnis abspielen
cargo run -p tmc-replay -- play --from tests/fixtures/journal/basic_session.log --to $env:TEMP\tmc-sim --speed 10

# 3. Echtes Journal vor der Aufnahme als Test-Fixture anonymisieren (entfernt Freunde, Chats und ersetzt CMDR/FID)
cargo run -p tmc-replay -- anonymize --from C:\Pfad\Journal.log --to tests\fixtures\journal\neues_fixture.log

# 4. Wissenschaftlicher Evaluator auf realen Journalen ohne Ausgabe personenbezogener Daten (8 Mathe-Orakel + SQLite-Zeiten)
cargo run -p tmc-headless -- eval --timings

# 5. Tatsächlichen RAM- und Leerlauf-CPU-Verbrauch gegen das Budget messen (< 150 MB RAM, < 1 % CPU)
pwsh scripts/measure-footprint.ps1 -Exe target/release/TMC.exe -JournalDir tests/fixtures/journal
```

Das Unterkommando `tmc-headless eval` prüft reale Journale ohne Offenlegung von Personendaten auf Genauigkeit der FSD-Reichweite (`±0,05 Lj`), Universal Cartographics-Erlöse (`±5 %`), Exobiologie-Vorhersagen und `×5`-Multiplikator, Kampfbelohnungsabrechnungen, Raffinerieausbeute, Handelsmargen, Abschusszählungen in Massaker-Stapeln und Ingenieursmaterial-Buchhaltung.

---

## 11. Kompilierung, Automatisierte Tests und NSIS-Paketierung

### Voraussetzungen
- **Rust** stabil (`1.90+`, Edition 2024) mit nativer Toolchain (MSVC Build Tools unter Windows; `libwebkit2gtk-4.1-dev` unter Linux für Desktop-GUI).
- **Node.js** `22+` und `npm`.
- **Tauri CLI v2** (`cargo install tauri-cli@^2`) zur Erstellung des Installers.

### Vollständiger Build- und Testablauf

```powershell
# 1. Frontend-Abhängigkeiten installieren und statische Oberfläche kompilieren (Svelte 5 + Vite)
npm.cmd --prefix ui ci
npm.cmd --prefix ui run build

# 2. Frontend-Qualitätsprüfungen & Tests (TypeScript / svelte-check + Vitest)
npm.cmd --prefix ui run check
npm.cmd --prefix ui run test

# 3. Rust-Backend-Qualitätsprüfungen & Tests (Formatierung, clippy ohne Warnungen und > 1.400 Workspace-Tests)
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# 4. UI-Entwicklung mit Hot-Reload (Vite-Proxy -> 127.0.0.1:47320)
# Terminal 1: cargo run -p tmc-headless -- --journal-dir <pfad>
# Terminal 2: npm.cmd --prefix ui run dev

# 5. Optimierte Release-Binärdateien erstellen
cargo build -p tmc-desktop --release
cargo build -p tmc-headless --release

# 6. Mehrsprachigen Windows-NSIS-Installer paketieren
Push-Location apps/desktop
cargo tauri build
Pop-Location
# Ausgabe: target/release/bundle/nsis/TMC_0.1.0_x64-setup.exe
```

> **Wichtiger Hinweis zur Build-Hygiene:** Kompilieren Sie stets in das reguläre `target/`-Verzeichnis des Workspaces (definieren Sie keine benutzerdefinierten `CARGO_TARGET_DIR`-Variablen, um ein Vervielfachen des Festplattenspeichers zu verhindern).

---

## 12. Mitwirken, Datenschutz und Lizenzierung

- **Lizenz:** Veröffentlicht unter **[PolyForm Noncommercial 1.0.0](LICENSE)** (gestattet das Einsehen, Prüfen, Anpassen, Kompilieren und Teilen der Software für persönliche, schulische oder gemeinnützige Zwecke). Die Aufnahme von Code oder Datensätzen unter inkompatiblen Lizenzen wie GPL/AGPL ist untersagt; externe Abhängigkeiten werden lückenlos in [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) gepflegt.
- **Richtlinien für Beiträge:**
  - Keine sichtbaren Oberflächen- oder Sprachtexte dürfen fest im Programmcode hinterlegt werden: Alle Texte werden in `locales/{es,en,fr,de,pt}/*.ftl` deklariert (`es` dient als per Test geprüfter Referenzkatalog).
  - Committen Sie niemals API-Schlüssel, private Tokens oder reale Journaldateien ohne vorherige Bearbeitung durch `tmc-replay anonymize` und manuelle Sichtung.
  - Befolgen Sie strikt das Cockpit-Designsystem in [`docs/design-system-glass-hud.md`](docs/design-system-glass-hud.md).
- **Datenschutz:** TMC enthält keinerlei Telemetrie oder Tracking. Optionale Zugangsdaten verbleiben ausschließlich in Ihrer lokalen Datenbank, Frontier CAPI nutzt direktes OAuth2 PKCE gegen den Frontier Store und EDDN verwendet lokal gesalzene Pseudonyme.
- **Rechtlicher Hinweis:** *Elite Dangerous*, *Horizons* und *Odyssey* sind eingetragene Marken von Frontier Developments plc. TMC ist ein unabhängiges Community-Projekt und wird von Frontier Developments weder unterstützt noch offiziell empfohlen.
