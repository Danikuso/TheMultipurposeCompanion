# TMC — The Multipurpose Companion

<p align="center">
  <a href="README.md"><strong>English</strong></a> ·
  <a href="README.es.md"><strong>Español</strong></a> ·
  <a href="README.fr.md"><strong>Français</strong></a> ·
  <a href="README.de.md"><strong>Deutsch</strong></a> ·
  <a href="README.pt.md"><strong>Português</strong></a>
</p>

[![Version](https://img.shields.io/badge/version-1.0.0-orange.svg)](https://github.com/Danikuso/TheMultipurposeCompanion/releases)
[![Licence: PolyForm Noncommercial 1.0.0](https://img.shields.io/badge/licence-PolyForm%20Noncommercial%201.0.0-blue.svg)](LICENSE)
[![Plateforme](https://img.shields.io/badge/plateforme-Windows%2010%2F11%20%7C%20Linux%20%2F%20Steam%20Deck-green.svg)]()
[![Inara Whitelist](https://img.shields.io/badge/Inara%20InAPI%20v1-Whitelist%20Officielle-success.svg)]()

**TMC (*The Multipurpose Companion*)** est un assistant de cockpit gratuit, open source et en temps réel pour ***Elite Dangerous*** (*Live 4.0 / Odyssey / Horizons* et *Legacy 3.8*), conçu avec **Rust**, **Tauri 2** et **Svelte 5**.

Son objectif est de servir de copilote complet pour les pilotes débutants comme pour les commandants chevronnés : il réunit dans une application ultra-légère **10 dispositions de cockpit combinées**, **16 modules spécialisés**, la **planification d'itinéraires**, un **guide étape par étape pour débloquer les ingénieurs**, un **copilote vocal et requêtes au micro**, ainsi qu'un **écran tactile auxiliaire pour tablette ou smartphone**.

---

## Sommaire de ce Document

- **Ressources pour les Joueurs :** [Manuel d'Instructions et Tutoriels (PDF)](website/Manual_TMC_Comandante.pdf) · [Site Web de Présentation (`website/`)](website/index.html)
- **[Partie I — Guide du Joueur et de l'Utilisateur](#partie-i--guide-du-joueur-et-de-lutilisateur)**
  1. [Philosophie et Atouts Majeurs](#1-philosophie-et-atouts-majeurs)
  2. [Installation et Prise en Main (Windows et Linux / Steam Deck)](#2-installation-et-prise-en-main)
  3. [Fonctionnement de l'Interface (`TopBar`, `Auto-companion` et les 10 Dispositions)](#3-fonctionnement-de-linterface)
  4. [Toutes les Fonctionnalités : Les 16 Modules en Détail](#4-toutes-les-fonctionnalités--les-16-modules-en-détail)
  5. [Copilote Vocal, Requêtes Micro, VoiceAttack et Tablette / Mobile (QR)](#5-copilote-vocal-requêtes-micro-voiceattack-et-tablette--mobile-qr)
  6. [Personnalisation dans les Paramètres et Support / Retours](#6-personnalisation-dans-les-paramètres-et-support--retours)
- **[Partie II — Documentation Technique pour Développeurs](#partie-ii--documentation-technique-pour-développeurs)**
  7. [Architecture du Workspace (12 Crates + 2 Apps + Frontend)](#7-architecture-du-workspace)
  8. [Ingestion du Journal, Stockage, Sécurité et Performances](#8-ingestion-du-journal-stockage-sécurité-et-performances)
  9. [Chemins Locaux, Arguments CLI et Variables d'Environnement](#9-chemins-locaux-arguments-cli-et-variables-denvironnement)
  10. [Outils de Développement, Rejeu et Audit Scientifique](#10-outils-de-développement-rejeu-et-audit-scientifique)
  11. [Compilation, Tests Automatisés et Empaquetage NSIS](#11-compilation-tests-automatisés-et-empaquetage-nsis)
  12. [Contribution, Confidentialité et Licence](#12-contribution-confidentialité-et-licence)

---

# Partie I — Guide du Joueur et de l'Utilisateur

## 1. Philosophie et Atouts Majeurs

- **Un guide qui ne vous laisse jamais bloqué :** si vous souhaitez améliorer un module de votre vaisseau mais n'avez pas encore découvert ou débloqué l'ingénieur requis, TMC n'affiche pas d'avertissement vide : il détaille d'abord **étape par étape comment le découvrir, obtenir son invitation et quelle tâche ou don son atelier demande** (en lisant votre progression réelle dans le journal), et calcule juste en dessous les matériaux et échanges nécessaires.
- **Clarté visuelle (« TMC Glass HUD ») et couleur synchronisée avec votre vaisseau :** chaque panneau affiche en haut les informations essentielles avec jauges visuelles, feux tricolores et boutons en un clic, laissant les tableaux détaillés repliés sous **`Voir les informations complémentaires`**. De plus, le thème **`HUD automatique`** détecte les couleurs personnalisées du cockpit de votre jeu pour adopter votre propre palette visuelle.
- **Léger et silencieux en arrière-plan :** conçu pour ne consommer ni framerate ni ressources PC. En fermant la fenêtre (`✕`), TMC reste actif dans la zone de notification Windows (systray) en libérant sa mémoire graphique, tout en maintenant actifs les alertes vocales et la copie automatique des sauts.
- **100 % respectueux de vos données et de la galaxie Legacy :** fonctionne localement dès la première seconde sans aucun compte obligatoire. S'il détecte que vous jouez dans la galaxie **Legacy 3.8**, il bloque automatiquement toute transmission vers les serveurs communautaires externes (EDDN, EDSM, Inara) pour éviter de corrompre les bases en ligne.
- **5 langues intégrales :** interface, données d'ingénierie et voix disponibles en **Français**, **Anglais**, **Espagnol**, **Allemand** et **Portugais**.

---

## 2. Installation et Prise en Main

### Windows 10 / 11 (64 bits)

1. Téléchargez le programme d'installation **`TMC_0.1.0_x64-setup.exe`** depuis la section [Releases](https://github.com/Danikuso/TheMultipurposeCompanion/releases) (assistant disponible dans les 5 langues).
2. Lancez l'installateur et suivez les instructions à l'écran.

> [!NOTE]
> **Avertissement Windows Defender SmartScreen :**  
> En tant que projet gratuit et open source sans certificat d'entreprise commercial payant, Windows peut afficher l'écran bleu *« Windows a protégé votre ordinateur »*. Cliquez sur **« Informations complémentaires »** puis sur **« Exécuter quand même »**.

3. Ouvrez **TMC** avant ou après avoir démarré *Elite Dangerous*. Il détectera automatiquement votre dossier de sauvegarde, importera votre historique en arrière-plan (`Importation de l'historique : X sur Y`) et mémorisera la taille et la position de sa fenêtre.
4. **Zone de notification :** en cliquant sur `✕`, TMC se minimise dans la barre des tâches près de l'horloge. Cliquez avec le bouton gauche (ou clic droit → **`Ouvrir TMC`**) pour restaurer la fenêtre, ou choisissez **`Quitter`** pour fermer totalement l'application.

### Linux et SteamOS (Steam Deck)

- TMC détecte automatiquement le dossier des journaux d'*Elite Dangerous* dans les préfixes **Proton** de Steam (installations natives comme Flatpak, AppID `359320`).
- Vous pouvez lancer l'application de bureau (`tmc-desktop`) ou démarrer le serveur léger sans fenêtre (`tmc-headless`) pour accéder à l'interface depuis un navigateur ou une tablette connectée au même réseau Wi-Fi.

---

## 3. Fonctionnement de l'Interface

### 3.1 Barre Supérieure (`TopBar`)
Toujours ancrée en haut de l'écran :
- **État et Galaxie :** voyant lumineux de connexion en direct, badge **`LIVE 4.0`** ou **`LEGACY`**, et messages contextuels (`Jeu non démarré` ou `Galaxie Legacy : données strictement locales`).
- **6 Métriques Immédiates de Vol :** **`Système`** actuel, **`MODE`** (parmi 10 états : *Amarré, Atterri, Espace normal, Supernavigation, Hyperespace, À pied, En VCS, En chasseur, En taxi, Équipage multiple*), **`Carburant`** (avec voyant ambre en dessous de 25 %), **`Coque`** (avec alerte rouge en dessous de 30 %), **`Cargaison`** (`utilisée / capacité`), et **`Crédits`**.
- **Commandes Rapides à Droite :** bouton de verrouillage tactique **`🔓` / `🔒`** (avec raccourci **`Alt + L`** pour figer l'écran immédiatement), bouton de micro **`🎙️`** et commutateur **`Auto-companion`**.

### 3.2 Auto-Companion Pro : Intelligence Prédictive, Verrouillage Tactique et Priorités
- **Commutation Intelligente et Continue :** avec **`Auto-companion` activé**, TMC bascule automatiquement vers la vue de cockpit la plus pertinente selon ce qui se passe en jeu (combat, prospection d'astéroïdes, amarrage en station, ingénieurs, saut de Fleet Carrier, colonisation ou navigation hyperspatiale).
- **Verrouillage Tactique en 1 Clic (`Alt + L` / Cadenas) :** lors d'un combat intense, de l'étude d'un itinéraire ou de la consultation d'une recette, cliquez sur le cadenas dans la barre supérieure ou appuyez sur **`Alt + L`** pour figer l'écran. Le cadenas s'illumine en ambre *Glass HUD*, empêchant tout changement automatique de vue jusqu'à ce que vous le déverrouilliez.
- **Hystérèse et Rétention Anti-Saccade (`linger_seconds`) :** à l'issue d'un combat, d'une prospection minière ou d'une vente en station, l'écran ne saute pas brutalement. Il conserve le cockpit actif pendant la durée configurée (par défaut 20 secondes, réglable de 0 à 60 s dans les Paramètres) pour vous laisser le temps de consulter dégâts, récompenses et statistiques.
- **Transitions Prédictives en Vol :** TMC anticipe les événements avant même qu'ils ne surviennent :
  - *Interdiction imminente :* lors d'une tentative d'interdiction subie ou initiée en supernavigation, la vue bascule instantanément vers *Combat* pendant la lutte vectorielle.
  - *Approche d'anneaux :* en descendant vers les anneaux ou ceintures d'une planète avec de l'équipement minier à bord, la vue passe proactivement sur *Minage*.
  - *Autorisation d'amarrage accordée :* dès l'obtention de l'autorisation d'atterrissage dans une station ou un avant-poste, la vue bascule sur *Commerce* en préparant le marché local et les services.
- **Ordre de Priorités sur Mesure :** dans les `Paramètres`, personnalisez l'ordre de priorité des 9 rôles grâce à de simples flèches `▲` et `▼`, ou cliquez sur `Rétablir l'ordre standard` pour réinitialiser la hiérarchie recommandée.
- **Retour Sensoriel de Cockpit :** activez le doux carillon acoustique (*Glass HUD Chime*, synthétisé nativement via Web Audio API de 880 Hz à 1320 Hz sans aucun fichier lourd) ou les annonces vocales TTS du copilote pour être informé de chaque changement sans quitter vos instruments des yeux.
- Dans la vue **`Exploration`**, le **`Panneau de contexte`** alterne automatiquement entre **`Corps du système`** et **`Exobiologie`** dès que vous atterrissez sur un monde présentant des signaux biologiques.
- Si vous cliquez manuellement sur une autre vue alors qu'`Auto-companion` est actif (sans le cadenas enclenché), TMC respecte votre choix temporaire jusqu'au prochain changement d'activité en jeu. Désactiver `Auto-companion` maintient l'écran fixe là où vous l'avez laissé.

### 3.3 Les 10 Dispositions Combinées vs. Les 16 Modules
Le menu latéral gauche propose deux modes de navigation (en plus du bouton **`⚙️ Paramètres`** épinglé en bas) :

| Disposition combinée | Panneaux regroupés sur un seul écran |
|---|---|
| **1. `Exploration`** | Bandeau d'`Itinéraire`, `Panneau de contexte` (sélecteur `Corps` / `Exobiologie`), `Planificateur de route` et `Portée de saut`. |
| **2. `Vaisseau`** | `Vaisseau` (équipement et ingénierie), `Portée de saut`, `Cargaison` et `Chantiers et Équipements`. |
| **3. `Combat`** | `Cible`, `Session de chasse`, `Mon vaisseau`, et panneaux automatiques de `Zone de conflit` et `Anti-Xeno (AX)` actifs. |
| **4. `Minage`** | `Objectifs de minage` (boucle triangulaire et meilleure vente), `Prospecteur`, `Session de minage` et `Où vendre`. |
| **5. `Commerce`** | `Marché actuel`, `Cargaison`, `Session de commerce`, `Route commerciale` et `Rechercher une marchandise`. |
| **6. `Missions`** | `Missions actives`, `Avis`, `Récompenses de session`, et panneaux automatiques d'`Empilement de massacres`, `Passagers`, `Tourisme` et `Objectifs communautaires`. |
| **7. `Ingénierie`** | `Matériaux`, `Ingénieurs`, `Avis d'ingénierie`, `Vaisseau modifié`, `Ingénieriser un module`, `Liste de courses`, `Échanges` et `Où trouver`. |
| **8. `Fleet Carrier`** | `État du Fleet Carrier`, `Séquence de saut` et `Finances et entretien`. |
| **9. `Colonisation`** | `Dépôt de construction` et `Approvisionnement et achats`. |
| **10. `Powerplay 2.0`** | `État Powerplay` et `Factions du système` (BGS). |

Sous les dispositions, vous trouverez la liste des **16 Modules individuels** (avec un point lumineux indiquant ceux qui ont une activité en cours) pour ouvrir chaque outil en plein écran.

---

## 4. Toutes les Fonctionnalités : Les 16 Modules en Détail

### 1. `Tableau de bord` (7 panneaux)
- **`Commandant` :** nom, solde de crédits et barres de progression avec pourcentage dans les 6 rangs de la *Fédération des Pilotes* (*Combat, Commerce, Exploration, Soldat, Exobiologiste, CQC*) et les 2 des *Superpuissances* (*Fédération, Empire*).
- **`Vaisseau & Localisation` :** récapitulatif du vaisseau actif, système stellaire, corps céleste le plus proche, station et mode actuel.
- **`État en direct` :** 7 voyants d'état instantanés (*Boucliers, Points d'emport, Train d'atterrissage, Verrouillage de masse, Navigation silencieuse, Récupérateur de cargaison, Surcharge SCO*) et répartition de l'énergie `SYS`, `ENG` et `WEP` (0 à 4 pips).
- **`Itinéraire & Session` :** prochain système avec bouton **`Copier le système suivant`**, sauts restants, durée de session, sauts effectués, années-lumière parcourues et crédits gagnés.
- **`Alertes` :** avertissements immédiats (*Carburant bas, Surchauffe, Interdiction en cours, Attaque ennemie, Dégâts de coque, Oxygène faible, Santé basse* ou panne de module).

### 2. `Vaisseau` (2 panneaux : Audit technique et recherche de Vaisseaux/Équipements)
- **En-tête & Jauges (`Vaisseau`) :** modèle canonique, nom, immatriculation, masse à vide, coût d'assurance (*rebuy*) et jauges de coque, boucliers, réservoirs principal + réserve et cargaison.
- **3 Onglets de vérification avant décollage :**
  - **`🧭 Exploration` :** portée maximale et liste de contrôle (`✅` / `⚪`) pour récupérateur de carburant, booster FSD Guardian, scanner de surface détaillée DSS, baie de VRS et unité de maintenance AFMU.
  - **`⚔️ Combat` :** boucliers, chaleur, répartition `SYS/ENG/WEP`, armes installées avec munitions `[chargeur / réserve]` et utilitaires de défense (*survolteurs de bouclier, paillettes, défenses ponctuelles et dissipateurs thermiques*).
  - **`📦 Fret & Minage` :** capacité totale, soutes installées, raffinerie, contrôleurs de drones et marchandises à bord.
- **`Détail module par module` :** filtrable par catégorie (*Tous, Points d'emport, Primaires, Optionnels, Utilitaires*) et par texte. Affiche classe & note `[5A]`, intégrité `%`, alimentation `ON/OFF`, priorité `P1..P5` et badge doré `G1..G5 ★` qui, au clic, déroule les détails de recette, l'ingénieur, l'effet expérimental et la **comparaison exacte de chaque attribut (valeur modifiée vs. valeur d'origine)**.
- **`Chantiers et Équipements` (Recherche d'achat en guichet unique) :**
  - **Panier combiné depuis votre position :** part automatiquement de votre système actuel (même dans l'espace profond non indexé via coordonnées 3D) et permet de sélectionner **1 vaisseau optionnel** (48 modèles) et/ou **jusqu'à 10 modules** (153 familles avec filtres de catégorie, classe `0..8`, note `A..I`, montage d'arme *Fixe/Cardan/Tourelle* et type de blindage) pour trouver les stations les plus proches vendant **l'intégralité du panier en une seule escale**.
  - **Secours intelligent par article :** si aucune station ne réunit tout le panier simultanément, l'outil localise automatiquement la station la plus proche pour chaque article manquant et identifie les pièces sans stock connu.
  - **Filtres et sécurités pour le pilote :** interrupteur **`−15 % Li Yong-Rui`** (qui réduit également votre coût d'assurance futur), interrupteur dédié **`Transporteurs Bastion de votre puissance`** pour inclure les *Stronghold Carriers* de votre faction Powerplay 2.0 (en excluant les transporteurs rivaux et privés sauf activation explicite), protection des systèmes sous permis (`Shinrarta Dezhra`, `Sol`, `Achenar`, `Sirius`, `Alioth`…), **feu tricolore d'assurance** (`Assurance OK` / `Ajuster l'assurance` / `Solde insuffisant`), détail des prix par article et boutons en un clic **`Copier`** et **`Itinéraire`**.

### 3. `Navigation` (3 panneaux et double planificateur Spansh)
- **Bandeau `Itinéraire` :** affiche votre plan Spansh actif ou la route tracée dans la carte de la galaxie du jeu. En cas d'écart, l'alerte **`Hors itinéraire`** propose les boutons **`Recalculer`** et **`Effacer le plan`**. Détaille chaque saut en 4 colonnes : navigation (distance, classe d'étoile, statut de ravitaillement **`⛽ Ravitaillement possible`**, carburant restant estimé), exploration (découvert/inconnu, visites antérieures, naine blanche/étoile à neutrons), politique (allégeance, gouvernement, faction et **`⚠ État de danger`** en cas de guerre ou épidémie), et économie/sécurité/Powerplay.
- **`Portée de saut` :** portée actuelle (avec badge **`Surchargé ×N`** par jet de neutron ou injection), portée plein fait, niveau de carburant et consommation maximale par saut.
- **`Planificateur de route` :** saisie semi-automatique des systèmes, départ automatique par coordonnées 3D y compris depuis des systèmes non répertoriés, et 2 moteurs de calcul :
  - **`⚡ Autoroute à neutrons` :** calcul rapide via étoiles à neutrons avec pourcentage d'**`Efficacité (%)`** réglable.
  - **`🗺️ Itinéraire exact (Exact Plotter)` :** lit automatiquement la masse, les réservoirs, le chargement, le FSD modifié et le booster Guardian de votre vaisseau sans téléverser vos données, avec options **`Étoiles ravitaillables uniquement`** et **`Utiliser injections FSD`**. En cliquant sur **`Accepter`**, il sauvegarde l'itinéraire et copie automatiquement chaque étape dans le presse-papiers saut après saut.

### 4. `Exploration` (`Corps du système`)
- **Synthèse supérieure :** statut découvert/inconnu du système, toutes les classes stellaires présentes (mise en avant des étoiles ravitaillables *O, B, A, F, G, K, M*), progression de **`Découverte FSS`**, cartes comparatives **`Valeur du système`** vs **`Valeur avec cartographies recommandées`**, corps à forte valeur non cartographiés et carte **`Données non vendues`** avec alerte rouge **`Données en danger`** en cas de dommages sur votre coque.
- **Tableau détaillé extensible :** 27 classifications planétaires et stellaires, distance en secondes-lumière, valeur de scan et de cartographie, signaux `Bio` et `Géo`, balises (*Terraformable, Atterrissable, Récoltable, 1re découverte, Déjà cartographié*), et colonne **`Conseil`** (`★ Cartographier` selon votre seuil de crédits, `Atterrir (exobiologie)` ou `Négligeable`, avec l'objectif de sondes DSS).

### 5. `Exobiologie` (Assistant d'échantillonnage de surface Odyssey)
- **En-tête planétaire :** gravité de surface, température, composition atmosphérique, nombre de signaux biologiques et badge **`NON RÉPERTORIÉ · ×5 (estimé)`** lorsqu'aucun commandant n'a encore enregistré la planète.
- **Suivi d'`Échantillonnage en cours` (`1/3 → 2/3 → 3/3`) :** barre et compteur en direct de **`Distance au spécimen précédent`** comparée à la distance minimale clonale requise (`Il vous manque X m`), **guidage directionnel relatif et cap cardinal** selon votre orientation (ex. *« le spécimen précédent est derrière sur votre gauche (SO) »*), alerte en cas d'abandon de spécimen incomplet et carte de valeur **`Non vendu à bord`** avec projection du gain final.
- **Tableau `Espèces potentielles (triées par valeur)` :** combine conditions astrophysiques, votre historique et données de *Canonn* pour afficher récompense de base, paiement `Avec bonus ×5`, distance minimale, probabilité et signaux de surface confirmés par les sondes DSS.

### 6. `Combat` (5 panneaux tactiques et Anti-Xeno)
- **`Cible` :** vaisseau sélectionné avec blason, nom du pilote, rang de combat, allégeance, bandeau haute visibilité **`PROPRE · NE PAS TIRER`** (vert) ou **`RECHERCHÉ · prime`** (rouge), barres de boucliers et de coque, et santé en pourcentage du **`Sous-système`** visé (*Générateur, Propulseurs, FSD, Armes…*), conservant les données 10 secondes après rupture de verrouillage.
- **`Mon vaisseau` :** état des boucliers, jauge de coque en 10 blocs, température avec compte à rebours en surchauffe, jauges de pips (`SYS`, `MOT`, `ARM`) et alertes actives de cabine.
- **`Session de chasse` :** victoires, crédits gagnés, taux horaire (`cr/h`), progression du rang de combat, bouton **`Réinitialiser la session`**, décompte des **`Primes non réclamées`** par faction (avec calcul net à **`−25 % chez l'Interstellar Factors`**) et localisation du contact le plus proche.
- **`Zone de conflit & Anti-Xeno (AX)` (Automatique) :** faction choisie, victoires et obligations de combat ; alerte de présence Tharguide, obligations AX et compteur de victoires par variante (*Éclaireur, Cyclops, Basilisk, Medusa, Hydra, Scythe, Glaive, Orthrus…*).

### 7. `Minage` (4 panneaux : Objectifs, Prospecteur, Session et Où vendre)
- **`Objectifs de minage et vente optimale (Miner's Companion)` :**
  - **Boucle triangulaire la plus courte :** calcule dans le cockpit le circuit optimal complet : depuis votre système actuel ➔ vers le meilleur gisement ou anneau ➔ vers la station la plus proche offrant le prix de rachat le plus élevé pour vos minerais.
  - **3 Profils de minage spécialisés & Presets instantanés :**
    - **Laser :** axé sur le Platine en anneaux métalliques, Painite et Osmium. Inclut les combinaisons avec sites d'extraction *Hazardous RES* (+100 % de fragments par roche) et *High RES* (+75 % de fragments) ainsi que les chevauchements vérifiés (*overlaps*).
    - **Cœur profond (*Core*) :** axé sur Musgravite, Monazite, Alexandrite, Opales du vide et Grandidiérite dans les anneaux rocheux et glacés.
    - **Missions & Ingénieurs :** Bromellite (pour débloquer l'ingénieur Bill Turner dans Alioth), Tritium (pour ravitailler les *Fleet Carriers*) et Osmium.
  - **Alerte de risque de taxe de demande (*Demand Tax Risk*) :** le marché d'*Elite Dangerous* réduit drastiquement les prix si la cargaison dépasse 25 % de la demande totale de la station. TMC vous avertit préventivement avec un feu tricolore ambre/rouge et indique le tonnage maximal de sécurité.
  - **Optimiseur Multi-Vente en 1 clic :** analyse les minerais réels dans votre soute et trouve la station optimale achetant l'ensemble de la cargaison avec le profit global le plus élevé.
  - **Catalogue hors ligne et synchronisation atomique en ligne :** intègre plus de 40 gisements de haute densité vérifiés par la communauté (*EDTools/Spansh*), fonctionnant immédiatement hors ligne et se mettant à jour en arrière-plan avec une connexion active.
  - **Actions rapides :** boutons **`📋 Copier le système`** et **`🚀 Tracer l'itinéraire`** directs sur chaque carte et dans le tableau adaptatif avec barre de défilement horizontal fixe.
- **`Prospecteur` :** niveau de teneur (*Faible/Moyen/Élevé*), bandeau mis en avant **`◆ Cœur : Minerai`** pour détonation en profondeur, barre multicolore de composition, liste de minerais avec favoris `★`, verdict automatique **`✔ Rentable`** ou **`✘ Négligeable`** selon votre seuil de crédits et graphique comparatif des **`8 derniers astéroïdes`**.
- **`Session de minage` :** durée, tonnage raffiné et débit en `t/h`, gains estimés et `cr/h`, drones **`Limpets`** restants et consommés, remplissage de soute, répartition par minerai et état en temps réel des trémies de la **`Raffinerie`**.
- **`Où vendre` :** meilleure station recommandée pour l'ensemble de votre soute, et options déroulantes par minerai avec station, système (avec bouton copie `⎘`), distance, prix de rachat, demande, taille de plateforme (`S/M/L`) et fraîcheur des données.

### 8. `Commerce` (5 panneaux)
- **`Marché actuel` :** met en valeur les meilleures opportunités des dernières 48 h et affiche le catalogue complet en signalant les marchandises **`RARE`** (avec distance optimale de vente à 160 al), prix d'achat, approvisionnement, meilleur acheteur proche et profit par tonne.
- **`Ma cargaison` :** occupation de la soute et suivi lot par lot du **`Prix moyen payé`**, étiquettes de *cargaison de mission* ou *volée*, comparaison entre vendre à quai ou sur le meilleur marché voisin et conseil direct **`VENDRE ICI`** ou **`TRANSPORTER`**.
- **`Route commerciale` :** optimiseur multi-étapes depuis votre station actuelle, détaillant quoi acheter et vendre à chaque escale, nombre de sauts, profit total et copie automatique de l'étape suivante dans le presse-papiers.
- **`Session de commerce & Recherche de marchandise` :** bénéfice net, `cr/h`, tonnage transporté, meilleure opération, progression du rang commercial et moteur de recherche avec autocomplétion pour trouver où acheter n'importe quel produit à proximité.

### 9. `Missions` (7 panneaux : Contrats, Massacres, Passagers, Tourisme et CG)
- **`Missions actives & Avis` :** contrats regroupés par destination avec mention `▶ Prochaine étape` et bouton **`Ajouter à l'itinéraire`**, type, faction, progression, compte à rebours d'expiration (en rouge si `< 1 h`) et alertes en cas de déviation ou marchandise manquante en soute.
- **`Empilement de massacres` :** regroupe les contrats de chasse contre la même faction pirate en barres parallèles par faction émettrice (chaque élimination comptant simultanément dans toutes les factions distinctes), calculant les cibles restantes et le paiement total.
- **`Passagers` :** occupation des cabines par classe (*Économique, Affaires, Première, Luxe*), mentions **`VIP`** / **`RECHERCHÉ`** et avertissement critique **`⚠ Passager recherché à bord : risque lors des scans de sécurité`**.
- **`Tourisme` :** réorganise automatiquement les balises touristiques dans l'**`Itinéraire optimal`** le plus court par rapport à l'ordre du tableau de mission (affichant années-lumière et sauts économisés), calcule `cr/al` et `cr/h` et rend un verdict **`✔ RENTABLE`** ou **`✖ NON VALABLE`**.
- **`Récompenses de session & Objectifs communautaires` :** synthèse des crédits, matériaux reçus et réputation/influence gagnée par faction, plus suivi en direct des *Community Goals* (*contribution, palier, rang en percentile, temps restant et récompense prévue*).

### 10. `Ingénierie` (8 panneaux pour modules de vaisseau et combinaisons/armes Odyssey)
- **`Matériaux` :** 4 onglets (*Matières premières, Manufacturés, Données encodées*, et *Casier* Odyssey scindé en *Objets, Composants, Consommables et Données*), recherche instantanée, **`Historique de session`** traçant l'origine (*Mission, Scan, Minage, Échange, Ingénierie…*) et alertes **`CAPACITÉ MAX`** sur les réserves pleines.
- **`Ingénieurs & Mon vaisseau modifié` :** catalogue filtrable (*Tous, Débloqués, En attente*) des **ingénieurs de vaisseau et ingénieurs à pied Odyssey** avec rang `★`, distance et progression du prérequis ; sélecteur rapide de tout module modifiable installé à bord.
- **`Ingénieriser un module` :** choisissez l'amélioration, l'effet expérimental, le rang `G1..G5` et les essais par rang. Si vous n'avez pas encore accès à un ingénieur pour ce rang, l'outil déploie le **`GUIDE COMPANION — DÉBLOCAGE DE L'INGÉNIEUR`** avec les 3 étapes (*1. Découverte → 2. Invitation → 3. Don de déblocage*), système avec copie rapide et progression actuelle, suivi du coût exact et du feu tricolore des matériaux (`OK`, **`Couvert par échange`** ou `Manquant`).
- **`Liste de courses, Échanges, Où trouver et Avis` :** enregistrez plusieurs projets, ordonnez-les par priorité (**`▲ Monter` / `▼ Descendre`**) pour réserver vos stocks, calculez les taux de conversion exacts avec vos excédents (`Donner X → Recevoir Y`) et localisez le **Commerçant de matériaux** ou le **Courtier en technologies** (*Humain* et *Guardian*) le plus proche avec guides de collecte.

### 11. `Fleet Carrier` (3 panneaux)
- **`État du Fleet Carrier` :** indicatif, nom, badge `À BORD`, système actuel, sauts réalisables (500 al/saut), autorisations d'amarrage, niveau de carburant **`Tritium`** combinant le réservoir (`1 000 t`) + réserves en soute, et répartition de la capacité libre/occupée.
- **`Séquence de saut` :** statut opérationnel et minuteur `MM:SS` en direct pour le compte à rebours de saut (15 min) et le refroidissement (5 min), avec système de destination et bouton **`Copier la destination`**.
- **`Finances et entretien` :** indicateur financier (*Sain, Avertissement, Critique*), **`Semaines couvertes`**, solde du transporteur, entretien hebdomadaire, réserve et date du prochain prélèvement.

### 12. `Colonisation` (2 panneaux)
- **`Dépôt de construction` :** pourcentage d'avancement global du chantier, tonnes livrées vs restant à fournir, calcul dynamique du **`Nombre de trajets estimés`** selon la soute de votre vaisseau actif, et tableau détaillé ressource par ressource.
- **`Approvisionnement et achats` :** fiches de matériaux en attente classées par urgence avec tonnages, trajets restants et bouton **`Copier`** pour s'approvisionner rapidement.

### 13. `Powerplay 2.0` et 14. `BGS`
- **`État Powerplay` :** blason de puissance et superpuissance, semaines d'allégeance, rang actuel (`X / 100`), mérites accumulés et requis pour le rang suivant, mérites de session, **`Taux horaire de mérites`** et décompte avant le tick hebdomadaire (*Jeudi 07:00 UTC*).
- **`Factions du système (BGS)` :** faction dirigeante, états en cours, suivi du **`Tick galactique`** quotidien (`Il y a X h`, indiquant la fraîcheur des données) et tableau de toutes les factions locales avec jauges d'influence, allégeance et gouvernement.

### 15. `Crime & Notoriété` (5 fiches tactiques)
- **`Statut légal et Notoriété` :** jauge de 0 à 10 avec compte à rebours de dissipation (2 heures de vol actif par point) qui détecte et affiche **`⏸️ Décroissance en pause (amarré en station)`** lorsque vous êtes au port.
- **`Guide pour se blanchir` :** signale si les contacts neutres sont verrouillés par la notoriété ou, à 0, affiche directement l'**`Interstellar Factors le plus proche`** avec distance, plateforme et bouton **`Copier le système`**.
- **`Contrebande et Marchés noirs` :** liste les marchandises illégales en soute, localise le **`Marché noir le plus proche`** et fournit un guide en 3 étapes d'**`Infiltration furtive`** (alignement à 8–10 km, navigation silencieuse / dissipateur à `< 4 km` et entrée rapide).
- **`Détail des amendes & Sécurité` :** amendes et primes ventilées par juridiction + niveau de sécurité du système avec délais d'intervention de la police.

### 16. `Comptes` (Synchronisation communautaire optionnelle)
- **`Elite Dangerous Data Network (EDDN)` :** partagez de façon anonyme prix de marché, équipements et sauts stellaires avec le réseau communautaire, et recevez les actualisations des marchés locaux dans votre rayon configuré.
- **`Elite Dangerous Star Map (EDSM)` :** enregistrez votre clé API pour synchroniser carnet de vol, rangs et crédits.
- **`Inara` :** application officiellement approuvée sur la **Whitelist d'Inara** sous le nom **`The Multipurpose Companion (TMC)`**, avec regroupement par lots intelligent (max 2 requêtes toutes les 60 s) et bouton de test de connexion.
- **`Frontier Companion API (CAPI)` :** connexion sécurisée par OAuth2 sur le portail officiel de Frontier et explorateur interactif de votre **`Flotte stockée à distance`** avec tous vos vaisseaux stationnés ailleurs, leur emplacement, valeur et distance en années-lumière.

---

## 5. Copilote Vocal, Requêtes Micro, VoiceAttack et Tablette / Mobile (QR)

### 5.1 Copilote Vocal (Alertes parlées / TTS)
Dans `Paramètres` → **Copilote vocal et alertes de cockpit**, choisissez entre :
- **`Natif du système d'exploitation (Ultra-léger)` :** utilise instantanément les voix installées sur votre système sans aucun téléchargement additionnel.
- **`Moteur neuronal KokoroTTS (Voix IA cinématographique)` :** 10 voix neuronales locales de haute fidélité dans plusieurs langues avec mise en veille automatique au repos pour économiser la RAM (et bascule automatique sur la voix du système si non disponible).
- Comprend les réglages de **volume**, **débit de parole**, le bouton **`🔊 Tester la voix du copilote`** et **5 canaux indépendants** (*Combat & Alertes critiques*, *Navigation & Ravitaillement stellaire*, *Exploration & Exobiologie*, *Minage & Prospection*, et *Fleet Carrier*).

### 5.2 Assistant Vocal au Micro (9 requêtes parlées dans les 5 langues)
Dans `Paramètres` → **Assistance vocale et intégration**, activez la reconnaissance vocale via **`Whisper AI Local`** (~39 Mo, 100 % hors ligne et privé, téléchargeable en un clic depuis le panneau) ou le moteur du système, en **Écoute continue** ou **Push-to-Talk** (bouton `🎙️` de la barre supérieure). Posez des questions naturelles sur :
1. **État général du vaisseau** (coque, boucliers et carburant).
2. **Niveau de carburant** (pourcentage et tonnes).
3. **Prochain saut stellaire** (annonce votre prochaine étape).
4. **Copier le système suivant** (annonce et **copie le système dans le presse-papiers** prêt à coller dans la carte galactique).
5. **Signaux biologiques du système** (décompte des biosignaux détectés).
6. **Synthèse des matériaux épinglés** (avancement de votre liste d'ingénierie).
7. **Statut légal et amendes** (notoriété, amendes et primes).
8. **Interstellar Factors le plus proche** (annonce station, système, distance et **copie le système dans le presse-papiers**).
9. **Marché noir le plus proche** (annonce station, système, distance et **copie le système dans le presse-papiers**).

### 5.3 Intégration `VoiceAttack`
TMC détecte automatiquement si *VoiceAttack* est en cours d'exécution sur votre machine, génère le profil préconfiguré **`TMC-Copiloto.vap`** et fournit un bouton **`📥 Télécharger le profil (.vap)`** dans les Paramètres prêt à être importé en un clic.

### 5.4 Écran Tactile Auxiliaire sur Tablette ou Mobile (`Code QR`)
Si vous jouez avec un seul écran ou en réalité virtuelle, ouvrez les `Paramètres`, activez l'interrupteur **`Accès réseau local`** et scannez le **`Code QR d'appairage`** affiché à l'écran avec votre appareil connecté au même Wi-Fi. Le cockpit complet s'affiche dans votre navigateur mobile sans installer d'application ; toucher « Copier le système » sur votre tablette colle directement la destination dans le presse-papiers de votre PC.

---

## 6. Personnalisation dans les Paramètres et Support / Retours

- **Langues et Thèmes :** sélection entre **5 langues** (`es`, `en`, `fr`, `de`, `pt`) et **3 thèmes visuels** (**`HUD automatique`** synchronisé avec les couleurs de votre vaisseau, **`Orange classique`** et **`Contraste élevé`**).
- **Auto-Companion Pro :** réorganisation des priorités tactiques pour les 9 rôles (`▲` / `▼`) avec bouton de réinitialisation, curseur d'hystérèse (0 à 60 s avec valeur en direct), interrupteur de prédiction et retours sensoriels (carillon *Glass HUD Chime* et voix TTS).
- **Préférences opérationnelles :** copie automatique du prochain saut, interrupteur général d'`Auto-companion`, accès réseau local LAN, bascule de confidentialité **`Consulter les services en ligne (Spansh, EDSM, Canonn)`** (désactivez pour un usage 100 % hors ligne), **`Seuil de cartographie DSS (cr)`** et **`Efficacité par défaut du planificateur (%)`**.
- **Économie, Tourisme et Ingénierie :** liste personnalisable de 30 **`Minerais d'intérêt`** (11 pré-cochés), **`Seuil de valeur de minage (cr)`**, **`Rayon de recherche de vente commerciale (al)`**, **`Rayon des données de marché EDDN (al)`**, **`Référence touristique (cr/al)`** et **`Essais d'ingénierie par rang (G1–G5)`**.
- **Canal direct de Support et Communauté :** dans `Paramètres` → **Support, Feedback et Communauté**, cliquez sur **`🚀 Signaler un problème ou une suggestion sur GitHub`** pour ouvrir un ticket pré-rempli sur [GitHub Issues](https://github.com/Danikuso/TheMultipurposeCompanion/issues) avec la version, la langue et le système d'exploitation (sans aucune donnée privée du joueur).

---

# Partie II — Documentation Technique pour Développeurs

## 7. Architecture du Workspace

Le projet est conçu comme un workspace modulaire en **Rust stable (édition 2024)** aux responsabilités clairement isolées et sans dépendance cyclique, associé à un frontend moderne en **Svelte 5 + TypeScript + Vite** et aux catalogues multilingues **Fluent** :

| Crate / Dossier | Responsabilité technique |
|---|---|
| `crates/tmc-model` | Modèles de domaine typés, événements du journal, état volatile du jeu, canaux WebSocket, diagnostics stricts (`app.toggle_lock`, `app.set_auto_priorities`) et schémas de configuration (`auto_companion_priorities`, `auto_companion_linger_seconds`, etc.). |
| `crates/tmc-journal` | Découverte multiplateforme des répertoires (Win32 `FOLDERID_SavedGames`, Proton/Flatpak sur Linux), lecture incrémentale par *byte offset* de `Journal.*.log` sans verrouiller le jeu et scrutation tolérante de `Status.json` + 9 fichiers `.json` associés. |
| `crates/tmc-store` | Persistance sous **SQLite (mode WAL)** avec `rusqlite`, migrations additives, historique d'exploration/commerce/courses, coffre-fort local de clés et restauration automatique en cas de corruption. |
| `crates/tmc-i18n` | Chargement et résolution des catalogues **Project Fluent** (`locales/{es,en,fr,de,pt}/*.ftl`) avec tests assurant une parité stricte à 100 % des clés par rapport à `es`. |
| `crates/tmc-net` | Clients HTTP asynchrones avec limitation de débit, cache et secours hors ligne : *Spansh* (Neutron Plotter, Exact Plotter `/api/generic/route`, recherche de chantiers/équipements `POST /api/stations/search`, anneaux/corps `/api/bodies/search` et recherche de proximité `/api/nearest`), *Ardent*, *Canonn*, *tick.edcd.io*, *EDSM*, *Inara InAPI v1* (lots max 2 req/min), et *Frontier CAPI* (OAuth2 S256 PKCE). |
| `crates/tmc-eddn` | Émetteur HTTP POST envoyant les schémas officiels à *EDDN* (`journal/1`, `commodity/3`, `outfitting/2`, `shipyard/2`) avec salage local pseudonymisé, et écouteur ZeroMQ filtrant les marchés dans votre rayon spatial. |
| `crates/tmc-tts` | File d'attente vocale prioritaire avec déduplication et délais de grâce (`TtsQueue`), moteur natif Windows (SAPI5 / OneCore `< 2 Mo` RAM) et processus travailleur neuronal découplé (`Kokoro-82M` ONNX avec mise en veille après 60 s d'inactivité). |
| `crates/tmc-modules` | Logique métier des 16 modules (incluant `outfitting_catalog.json` dans `ship` et `mining_hotspots.json` avec rechargement atomique en mémoire dans `mining`), calcul de pertinence contextuelle avec hystérèse (`linger_seconds`), déclencheurs prédictifs et projections UI. |
| `crates/tmc-core` | Acteur central d'état, routage des événements, générateur tactique `VoiceAdvisor` et parseur d'intentions multilingues `VoiceIntentResolver` (9 intentions vocales). |
| `crates/tmc-server` | Serveur HTTP/REST et WebSocket (`/ws`) basé sur **axum**, coalescence des messages toutes les 100 ms (max 10 màj/s), générateur de profil `.vap` pour *VoiceAttack*, protections DNS-rebinding/CSRF et authentification réseau local par jeton 128 bits (`HttpOnly; SameSite=Strict`). |
| `crates/tmc-app` | Composition des services, cycle de vie, importation historique avec régulation adaptative et boucle de supervision. |
| `crates/tmc-replay` | Outil CLI de rejeu accéléré de journaux (`play`) et d'anonymisation de données personnelles (`anonymize`) pour confectionner des bancs d'essai. |
| `apps/desktop` | Application native **Tauri 2** (`TMC.exe`), mutex d'instance unique, persistance de la fenêtre, intégration systray et optimisation de mémoire WebView2. |
| `apps/headless` | Serveur CLI autonome sans interface graphique et moteur d'audit mathématique (`tmc-headless eval`). |
| `ui/` | Interface réactive en **Svelte 5** (runes), TypeScript, design system *TMC Glass HUD*, verrouillage d'écran (`Alt + L`), réordonnancement de priorités, synthétiseur audio Web Audio API, listes virtuelles dynamiques et reconnaissance vocale WebAssembly locale (`Whisper` via `@huggingface/transformers`). |
| `locales/` | Catalogues de traduction `.ftl` synchronisés pour `es` (référence), `en`, `fr`, `de` et `pt`. |

---

## 8. Ingestion du Journal, Stockage, Sécurité et Performances

- **Lecture par décalage d'octets (*byte offset*) non intrusive :** les notifications de fichiers de l'OS servent uniquement à détecter la création d'un nouveau fichier `Journal.*.log` ; la lecture ligne par ligne progresse par offset sans jamais verrouiller les descripteurs d'`EliteDangerous64.exe`. Les fichiers temporaires (`Status.json`, `Cargo.json`, `Market.json`, `Outfitting.json`, `Shipyard.json`, `ModulesInfo.json`, `NavRoute.json`, `Backpack.json`, `ShipLocker.json`, `FCMaterials.json`) appliquent jusqu'à 10 tentatives espacées de 50 ms pour tolérer les écritures partielles du jeu.
- **Résilience et auto-réparation :** une ligne JSON corrompue ou un événement inconnu n'interrompt jamais l'application. Si `tmc.db` est corrompue au démarrage, elle est renommée en `tmc.db.corrupt-YYYYMMDDHHMMSS`, un schéma neuf est déployé et l'historique est réimporté. Si `config.toml` est invalide, il est conservé sous `config.toml.bak` et les valeurs sûres sont restaurées.
- **Sécurité du serveur local :** écoute par défaut exclusivement sur l'interface locale `127.0.0.1:47320` (en scannant `47320..=47340` en cas d'occupation), avec contrôle strict des en-têtes `Host` et `Origin`. En cas d'activation du mode réseau local `lan = true` (`0.0.0.0`), toute requête externe exige le jeton aléatoire de session de 128 bits du QR code.
- **Budget strict de ressources (Exigence contractuelle) :** `< 150 Mo` de RAM totale fenêtre ouverte (~35–112 Mo en pratique), `< 1 %` de CPU au repos, aucun cycle `requestAnimationFrame` continu et 10 actualisations/seconde au maximum sur WebSocket. Lors de la réduction dans la barre des tâches sous Windows, l'application appelle `COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW` et `SetIsVisible(false)`.

---

## 9. Chemins Locaux, Arguments CLI et Variables d'Environnement

### Chemins par défaut
- **Journaux du jeu :**
  - Windows : `%USERPROFILE%\Saved Games\Frontier Developments\Elite Dangerous\` (et couleurs personnalisées dans `%LOCALAPPDATA%\Frontier Developments\Elite Dangerous\Options\Graphics\GraphicsConfigurationOverride.xml`).
  - Linux / SteamOS : préfixes Proton pour l'AppID `359320` dans `~/.steam/steam/...`, `~/.local/share/Steam/...` ou `~/.var/app/com.valvesoftware.Steam/...`.
- **Configuration :** `%APPDATA%\TMC\config.toml` (Windows) / `~/.config/tmc/config.toml` (Linux).
- **Base de données SQLite et journaux rotatifs (7 jours max) :** `%LOCALAPPDATA%\TMC\tmc.db` et `tmc*.log` (Windows) / `~/.local/share/tmc/tmc.db` (Linux).

### Arguments en ligne de commande (`tmc-desktop` et `tmc-headless`)
- `--journal-dir <chemin>` : force le répertoire de lecture des journaux.
- `--config <chemin>` : utilise un fichier `config.toml` spécifique.
- `--ui-dir <chemin>` : sert les fichiers de l'interface web compilée depuis un emplacement personnalisé.

### Variables d'environnement optionnelles
- `TMC_CAPI_CLIENT_ID` : identifiant client OAuth2 enregistré sur le portail développeur de Frontier pour surcharger la configuration CAPI locale (ne jamais versionner de secret dans git ; utilisez un fichier local `.env`).
- `VOICEATTACK_PATH` / `VOICEATTACK_DIR` : répertoire d'installation personnalisé pour *VoiceAttack*.

---

## 10. Outils de Développement, Rejeu et Audit Scientifique

```powershell
# 1. Lancer le serveur et le noyau sans interface graphique (idéal pour Linux distant ou dev UI)
cargo run -p tmc-headless -- --journal-dir tests/fixtures/journal

# 2. Rejouer un journal de test en temps accéléré (10x) vers un répertoire temporaire
cargo run -p tmc-replay -- play --from tests/fixtures/journal/basic_session.log --to $env:TEMP\tmc-sim --speed 10

# 3. Anonymiser un journal réel avant intégration en banc d'essai (supprime amis, chats et remplace CMDR/FID)
cargo run -p tmc-replay -- anonymize --from C:\Chemin\Journal.log --to tests\fixtures\journal\nouveau_fixture.log

# 4. Évaluateur scientifique sur journaux réels sans afficher de données personnelles (8 oracles mathématiques + temps SQLite)
cargo run -p tmc-headless -- eval --timings

# 5. Mesurer l'empreinte mémoire RAM et CPU au repos par rapport au budget (< 150 Mo RAM, < 1 % CPU)
pwsh scripts/measure-footprint.ps1 -Exe target/release/TMC.exe -JournalDir tests/fixtures/journal
```

La sous-commande `tmc-headless eval` vérifie sur des journaux réels sans divulguer de données personnelles la précision de portée FSD (`±0,05 al`), les gains Universal Cartographics (`±5 %`), les estimations exobiologiques et le multiplicateur `×5`, la réconciliation des primes de combat, les rendements de raffinage, les marges commerciales, le suivi des cibles de massacres empilés et la comptabilité des matériaux d'ingénieurs.

---

## 11. Compilation, Tests Automatisés et Empaquetage NSIS

### Prérequis
- **Rust** stable (`1.90+`, édition 2024) avec chaîne native (MSVC Build Tools sous Windows ; `libwebkit2gtk-4.1-dev` sous Linux pour le bureau graphique).
- **Node.js** `22+` et `npm`.
- **Tauri CLI v2** (`cargo install tauri-cli@^2`) pour confectionner l'installateur.

### Déroulement complet de vérification et de build

```powershell
# 1. Installer les dépendances du frontend et compiler l'interface (Svelte 5 + Vite)
npm.cmd --prefix ui ci
npm.cmd --prefix ui run build

# 2. Vérification et tests du frontend (TypeScript / svelte-check + Vitest)
npm.cmd --prefix ui run check
npm.cmd --prefix ui run test

# 3. Vérification et tests du backend Rust (formatage, clippy sans avertissement et > 1 400 tests du workspace)
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# 4. Développement UI avec rechargement à chaud (proxy Vite -> 127.0.0.1:47320)
# Terminal 1 : cargo run -p tmc-headless -- --journal-dir <chemin>
# Terminal 2 : npm.cmd --prefix ui run dev

# 5. Compiler les binaires de production optimisés (Release)
cargo build -p tmc-desktop --release
cargo build -p tmc-headless --release

# 6. Empaqueter l'installateur multilingue NSIS pour Windows
Push-Location apps/desktop
cargo tauri build
Pop-Location
# Fichier généré : target/release/bundle/nsis/TMC_0.1.0_x64-setup.exe
```

> **Règle d'hygiène de compilation :** compilez toujours dans le dossier standard `target/` du workspace (ne définissez pas de variable `CARGO_TARGET_DIR` personnalisée pour éviter de dupliquer des dizaines de gigaoctets d'artefacts).

---

## 12. Contribution, Confidentialité et Licence

- **Licence :** distribué sous **[PolyForm Noncommercial 1.0.0](LICENSE)** (autorise la consultation, l'audit, la modification, la compilation et le partage du logiciel à des fins personnelles, éducatives ou communautaires à but non lucratif). L'incorporation de code ou de données sous licence incompatible (comme GPL/AGPL) est strictement interdite ; toutes les dépendances tierces sont répertoriées dans [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
- **Règles pour les contributions :**
  - Aucune chaîne visible dans l'interface ou prononcée à la voix ne doit être codée en dur : toutes les chaînes sont déclarées dans `locales/{es,en,fr,de,pt}/*.ftl` (`es` servant de catalogue de référence validé par `cargo test -p tmc-i18n`).
  - Ne committez jamais de clés API, de jetons ou de journaux réels sans passer préalablement par `tmc-replay anonymize` et examen manuel.
  - Respectez rigoureusement le système de conception de cockpit dans [`docs/design-system-glass-hud.md`](docs/design-system-glass-hud.md).
- **Confidentialité :** TMC n'intègre aucune télémétrie ni outil d'analyse d'audience. Les identifiants optionnels restent exclusivement dans votre base de données locale, Frontier CAPI utilise le flux direct OAuth2 PKCE auprès du magasin Frontier et les soumissions EDDN emploient un pseudonyme salé localement.
- **Mentions légales :** *Elite Dangerous*, *Horizons* et *Odyssey* sont des marques déposées de Frontier Developments plc. TMC est un projet communautaire indépendant, non affilié et non soutenu officiellement par Frontier Developments.
