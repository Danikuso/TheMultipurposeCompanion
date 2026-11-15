/**
 * TMC — The Multipurpose Companion
 * Reportador Guiado de Bugs (3 pasos dinámicos)
 * 1. Función afectada (16 módulos + 5 áreas del sistema = 21)
 * 2. Forma específica en la que falla esa función
 * 3. Pautas seguidas hasta esa función y ese fallo + entorno + log
 */
(function (root, factory) {
  const exported = factory();
  if (typeof module !== 'undefined' && module.exports) {
    module.exports = exported;
  }
  if (root) {
    root.TMCBugReporter = exported;
  }
})(typeof window !== 'undefined' ? window : globalThis, function () {
  'use strict';

  const GITHUB_REPO = 'Danikuso/TheMultipurposeCompanion';
  const GITHUB_ISSUES_NEW = `https://github.com/${GITHUB_REPO}/issues/new`;
  const MAX_URL_LENGTH = 7600;
  const MAX_LOG_UPLOAD_BYTES = 256 * 1024;
  const MAX_LOG_LINES = 80;
  const MAX_LOG_CHARS_PREVIEW = 6000;
  const MAX_LOG_CHARS_FULL = 24000;

  const REPORTER_I18N = {
    es: {
      step1Badge: 'Paso 1',
      step2Badge: 'Paso 2',
      step3Badge: 'Paso 3',
      allFilter: 'Todas',
      modulesFilter: 'Módulos',
      systemFilter: 'Sistema',
      searchPlaceholder: 'Filtrar función (ej. ingenieros, voz, diario, minería, tablet)...',
      selectPlaceholder: '-- Selecciona una función en la lista --',
      noFeatureMatch: 'Ninguna función coincide con tu búsqueda. Prueba otro término o limpia el filtro.',
      chooseFeatureFirst: 'Selecciona primero una función en el Paso 1 para ver las formas concretas en las que puede fallar.',
      chooseFailureFirst: 'Elige primero cómo está fallando la función seleccionada en el Paso 2 para cargar las pautas exactas de reproducción.',
      featureStepsHeading: 'Ruta seguida para llegar a la función',
      failureStepsHeading: 'Pautas concretas que desencadenan este fallo',
      failureDetailLabel: 'Detalle puntual adicional sobre cómo se manifiesta (opcional)',
      failureDetailPlaceholder: 'Ej. Ocurre solo al tener más de 3 ingenieros fijados o tras cambiar de nave en estación...',
      envAppNotice: 'Datos de entorno y registro precargados automáticamente desde TMC.',
      envVersionLabel: 'Versión de TMC',
      envPlatformLabel: 'Sistema / Modo de uso',
      envGameModeLabel: 'Edición de Elite Dangerous',
      envFrequencyLabel: 'Frecuencia del fallo',
      envSystemContextLabel: 'Sistema / Estación / Nave actual (opcional)',
      envSystemContextPlaceholder: 'Ej. Deciat · Farseer Inc. · Mandalay',
      extraNotesLabel: 'Notas adicionales y resultado esperado (opcional)',
      extraNotesPlaceholder: 'Describe qué esperabas que ocurriera y si encontraste alguna forma de esquivarlo...',
      customStepLabel: 'Añadir otro paso o pauta que seguiste (opcional)',
      customStepPlaceholder: 'Ej. Antes había cambiado el idioma en Ajustes sin reiniciar la vista...',
      logHeader: 'Registro reciente de la aplicación (tmc.log)',
      logLoadedFromApp: 'Cargado automáticamente desde TMC ({file}, {lines} líneas).',
      logPrivacyNote: 'El registro se procesa en tu navegador. Revisa que no aparezcan rutas o datos personales antes de publicarlo.',
      logFileBtn: '📂 Cargar archivo tmc.log',
      logClearBtn: 'Limpiar log',
      logRestoreBtn: 'Restaurar log recibido',
      logPlaceholder: 'Pega aquí las últimas líneas del archivo de registro (tmc.log) o cárgalo con el botón superior...',
      summaryFunction: 'Función afectada',
      summaryFailure: 'Forma de fallo',
      summaryGuidelines: 'Pautas marcadas',
      summaryGuidelinesNone: 'Ninguna marcada aún',
      previewHeader: 'Vista previa del reporte (Markdown)',
      btnOpenGithub: '🚀 Abrir Issue en GitHub',
      btnCopyReport: '📋 Copiar reporte completo',
      copiedToast: 'Reporte completo copiado al portapapeles.',
      truncatedToast: 'El log era demasiado largo para la URL de GitHub: se ha copiado el reporte completo al portapapeles para que puedas pegarlo con Ctrl+V.',
      selectErrorToast: 'Completa el Paso 1 (función) y el Paso 2 (forma de fallo) antes de generar el issue.',
      selectedBadge: 'SELECCIONADA',
      failureModesCount: '{count} formas de fallo',
      categoryModule: 'Módulo',
      categorySystem: 'Sistema',
      freqOptions: [
        { id: 'always', label: 'Siempre que repito estas pautas' },
        { id: 'often', label: 'A menudo (varias veces por sesión)' },
        { id: 'once', label: 'Solo una vez / puntual' },
        { id: 'startup', label: 'Al iniciar la aplicación' },
      ],
      platformOptions: [
        { id: 'windows-desktop', label: 'Windows · App de escritorio (TMC.exe)' },
        { id: 'windows-steamdeck', label: 'Steam Deck / Proton / Linux' },
        { id: 'tablet-lan', label: 'Navegador Tablet / Móvil (LAN)' },
        { id: 'other', label: 'Otro entorno' },
      ],
      gameOptions: [
        { id: 'odyssey', label: 'Elite Dangerous: Odyssey' },
        { id: 'horizons-live', label: 'Elite Dangerous: Horizons (Live)' },
        { id: 'legacy', label: 'Galaxia Legacy' },
        { id: 'no-game', label: 'Sin el juego abierto / Replay' },
      ],
      issueSections: {
        feature: 'Función afectada',
        category: 'Categoría',
        failureMode: 'Forma de fallo',
        featurePath: 'Pautas seguidas para llegar a la función',
        failurePath: 'Pautas concretas hasta el fallo',
        customStep: 'Paso adicional indicado por el piloto',
        extraNotes: 'Notas adicionales / Resultado esperado',
        environment: 'Entorno',
        version: 'Versión de TMC',
        platform: 'Plataforma / Modo',
        gameEdition: 'Edición del juego',
        frequency: 'Frecuencia',
        systemContext: 'Contexto (Sistema / Estación / Nave)',
        origin: 'Origen del reporte',
        logSection: 'Registro de aplicación',
        logNotAttached: '_No se adjuntó registro de aplicación._',
        logTruncatedNote: '_(Registro recortado para ajustarse al límite URL de GitHub; el texto completo se copió al portapapeles si usaste el asistente.)_',
        notSpecified: '_No marcado por el usuario._',
      },
    },
    en: {
      step1Badge: 'Step 1',
      step2Badge: 'Step 2',
      step3Badge: 'Step 3',
      allFilter: 'All',
      modulesFilter: 'Modules',
      systemFilter: 'System',
      searchPlaceholder: 'Filter feature (e.g. engineers, voice, journal, mining, tablet)...',
      selectPlaceholder: '-- Select a feature from the list --',
      noFeatureMatch: 'No feature matches your search. Try another term or clear the filter.',
      chooseFeatureFirst: 'First select a feature in Step 1 to display the specific ways it can fail.',
      chooseFailureFirst: 'First pick how the selected feature is failing in Step 2 to load exact reproduction steps.',
      featureStepsHeading: 'Path followed to reach the feature',
      failureStepsHeading: 'Specific steps leading to this failure mode',
      failureDetailLabel: 'Extra detail on how this failure looks (optional)',
      failureDetailPlaceholder: 'E.g. Happens only when tracking 3+ engineers or after swapping ships in a station...',
      envAppNotice: 'Environment info and recent app log were preloaded automatically from TMC.',
      envVersionLabel: 'TMC Version',
      envPlatformLabel: 'OS / Mode',
      envGameModeLabel: 'Elite Dangerous Edition',
      envFrequencyLabel: 'Frequency',
      envSystemContextLabel: 'Current System / Station / Ship (optional)',
      envSystemContextPlaceholder: 'E.g. Deciat · Farseer Inc. · Mandalay',
      extraNotesLabel: 'Additional notes & expected behavior (optional)',
      extraNotesPlaceholder: 'Describe what you expected to happen and whether any workaround worked...',
      customStepLabel: 'Add another step you followed (optional)',
      customStepPlaceholder: 'E.g. I switched UI language right before opening this panel...',
      logHeader: 'Recent application log (tmc.log)',
      logLoadedFromApp: 'Automatically loaded from TMC ({file}, {lines} lines).',
      logPrivacyNote: 'The log stays in your browser until you submit. Review it for any personal paths before posting.',
      logFileBtn: '📂 Load tmc.log file',
      logClearBtn: 'Clear log',
      logRestoreBtn: 'Restore received log',
      logPlaceholder: 'Paste the last lines of tmc.log here or load your local log file using the button above...',
      summaryFunction: 'Affected feature',
      summaryFailure: 'Failure mode',
      summaryGuidelines: 'Checked steps',
      summaryGuidelinesNone: 'None checked yet',
      previewHeader: 'Live Report Preview (Markdown)',
      btnOpenGithub: '🚀 Open GitHub Issue',
      btnCopyReport: '📋 Copy Full Report',
      copiedToast: 'Full report copied to clipboard.',
      truncatedToast: 'The log exceeded GitHub URL length: the full Markdown report was copied to your clipboard so you can paste it with Ctrl+V.',
      selectErrorToast: 'Complete Step 1 (feature) and Step 2 (failure mode) before creating the issue.',
      selectedBadge: 'SELECTED',
      failureModesCount: '{count} failure modes',
      categoryModule: 'Module',
      categorySystem: 'System',
      freqOptions: [
        { id: 'always', label: 'Every time I follow these steps' },
        { id: 'often', label: 'Frequently (multiple times per session)' },
        { id: 'once', label: 'Just once / intermittent' },
        { id: 'startup', label: 'On application startup' },
      ],
      platformOptions: [
        { id: 'windows-desktop', label: 'Windows · Desktop app (TMC.exe)' },
        { id: 'windows-steamdeck', label: 'Steam Deck / Proton / Linux' },
        { id: 'tablet-lan', label: 'Tablet / Mobile Browser (LAN)' },
        { id: 'other', label: 'Other environment' },
      ],
      gameOptions: [
        { id: 'odyssey', label: 'Elite Dangerous: Odyssey' },
        { id: 'horizons-live', label: 'Elite Dangerous: Horizons (Live)' },
        { id: 'legacy', label: 'Legacy Galaxy' },
        { id: 'no-game', label: 'Game not running / Replay' },
      ],
      issueSections: {
        feature: 'Affected Feature',
        category: 'Category',
        failureMode: 'Failure Mode',
        featurePath: 'Steps Followed to Reach the Feature',
        failurePath: 'Specific Steps Leading to Failure',
        customStep: 'Additional User Step',
        extraNotes: 'Additional Notes / Expected Result',
        environment: 'Environment',
        version: 'TMC Version',
        platform: 'Platform / Mode',
        gameEdition: 'Game Edition',
        frequency: 'Frequency',
        systemContext: 'Context (System / Station / Ship)',
        origin: 'Report Source',
        logSection: 'Application Log',
        logNotAttached: '_No application log attached._',
        logTruncatedNote: '_(Log trimmed to fit GitHub URL limit; full report was copied to clipboard automatically.)_',
        notSpecified: '_Not checked by user._',
      },
    },
  };

  /**
   * Catálogo completo de 21 funcionalidades:
   * - 16 módulos del juego
   * - 5 áreas transversales del sistema
   * Cada función contiene pasos base de entrada + formas de fallo específicas con pautas propias.
   */
  const FEATURE_CATALOG = [
    // ======================== 16 MÓDULOS DE JUEGO ========================
    {
      id: 'dashboard',
      category: 'module',
      icon: '🧭',
      title: { es: 'Panel General', en: 'General Dashboard' },
      summary: {
        es: 'Resumen del comandante, saldo, naves, testigo SCO, ruta y sesión actual.',
        en: 'Commander summary, credits, ships, SCO gauge, active route, and session telemetry.',
      },
      entrySteps: [
        { id: 'dash-nav', es: 'Abrí TMC o cambié al modo / módulo «Panel General» desde la barra lateral', en: 'Opened TMC or switched to the «General Dashboard» module from the sidebar' },
        { id: 'dash-live', es: 'Estaba en vuelo o atracado mientras el diario escribía eventos nuevos', en: 'Was flying or docked while the game journal was writing new events' },
        { id: 'dash-tab', es: 'Alterné entre vistas combinadas de cabina y volví al Panel General', en: 'Switched between combined cockpit layouts and returned to General Dashboard' },
      ],
      failureModes: [
        {
          id: 'dash-stale-telemetry',
          label: { es: 'Casco, escudo, combustible o SCO no se actualizan en tiempo real', en: 'Hull, shield, fuel, or SCO values do not update in real time' },
          desc: { es: 'Los indicadores del HUD permanecen congelados o con el valor del último arranque.', en: 'HUD gauges remain frozen or stuck at startup values.' },
          steps: [
            { id: 'dash-f1-1', es: 'Despegué, activé supercrucero o sobrecarga SCO y miré los medidores en pantalla', en: 'Took off, entered supercruise, or engaged SCO and watched the on-screen gauges' },
            { id: 'dash-f1-2', es: 'Recibí daño o consumí combustible sin que variara el porcentaje en TMC', en: 'Took hull/shield damage or burned fuel without percentage updating in TMC' },
            { id: 'dash-f1-3', es: 'Comprobé Status.json / diario y el juego seguía actualizando sus ficheros locales', en: 'Verified Status.json / journal files were actively updating on disk' },
          ],
        },
        {
          id: 'dash-credits-ranks-wrong',
          label: { es: 'Créditos, rangos o resumen de sesión muestran cifras incorrectas', en: 'Credits, ranks, or session summary show wrong numbers' },
          desc: { es: 'El saldo total, la ganancia de sesión o los 6 rangos federativos no coinciden con el juego.', en: 'Total balance, session profit, or ranks do not match the in-game panel.' },
          steps: [
            { id: 'dash-f2-1', es: 'Realicé una venta, cobro de recompensa o misión antes de revisar el resumen', en: 'Sold commodities, cashed bounties, or turned in missions before checking summary' },
            { id: 'dash-f2-2', es: 'Abrí TMC a mitad de sesión o recargué el diario antiguo tras reiniciar', en: 'Launched TMC mid-session or reloaded an older journal session' },
            { id: 'dash-f2-3', es: 'Observé cifras desfasadas frente al panel derecho de la nave', en: 'Noticed stale or negative session deltas compared to the in-game right panel' },
          ],
        },
        {
          id: 'dash-alerts-missing',
          label: { es: 'Los testigos de alerta en cabina no saltan o quedan encendidos', en: 'Cockpit warning indicators fail to trigger or stay stuck on' },
          desc: { es: 'Alertas de combustible bajo, calor crítico, masa bloqueada o interdicción fallan.', en: 'Low fuel, heat, mass-lock, or interdiction warnings fail to appear or clear.' },
          steps: [
            { id: 'dash-f3-1', es: 'Entré en una situación de riesgo (combustible < 25 %, sobrecalentamiento > 100 % o masa)', en: 'Entered a warning state (low fuel < 25%, heat > 100%, or mass-lock)' },
            { id: 'dash-f3-2', es: 'Salí de la situación de alerta (reposté o enfrié la nave) pero el testigo siguió rojo', en: 'Cleared the condition (scooped fuel or cooled down) yet the warning stayed active' },
          ],
        },
        {
          id: 'dash-route-sync',
          label: { es: 'La ruta de saltos del Panel General no coincide con la trazada en el mapa galáctico', en: 'Jump route on Dashboard does not match galaxy map plotted route' },
          desc: { es: 'Muestra saltos ya completados o no refresca al recalcular ruta en el juego.', en: 'Shows already completed jumps or fails to refresh on replotting.' },
          steps: [
            { id: 'dash-f4-1', es: 'Trazé una ruta nueva en el Mapa Galáctico del juego y salté al primer sistema', en: 'Plotted a new route in the in-game Galaxy Map and jumped to the first system' },
            { id: 'dash-f4-2', es: 'Cambié o cancelé el destino en el mapa y el Panel General conservó el destino previo', en: 'Cleared or changed the destination in-game while Dashboard kept the old target' },
          ],
        },
      ],
    },
    {
      id: 'ship',
      category: 'module',
      icon: '🚀',
      title: { es: 'Nave y Equipamiento', en: 'Ship & Outfitting' },
      summary: {
        es: 'Localizador de naves y módulos (Li Yong-Rui / Strongholds), cesta y loadout actual.',
        en: 'Nearby ship & module finder (Li Yong-Rui / Strongholds), shopping cart, and current loadout.',
      },
      entrySteps: [
        { id: 'ship-nav', es: 'Entré en el módulo «Nave y Equipamiento» con mi nave actual cargada', en: 'Opened the «Ship & Outfitting» module with my active ship loaded' },
        { id: 'ship-docked', es: 'Consulté el equipamiento actual o abrí el buscador de astilleros/módulos', en: 'Inspected current outfitting or opened the shipyard/module finder' },
      ],
      failureModes: [
        {
          id: 'ship-wrong-modules',
          label: { es: 'Los módulos, masa o alcance de salto de mi nave no coinciden con el juego', en: 'Ship modules, mass, or jump range do not match in-game outfitting' },
          desc: { es: 'Al cambiar módulos en Outfitting, TMC conserva equipamiento anterior o calcula mal los años luz.', en: 'After swapping modules in Outfitting, TMC retains old gear or miscalculates jump range.' },
          steps: [
            { id: 'ship-f1-1', es: 'Cambié de nave o instalé/quité módulos (p. ej. FSD SCO, propulsores o carga) en una estación', en: 'Swapped ships or fitted/removed modules (e.g. SCO FSD, thrusters, cargo) at a station' },
            { id: 'ship-f1-2', es: 'Modifiqué el módulo con ingeniería o añadí carga y el alcance mostró un valor desfasado', en: 'Engineered a module or loaded cargo and saw an outdated jump range' },
          ],
        },
        {
          id: 'ship-finder-no-results',
          label: { es: 'El buscador de naves o módulos no devuelve resultados o indica estaciones sin stock', en: 'Ship/module locator returns no results or outdated out-of-stock stations' },
          desc: { es: 'Estaciones con descuento de Li Yong-Rui o Stronghold Carriers no aparecen o ya no tienen la pieza.', en: 'Li Yong-Rui discount or Stronghold stations fail to match or lack stock.' },
          steps: [
            { id: 'ship-f2-1', es: 'Busqué una nave o módulo concreto (p. ej. clase/tamaño exacto) con filtro de descuento', en: 'Searched for a specific ship or module class/rating with discount filter active' },
            { id: 'ship-f2-2', es: 'Copié el sistema recomendado y al llegar al juego la estación no tenía esa nave/módulo', en: 'Copied the suggested station system and flew there only to find it unavailable' },
          ],
        },
        {
          id: 'ship-checklist-cart',
          label: { es: 'La cesta de compra o la lista previa al despegue no guarda ni marca cambios', en: 'Module shopping cart or pre-flight checklist fails to save or toggle' },
          desc: { es: 'Los elementos añadidos desaparecen al cambiar de módulo o reiniciar.', en: 'Added items disappear when switching views or restarting.' },
          steps: [
            { id: 'ship-f3-1', es: 'Añadí varios módulos a la cesta de compra o marqué ítems en la lista de comprobación', en: 'Added modules to the shopping cart or checked pre-flight checklist items' },
            { id: 'ship-f3-2', es: 'Cambié a otro módulo del menú lateral o cerré y reabrí TMC', en: 'Navigated to another sidebar module or closed and reopened TMC' },
          ],
        },
        {
          id: 'ship-coriolis-export',
          label: { es: 'Error al copiar/exportar el equipamiento o calcular consumo de energía', en: 'Error exporting loadout or miscalculating power priorities' },
          desc: { es: 'Falla la exportación JSON o los porcentajes de consumo de reactor son incorrectos.', en: 'JSON export fails or power draw percentages look wrong.' },
          steps: [
            { id: 'ship-f4-1', es: 'Pulsé el botón de copiar/exportar ficha de nave o revisé el desglose de energía', en: 'Clicked copy/export ship build or inspected the power consumption breakdown' },
            { id: 'ship-f4-2', es: 'La nave tenía módulos guardianes, armas Powerplay o módulos desconectados en prioridades', en: 'Ship had Guardian modules, Powerplay weapons, or modules powered down in priority groups' },
          ],
        },
      ],
    },
    {
      id: 'navigation',
      category: 'module',
      icon: '🗺️',
      title: { es: 'Navegación y Planificador', en: 'Navigation & Route Planner' },
      summary: {
        es: 'Rutas de neutrones, estrellas repostables KGBFOAM, autocopia e interdicciones.',
        en: 'Neutron plotter, KGBFOAM scoopable check, clipboard autocopy, and interdiction monitor.',
      },
      entrySteps: [
        { id: 'nav-open', es: 'Abrí «Navegación y Planificador» y escribí origen/destino o cargué una ruta externa', en: 'Opened «Navigation & Route Planner» and entered origin/destination or loaded a route' },
        { id: 'nav-jump', es: 'Estaba realizando saltos consecutivos entre sistemas con la ruta activa', en: 'Was executing sequential jumps with an active route' },
      ],
      failureModes: [
        {
          id: 'nav-plotter-fail',
          label: { es: 'El calculador de rutas (incl. neutrones) devuelve error o tarda demasiado', en: 'Route / neutron plotter returns an error or hangs' },
          desc: { es: 'No calcula entre dos sistemas válidos o falla con Spansh/edsm al pedir saltos con sobrecarga.', en: 'Fails between valid systems or errors when fetching neutron jumps.' },
          steps: [
            { id: 'nav-f1-1', es: 'Introduje sistema de origen y destino válidos y seleccioné alcance o sobrecarga de neutrones', en: 'Entered valid origin & target systems and set jump range or neutron boost options' },
            { id: 'nav-f1-2', es: 'Pulsé calcular ruta y devolvió error de red, ruta vacía o quedó girando sin fin', en: 'Clicked compute route and got a network/empty error or endless spinner' },
          ],
        },
        {
          id: 'nav-autocopy-stuck',
          label: { es: 'La autocopia al portapapeles no avanza al siguiente salto o copia un sistema antiguo', en: 'Clipboard autocopy fails to advance to next waypoint or copies stale system' },
          desc: { es: 'Al completar FSDJump el portapapeles sigue con el sistema actual o no pega nada.', en: 'After FSDJump clipboard still holds old system or copies nothing.' },
          steps: [
            { id: 'nav-f2-1', es: 'Activé la opción de autocopiar el siguiente salto al portapapeles en ruta', en: 'Enabled automatic clipboard copy for the next route waypoint' },
            { id: 'nav-f2-2', es: 'Completé un salto (FSDJump) y al pegar (Ctrl+V) en el mapa galáctico salió el mismo sistema', en: 'Completed an FSDJump and pasting (Ctrl+V) into Galaxy Map pasted the previous waypoint' },
          ],
        },
        {
          id: 'nav-fuel-warning',
          label: { es: 'Error al detectar estrellas KGBFOAM aptas para repostar o alerta de tanque vacío', en: 'Wrong KGBFOAM scoopable detection or false empty-tank alert' },
          desc: { es: 'Marca como no repostable una estrella K/G/B/F/O/A/M o no avisa en estrella T Tauri / marrón.', en: 'Misclassifies star class or misses warning before un-scoopable gap.' },
          steps: [
            { id: 'nav-f3-1', es: 'Cargué una ruta de varios saltos pasando por estrellas enanas marrones / T Tauri / KGBFOAM', en: 'Loaded a multi-hop route crossing brown dwarfs / T Tauri / KGBFOAM stars' },
            { id: 'nav-f3-2', es: 'Revisé los iconos de combustible por salto o el cálculo de consumo tras repostar', en: 'Checked fuel icons per hop or remaining fuel calculation after scooping' },
          ],
        },
        {
          id: 'nav-csv-import',
          label: { es: 'No carga o parsea mal una ruta importada desde archivo CSV / Spansh', en: 'Imported Spansh / CSV route fails to load or parses wrong columns' },
          desc: { es: 'Al pegar o importar listas Road to Riches / Neutron, faltan saltos o da error de formato.', en: 'Pasting or importing Road to Riches / Neutron CSV misses rows or fails format check.' },
          steps: [
            { id: 'nav-f4-1', es: 'Exporté un CSV desde Spansh o pegué texto con nombres de sistemas en la zona de importación', en: 'Exported a CSV from Spansh or pasted system lines into the route importer' },
            { id: 'nav-f4-2', es: 'Al importarlo, faltaron columnas (cuerpos/distancia) o quedó la lista vacía', en: 'After importing, body/distance columns were missing or the waypoint list stayed empty' },
          ],
        },
      ],
    },
    {
      id: 'exploration',
      category: 'module',
      icon: '🔭',
      title: { es: 'Exploración Cartográfica', en: 'Cartographic Exploration' },
      summary: {
        es: 'Progreso FSS, semáforos de cuerpos valiosos (ELW/WW/Terraformables) y valor en créditos.',
        en: 'FSS scan progress, valuable body highlights (ELW/WW/Terraformables), and scan payout estimates.',
      },
      entrySteps: [
        { id: 'exp-fss', es: 'Salté a un sistema y lancé el escáner de descubrimiento (Honk) y/o FSS completo', en: 'Jumped into a system and ran Discovery Scanner (Honk) and/or full FSS' },
        { id: 'exp-panel', es: 'Consulté el módulo «Exploración Cartográfica» o la vista combinada de Exploración', en: 'Opened «Cartographic Exploration» or the combined Exploration cockpit layout' },
      ],
      failureModes: [
        {
          id: 'exp-body-value-wrong',
          label: { es: 'El valor cartográfico estimado difiere notablemente al vender en Universal Cartographics', en: 'Estimated scan payout differs noticeably when selling at Universal Cartographics' },
          desc: { es: 'Falta aplicar bono de primer descubrimiento, mapeo DSS eficiente o tipo terraformable.', en: 'First discovery, efficient DSS bonus, or terraformable multiplier is miscalculated.' },
          steps: [
            { id: 'exp-f1-1', es: 'Escaneé cuerpos con FSS y mapeé con sondas DSS los planetas marcados en verde/cian', en: 'Scanned bodies with FSS and DSS-mapped planets highlighted in green/cyan' },
            { id: 'exp-f1-2', es: 'Atraqué en estación o Fleet Carrier y vendí los datos comparando el total recibido', en: 'Docked at a station or Carrier and sold data comparing actual payout vs TMC total' },
          ],
        },
        {
          id: 'exp-fss-progress-stuck',
          label: { es: 'La barra de progreso FSS o la lista de cuerpos no aparece / queda incompleta', en: 'FSS progress bar or body table fails to appear or misses bodies' },
          desc: { es: 'Tras usar FSSDiscoveryScan, los cuerpos ya conocidos o anillos aparecen mal ordenados o faltantes.', en: 'Known bodies, belts, or FSS count do not match in-game system map.' },
          steps: [
            { id: 'exp-f2-1', es: 'Activé el modo FSS en un sistema con muchos cuerpos o un sistema ya descubierto previamente', en: 'Used FSS in a dense system or a pre-discovered Bubble system' },
            { id: 'exp-f2-2', es: 'Observé que el contador decía X de Y cuerpos sin actualizar al terminar o sin mostrar algunos planetas', en: 'Saw X/Y counter fail to reach 100% or hide scanned bodies' },
          ],
        },
        {
          id: 'exp-threshold-ignored',
          label: { es: 'El umbral mínimo para recomendar mapeo DSS o el semáforo de colores no se respeta', en: 'Configurable DSS mapping value threshold or color coding is ignored' },
          desc: { es: 'Cambié el umbral en Ajustes pero sigue marcando cuerpos baratos o ignorando terraformables.', en: 'Changed threshold in Settings yet low-value or terraformable flags did not adapt.' },
          steps: [
            { id: 'exp-f3-1', es: 'Ajusté el umbral mínimo de créditos de mapeo en Ajustes y guardé los cambios', en: 'Modified the minimum DSS value threshold in Settings and saved' },
            { id: 'exp-f3-2', es: 'Escaneé un planeta por encima/debajo de ese umbral y la insignia DSS no coincidía', en: 'Scanned a planet across that threshold and saw the wrong DSS badge' },
          ],
        },
        {
          id: 'exp-table-scroll',
          label: { es: 'Problemas de scroll o columnas cortadas en la tabla de cuerpos del sistema', en: 'Scroll issue or clipped columns in the system bodies table' },
          desc: { es: 'Faltan columnas a la derecha, la barra horizontal queda oculta o hay scroll vacío con pocos cuerpos.', en: 'Columns clipped on the right, missing sticky horizontal scroll, or extra blank scroll space.' },
          steps: [
            { id: 'exp-f4-1', es: 'Entré en un sistema con pocos cuerpos (1-3) o en un sistema gigante (> 30 cuerpos)', en: 'Entered a tiny system (1-3 bodies) or huge system (> 30 bodies)' },
            { id: 'exp-f4-2', es: 'Intenté desplazarme horizontalmente por las columnas de gravedad, distancia o valor', en: 'Tried scrolling horizontally across gravity, distance, or value columns' },
          ],
        },
      ],
    },
    {
      id: 'exobiology',
      category: 'module',
      icon: '🧬',
      title: { es: 'Exobiología', en: 'Exobiology' },
      summary: {
        es: 'Radar de bioseñales, brújula de rumbo hacia muestras, distancia mínima y bono 5×.',
        en: 'Biosignal radar, heading compass for uncollected samples, min sample distance, and 5x first-footfall bonus.',
      },
      entrySteps: [
        { id: 'exo-dss', es: 'Mapeé un planeta con señales biológicas y aterricé (o bajé en VRS / a pie)', en: 'Mapped a body with biological signals and landed (SRV or on-foot)' },
        { id: 'exo-scan', es: 'Usé el Escáner Genético para tomar la 1.ª o 2.ª muestra de una especie', en: 'Used the Genetic Sampler to scan the 1st or 2nd sample of a species' },
      ],
      failureModes: [
        {
          id: 'exo-distance-circle',
          label: { es: 'El radar de distancia mínima (m) no cuenta correctamente al alejarme de la muestra previa', en: 'Minimum sample distance meter fails to track distance while moving away' },
          desc: { es: 'Aunque ya superé los 150-800 m requeridos, la barra sigue en rojo o no avisa de zona válida.', en: 'Already moved past required CCR distance yet gauge stays red or misses valid range.' },
          steps: [
            { id: 'exo-f1-1', es: 'Tomé la muestra 1/3 y me desplacé en VRS, nave o a pie mirando la distancia en metros', en: 'Took sample 1/3 and moved via SRV, ship, or on-foot watching the meter readout' },
            { id: 'exo-f1-2', es: 'Comprobé mis coordenadas lat/lon en el HUD del juego y noté que el cálculo en TMC no variaba o daba saltos', en: 'Checked lat/lon in game HUD and noticed TMC distance froze or jumped unexpectedly' },
          ],
        },
        {
          id: 'exo-compass-heading',
          label: { es: 'La brújula visual de rumbo hacia muestras anteriores apunta en dirección errónea', en: 'Visual heading compass points in the wrong direction relative to previous samples' },
          desc: { es: 'El rumbo marcado no coincide con la orientación real en la superficie planetaria.', en: 'Bearing indicator does not align with true planetary surface heading.' },
          steps: [
            { id: 'exo-f2-1', es: 'Marqué una muestra y me alejé girando hacia un punto cardinal distinto', en: 'Logged a sample and turned to a different compass heading' },
            { id: 'exo-f2-2', es: 'Seguí el rumbo sugerido por la brújula de TMC cerca de los polos o ecuador del planeta', en: 'Followed TMC compass bearing near planetary poles or equator' },
          ],
        },
        {
          id: 'exo-value-multiplier',
          label: { es: 'No aplica o calcula mal el multiplicador 5× por primer descubrimiento (First Footfall)', en: 'Fails to apply or miscalculates 5x First Footfall payout bonus' },
          desc: { es: 'El planeta era virgen con First Footfall pero se muestra valor base ×1 (o viceversa).', en: 'Untouched planet shows 1x value or mapped world shows wrong 5x estimate.' },
          steps: [
            { id: 'exo-f3-1', es: 'Desembarqué en un planeta con o sin «First Footfall» previo y escanee señales con DSS', en: 'Disembarked on a planet with or without prior First Footfall after DSS scan' },
            { id: 'exo-f3-2', es: 'Al completar la especie (3/3), comparé el valor mostrado en TMC frente a Vista Genomics', en: 'Completed 3/3 scans and compared TMC payout against Vista Genomics' },
          ],
        },
        {
          id: 'exo-species-missing',
          label: { es: 'Una especie detectada por FSS/DSS o analizada con el escáner no aparece en la lista', en: 'Detected or scanned species fails to appear or update in species list' },
          desc: { es: 'Falta alguna variante regional o la muestra 3/3 no queda marcada como completada.', en: 'Specific genus/species missing or 3/3 completion state never clears.' },
          steps: [
            { id: 'exo-f4-1', es: 'Analicé una bacteria, tussock o stratuum tras cambiar de especie a mitad o morir en superficie', en: 'Scanned a species after switching partial samples or dying on surface' },
            { id: 'exo-f4-2', es: 'Entré de nuevo en la nave o cerré sesión antes de terminar las 3 muestras', en: 'Re-boarded ship or relogged before finishing all 3 samples' },
          ],
        },
      ],
    },
    {
      id: 'combat',
      category: 'module',
      icon: '⚔️',
      title: { es: 'Combate y Defensa Anti-Xeno', en: 'Combat & Anti-Xeno Defense' },
      summary: {
        es: 'Objetivos hostiles, seguimiento de corazones/enjambres Thargoides, calor crítico y recompensas.',
        en: 'Hostile targets, Thargoid heart & swarm tracker, overheat alerts, and bounty ledger.',
      },
      entrySteps: [
        { id: 'com-zone', es: 'Entré en una Zona de Conflicto, Baliza de Navegación, RES o Zona AX Thargoide', en: 'Entered a Conflict Zone, Nav Beacon, RES, or Thargoid AX Zone' },
        { id: 'com-target', es: 'Seleccioné blancos enemigos e inicié combate mientras observaba el módulo «Combate»', en: 'Targeted hostiles and engaged while monitoring the «Combat» module' },
      ],
      failureModes: [
        {
          id: 'com-thargoid-hearts',
          label: { es: 'Los corazones expuestos o destruidos del Interceptor Thargoide no se actualizan bien', en: 'Thargoid Interceptor exposed/destroyed hearts fail to update accurately' },
          desc: { es: 'Muestra el número equivocado de corazones restantes o no detecta el enjambre.', en: 'Shows wrong remaining heart count or misses swarm status.' },
          steps: [
            { id: 'com-f1-1', es: 'Fijé un Interceptor (Cyclops, Basilisk, Medusa o Hydra) e inicié ataque sobre un pétalo/corazón', en: 'Targeted an Interceptor (Cyclops, Basilisk, Medusa, Hydra) and exerted/destroyed a heart' },
            { id: 'com-f1-2', es: 'Cambie de objetivo brevemente (p. ej. al enjambre o a otro jugador) y volví al Interceptor', en: 'Switched target briefly (e.g. swarm or wingmate) and re-targeted the Interceptor' },
          ],
        },
        {
          id: 'com-bounties-wrong',
          label: { es: 'Los bonos de combate o recompensas acumuladas no suman o no se limpian al cobrar', en: 'Combat bonds or bounties fail to sum or remain listed after redemption' },
          desc: { es: 'Las muertes confirmadas no incrementan el total de la sesión o siguen tras cobrarlas en estación.', en: 'Confirmed kills do not add up or persist after cashing in at station.' },
          steps: [
            { id: 'com-f2-1', es: 'Destruí naves buscadas o Thargoides en solitario o en escuadrón (Wing / Multicrew)', en: 'Destroyed wanted ships or Thargoids solo or in a Wing / Multicrew' },
            { id: 'com-f2-2', es: 'Atraqué en estación o Interstellar Factors para cobrar bonos/recompensas', en: 'Docked at a station or Interstellar Factors and redeemed bonds/bounties' },
          ],
        },
        {
          id: 'com-heat-shields',
          label: { es: 'Fallo en avisos de sobrecalentamiento crítico o caídas de escudo', en: 'Critical heat or shield drop warnings fail to trigger' },
          desc: { es: 'Subió la temperatura > 100 % o cayeron los escudos sin cambio visual ni alerta.', en: 'Temperature exceeded 100% or shields dropped without visual/audio indicator.' },
          steps: [
            { id: 'com-f3-1', es: 'Disparé armas térmicas/gauss o usé células de escudo llegando a calor crítico', en: 'Fired thermal/gauss weapons or SCBs reaching critical heat' },
            { id: 'com-f3-2', es: 'Sufrí caída completa de escudo y comprobé los indicadores de Combate', en: 'Suffered full shield collapse and checked Combat gauges' },
          ],
        },
      ],
    },
    {
      id: 'mining',
      category: 'module',
      icon: '⛏️',
      title: { es: 'Minería', en: 'Mining' },
      summary: {
        es: 'Catálogo offline de hotspots, radar de prospección, bucle triangular y tasa por demanda.',
        en: 'Offline hotspot catalog, prospector limpet radar, triangular sell loop, and demand tax alert.',
      },
      entrySteps: [
        { id: 'min-open', es: 'Abrí el módulo «Minería» o la vista combinada de minería en anillos', en: 'Opened the «Mining» module or combined ring mining layout' },
        { id: 'min-ring', es: 'Me encontraba buscando yacimiento o lanzando drones prospectores en un anillo', en: 'Was searching for a ring hotspot or firing prospector limpets in a ring' },
      ],
      failureModes: [
        {
          id: 'min-prospector-miss',
          label: { es: 'Los drones prospectores no se muestran o no filtran bien mis minerales favoritos', en: 'Prospector scans do not display or ignore favorite mineral thresholds' },
          desc: { es: 'Al analizar un asteroide en español o inglés no aparece la pureza ni destaca Platino/Painita.', en: 'Asteroid mineral percentages do not appear or fail to match configured favorite.' },
          steps: [
            { id: 'min-f1-1', es: 'Marqué minerales favoritos y umbral mínimo de pureza en Ajustes / Minería', en: 'Configured favorite minerals and min purity % threshold in Settings / Mining' },
            { id: 'min-f1-2', es: 'Lancé un dron prospector (`ProspectedAsteroid`) sobre un asteroide con contenido metálico/núcleo', en: 'Fired a prospector limpet (`ProspectedAsteroid`) into a laser or core asteroid' },
            { id: 'min-f1-3', es: 'El juego estaba en idioma español/francés/alemán o inglés', en: 'Game client language was Spanish/French/German or English' },
          ],
        },
        {
          id: 'min-hotspots-empty',
          label: { es: 'El buscador de hotspots / anillos no encuentra yacimientos cercanos', en: 'Curated hotspot / ring finder returns no nearby overlapping rings' },
          desc: { es: 'No aparecen sistemas conocidos cerca de mi ubicación actual al filtrar por mineral.', en: 'No systems appear near current star when filtering by mineral.' },
          steps: [
            { id: 'min-f2-1', es: 'Seleccioné un mineral (ej. Platino, Ópalos de Vacío, Monazita) en el buscador de yacimientos', en: 'Selected a mineral (e.g. Platinum, Void Opals, Monazite) in hotspot finder' },
            { id: 'min-f2-2', es: 'Me encontraba lejos de la Burbuja / Colonia o filtré por tipo de anillo específico', en: 'Was far from Bubble/Colonia or restricted to pristine reserves / double overlaps' },
          ],
        },
        {
          id: 'min-sell-loop-tax',
          label: { es: 'El planificador de bucle triangular o alerta de tasa por demanda da precios irreales', en: 'Triangular sell loop or demand bulk-sales tax alert shows inaccurate prices' },
          desc: { es: 'Recomienda estación con demanda baja (< 4× bodega) sin avisar o precio antiguo.', en: 'Recommends low-demand station without bulk tax warning or shows stale prices.' },
          steps: [
            { id: 'min-f3-1', es: 'Llené parte de la bodega de carga con un mineral y busqué mejor precio / bucle de venta', en: 'Filled part of cargo hold with mined ore and queried best sell station / loop' },
            { id: 'min-f3-2', es: 'Viajé a la estación sugerida y el precio real en mercado era muy inferior por demanda o desfase', en: 'Flew to the recommended station and saw a much lower market sell price' },
          ],
        },
        {
          id: 'min-cargo-tph',
          label: { es: 'La capacidad de bodega, drones restantes o tasa toneladas/hora (t/h) marca mal', en: 'Cargo capacity, remaining limpets, or tons/hour rate displays wrong totals' },
          desc: { es: 'Al refinar minerales no suma toneladas o los drones activos/muertos descuadran.', en: 'Refining ore does not add tons cleanly or limpet counts drift.' },
          steps: [
            { id: 'min-f4-1', es: 'Comencé una sesión de minería con drones en bodega y fui refinando fragmentos', en: 'Started mining with limpets in hold and refined fragments into cargo' },
            { id: 'min-f4-2', es: 'Eyecté minerales o perdí drones recolectores y miré los indicadores de bodega / t/h', en: 'Jettisoned cargo or lost collector limpets and checked hold / t/h gauges' },
          ],
        },
      ],
    },
    {
      id: 'trade',
      category: 'module',
      icon: '📦',
      title: { es: 'Comercio y Mercados', en: 'Trade & Markets' },
      summary: {
        es: 'Mercado de estación, beneficio por tonelada, mejores rutas de venta y compras/ventas.',
        en: 'Station commodity market, profit per ton, best sell finder, and trade history.',
      },
      entrySteps: [
        { id: 'trd-open', es: 'Atraqué en una estación / Carrier y abrí la pantalla «Comercio y Mercados»', en: 'Docked at a station / Carrier and opened «Trade & Markets»' },
        { id: 'trd-market', es: 'Abrí el Mercado de Mercancías dentro del juego para generar Market.json', en: 'Opened the in-game Commodity Market to emit Market.json' },
      ],
      failureModes: [
        {
          id: 'trd-market-not-loaded',
          label: { es: 'No carga la lista de mercancías de la estación actual o muestra la estación anterior', en: 'Current station market table does not load or shows the previous station' },
          desc: { es: 'Tras abrir el Mercado en el juego, TMC sigue vacío o con precios viejos.', en: 'After opening Market in-game, TMC stays empty or shows stale data.' },
          steps: [
            { id: 'trd-f1-1', es: 'Atraqué en una estación nueva y abrí «Servicios del Puerto -> Mercado» en Elite Dangerous', en: 'Docked at a new station and opened Starport Services -> Commodities Market' },
            { id: 'trd-f1-2', es: 'Volví a TMC tras unos segundos y no refrescó precios ni existencias locales', en: 'Switched back to TMC after a few seconds and prices/stock had not updated' },
          ],
        },
        {
          id: 'trd-route-bad-pad',
          label: { es: 'Recomienda estaciones donde mi nave no cabe (pista pequeña/mediana) o demasiado lejanas', en: 'Recommends stations incompatible with ship pad size (Large) or outside distance filter' },
          desc: { es: 'Estando en nave grande sugiere un Outpost, o sugiere superficies planetarias cuando las desactivé.', en: 'Flying a Large ship suggests an Outpost or ignored planetary/Carrier filter.' },
          steps: [
            { id: 'trd-f2-1', es: 'Piloteaba una nave grande (Anaconda/Cutter/Type-9) o apliqué filtros de tamaño de pista/superficie', en: 'Flew a Large ship (Anaconda/Cutter/Type-9) or enabled pad-size / surface filters' },
            { id: 'trd-f2-2', es: 'Ejecuté la búsqueda de mejor estación de compra o venta para una mercancía en bodega', en: 'Ran best buy/sell station search for a commodity in hold' },
          ],
        },
        {
          id: 'trd-profit-math',
          label: { es: 'El beneficio por tonelada, coste medio de compra o historial está mal calculado', en: 'Profit per ton, buy cost basis, or trade ledger shows incorrect math' },
          desc: { es: 'Cuando combino mercancía comprada y minada/recompensa de misión, el margen no cuadra.', en: 'Mixing bought, mined, or mission cargo yields wrong margin.' },
          steps: [
            { id: 'trd-f3-1', es: 'Compré mercancía en una estación (o recibí carga de misión/carrier) y la vendí en otro sistema', en: 'Bought commodities (or took mission/carrier cargo) and sold in another system' },
            { id: 'trd-f3-2', es: 'Comparé el beneficio estimado en TMC con el crédito real ingresado al vender', en: 'Compared TMC estimated profit with actual credits earned upon sale' },
          ],
        },
      ],
    },
    {
      id: 'missions',
      category: 'module',
      icon: '📋',
      title: { es: 'Misiones, Turismo y Eventos', en: 'Missions, Tourism & Events' },
      summary: {
        es: 'Apilamiento por destino/facción, rutas de pasajeros VIP y Community Goals activos.',
        en: 'Mission stacking by destination/faction, VIP passenger routes, and active Community Goals.',
      },
      entrySteps: [
        { id: 'mis-open', es: 'Entré en «Misiones, Turismo y Eventos» tras aceptar o entregar misiones en una estación', en: 'Opened «Missions, Tourism & Events» after accepting or completing missions' },
        { id: 'mis-cg', es: 'Consulté el apilamiento por sistema/facción, pestaña de pasajeros o Community Goals', en: 'Viewed destination/faction stack, passenger tab, or Community Goals' },
      ],
      failureModes: [
        {
          id: 'mis-stack-wrong',
          label: { es: 'Misiones completadas siguen activas o el apilamiento suma mal las bajas/entregas', en: 'Completed missions remain active or massacre stacking totals sum incorrectly' },
          desc: { es: 'Tras entregar o abandonar misiones no desaparecen, o mezcla facciones repetidas.', en: 'Abandoned/completed missions persist or stack groups count wrong faction overlap.' },
          steps: [
            { id: 'mis-f1-1', es: 'Acepté múltiples misiones de masacre / reparto de varias facciones hacia el mismo sistema', en: 'Stacked multiple massacre/delivery missions from multiple factions to one target system' },
            { id: 'mis-f1-2', es: 'Completé, entregué, expiré o abandoné al menos una misión sin que la lista se limpiara', en: 'Completed, redeemed, expired, or abandoned at least one mission while list kept it' },
          ],
        },
        {
          id: 'mis-passengers-route',
          label: { es: 'El optimizador de turismo/pasajeros calcula mal el orden de saltos o no detecta balizas', en: 'VIP passenger route optimizer sorts hops badly or misses tourist beacons' },
          desc: { es: 'Propone rutas dando rodeos o ignora las múltiples paradas de un pasajero VIP.', en: 'Produces inefficient hop order or drops multi-stop VIP waypoints.' },
          steps: [
            { id: 'mis-f2-1', es: 'Acepté una o más misiones de pasajeros con varias balizas turísticas de destino', en: 'Accepted one or more passenger missions with multiple tourist beacon stops' },
            { id: 'mis-f2-2', es: 'Escaneé una baliza turística y revisé si TMC marcaba esa parada como completada', en: 'Scanned a tourist beacon and checked whether TMC marked that stop done' },
          ],
        },
        {
          id: 'mis-cg-empty',
          label: { es: 'Los Community Goals (Eventos) no cargan progreso, recompensas o plazo límite', en: 'Community Goals show no active events, stale tier progress, or wrong deadline' },
          desc: { es: 'Eventos activos en el juego no aparecen en TMC o muestran aportación 0 tras entregar.', en: 'In-game CGs are missing in TMC or contribution stays at 0 after turning in.' },
          steps: [
            { id: 'mis-f3-1', es: 'Me apunté a un Community Goal en estación y entregué mercancía / bonos de combate', en: 'Signed up for a CG at station and delivered cargo or combat bonds' },
            { id: 'mis-f3-2', es: 'Abrí el panel de Eventos o consulté el tablero de misiones del juego para refrescar diario', en: 'Checked the Events tab or opened the in-game Mission Board to trigger journal updates' },
          ],
        },
      ],
    },
    {
      id: 'engineering',
      category: 'module',
      icon: '🔧',
      title: { es: 'Ingeniería y Materiales', en: 'Engineering & Materials' },
      summary: {
        es: 'Guía paso a paso para desbloquear ingenieros, planos, lista de compra e intercambios.',
        en: 'Step-by-step engineer unlock guide, blueprints, shopping list, and Material Trader solver.',
      },
      entrySteps: [
        { id: 'eng-open', es: 'Abrí el módulo «Ingeniería y Materiales» y seleccioné un ingeniero o plano concreto', en: 'Opened «Engineering & Materials» and picked an engineer or blueprint' },
        { id: 'eng-load', es: 'Tenía cargado el diario con mi inventario de materiales Raw / Encoded / Manufactured', en: 'Had journal loaded with Raw / Encoded / Manufactured inventory' },
      ],
      failureModes: [
        {
          id: 'eng-unlock-stuck',
          label: { es: 'El ayudante de desbloqueo marca un ingeniero bloqueado cuando ya lo tengo (o viceversa)', en: 'Unlock helper shows an already-unlocked engineer as locked (or vice versa)' },
          desc: { es: 'El stepper de Descubierto -> Invitado -> Desbloqueado/Rango no refleja mi progreso real en el juego.', en: 'Discovered -> Invited -> Unlocked/Grade stepper does not match in-game engineer status.' },
          steps: [
            { id: 'eng-f1-1', es: 'Desbloqueé o subí de rango con un ingeniero en su taller dentro del juego (o tras cumplir el requisito)', en: 'Unlocked or ranked up an engineer in-game at their workshop' },
            { id: 'eng-f1-2', es: 'Reinicié sesión o abrí la pestaña «Ingenieros» en el panel derecho del juego para emitir EngineerProgress', en: 'Relogged or opened the in-game Engineers screen to emit EngineerProgress' },
            { id: 'eng-f1-3', es: 'El stepper en TMC seguía mostrando el paso previo o pedía donación ya realizada', en: 'TMC stepper stayed on an earlier step or still asked for an already-paid donation' },
          ],
        },
        {
          id: 'eng-materials-mismatch',
          label: { es: 'El stock de materiales o los grados restantes de tiradas no coinciden con la nave', en: 'Material counts or estimated grade rolls do not match in-game inventory' },
          desc: { es: 'Tras gastar materiales en modificaciones o recolectar en superficie/señales, las cifras quedan desfasadas.', en: 'After crafting modifications or farming mats, stock counts are out of sync.' },
          steps: [
            { id: 'eng-f2-1', es: 'Recogí materiales, realicé intercambios en un Material Trader o apliqué tiradas de plano', en: 'Collected materials, traded at a Material Trader, or rolled blueprint upgrades' },
            { id: 'eng-f2-2', es: 'Verifiqué las cantidades de Raw/Manufactured/Encoded entre el juego y TMC', en: 'Compared Raw/Manufactured/Encoded stock counts between game and TMC' },
          ],
        },
        {
          id: 'eng-trader-bad',
          label: { es: 'La calculadora de intercambio de materiales sugiere trades imposibles o un trader erróneo', en: 'Material Trader solver suggests invalid trades or wrong trader type/location' },
          desc: { es: 'Propone intercambiar con materiales que tengo a 0 o calcula mal el ratio 6:1 / 3:1.', en: 'Suggests missing source mats, wrong 6:1 / 3:1 ratio, or wrong trader category.' },
          steps: [
            { id: 'eng-f3-1', es: 'Añadí uno o varios planos/efectos experimentales a la lista de la compra', en: 'Added one or more blueprints/experimentals to the shopping list' },
            { id: 'eng-f3-2', es: 'Pulsé en calcular intercambios con excedentes y revisé la propuesta del comerciante más cercano', en: 'Clicked calculate material trades and checked the proposed trades & nearest trader' },
          ],
        },
        {
          id: 'eng-blueprint-missing',
          label: { es: 'Falta algún plano, efecto experimental o requisito correcto (Colonia / Odyssey / módulos)', en: 'Missing blueprint, experimental effect, or wrong unlock requirement (Colonia/Odyssey)' },
          desc: { es: 'Los materiales pedidos por grado o por ingeniería de trajes/armas Odyssey no coinciden.', en: 'Recipe costs per grade or Odyssey suit/weapon mods look wrong.' },
          steps: [
            { id: 'eng-f4-1', es: 'Busqué un plano de módulo de nave o mejora de traje/arma Odyssey en la lista', en: 'Searched for a ship module blueprint or Odyssey suit/weapon mod' },
            { id: 'eng-f4-2', es: 'Comparé los materiales indicados en TMC con el taller del ingeniero dentro del juego', en: 'Compared required materials in TMC against the in-game workshop recipe' },
          ],
        },
      ],
    },
    {
      id: 'carrier',
      category: 'module',
      icon: '🛸',
      title: { es: 'Fleet Carrier', en: 'Fleet Carrier' },
      summary: {
        es: 'Salto del megacrucero, cuenta atrás, tritio, mantenimiento semanal y finanzas.',
        en: 'Carrier jump control, countdown timer, tritium tank, weekly upkeep, and finances.',
      },
      entrySteps: [
        { id: 'car-open', es: 'Abrí el módulo «Fleet Carrier» siendo propietario de un Fleet Carrier o tras gestionar servicios', en: 'Opened «Fleet Carrier» as a Carrier owner or after opening Carrier management' },
        { id: 'car-capi', es: 'Tenía vinculado o no Frontier CAPI / diario con eventos recientes de Carrier', en: 'Had Frontier CAPI linked (or journal-only) with recent Carrier events' },
      ],
      failureModes: [
        {
          id: 'car-jump-countdown',
          label: { es: 'La cuenta atrás de salto del Fleet Carrier no arranca, se congela o acaba fuera de hora', en: 'Carrier jump countdown fails to start, freezes, or shows wrong time' },
          desc: { es: 'Tras programar salto en el juego, el temporizador no coincide con la hora UTC real de salto.', en: 'After plotting Carrier jump, timer drifts from actual UTC departure time.' },
          steps: [
            { id: 'car-f1-1', es: 'Programé un salto de Fleet Carrier desde la gestión a bordo en el juego', en: 'Plotted a Fleet Carrier jump from in-game Carrier Management' },
            { id: 'car-f1-2', es: 'Cancelé el salto o esperé el cierre de plataforma y la cuenta atrás en TMC quedó desincronizada', en: 'Cancelled jump or waited through lockdown while TMC countdown stayed wrong' },
          ],
        },
        {
          id: 'car-tritium-balance',
          label: { es: 'El depósito de tritio, bodega a bordo o saldo financiero semanal no se actualiza', en: 'Tritium depot, carrier cargo, or weekly finance balance does not update' },
          desc: { es: 'Doné tritio al depósito o transferí créditos y los indicadores muestran cifras anteriores.', en: 'Donated tritium or transferred credits without numbers refreshing.' },
          steps: [
            { id: 'car-f2-1', es: 'Doné tritio al depósito o moví créditos en el banco del Fleet Carrier', en: 'Donated tritium or transferred credits in the Carrier bank' },
            { id: 'car-f2-2', es: 'Abrí «Gestión del Fleet Carrier» en el juego para forzar la actualización de estado', en: 'Opened Carrier Management in-game to emit update events' },
          ],
        },
        {
          id: 'car-not-detected',
          label: { es: 'TMC indica que no tengo Fleet Carrier a pesar de poseer uno en el juego', en: 'TMC shows no owned Fleet Carrier even though I own one in-game' },
          desc: { es: 'El módulo queda en estado vacío tras haber abierto el panel del Carrier o conectado CAPI.', en: 'Panel remains in empty state despite owning a Carrier.' },
          steps: [
            { id: 'car-f3-1', es: 'Compré o accedí a la pantalla de Gestión de mi Fleet Carrier durante la sesión actual', en: 'Purchased or opened my Fleet Carrier Management screen during the current session' },
            { id: 'car-f3-2', es: 'Cambié entre cuentas o conecté Frontier OAuth CAPI en Ajustes -> Cuentas', en: 'Switched accounts or connected Frontier OAuth CAPI in Accounts settings' },
          ],
        },
      ],
    },
    {
      id: 'colonisation',
      category: 'module',
      icon: '🏗️',
      title: { es: 'Colonización de Sistemas', en: 'System Colonisation' },
      summary: {
        es: 'Obras en estaciones/naves de colonización, materiales pendientes y suministros aportados.',
        en: 'Colonisation construction progress, pending material manifests, and delivered commodities.',
      },
      entrySteps: [
        { id: 'col-open', es: 'Abrí el módulo «Colonización de Sistemas» con un proyecto o depósito de construcción activo', en: 'Opened «System Colonisation» with an active construction project or depot' },
        { id: 'col-depot', es: 'Atraqué en la Nave de Colonización / Obra Orbital o planetaria del sistema', en: 'Docked at a System Colonisation Ship or orbital/planetary construction depot' },
      ],
      failureModes: [
        {
          id: 'col-materials-not-updating',
          label: { es: 'Los materiales pendientes del proyecto no descuentan lo ya entregado', en: 'Remaining project materials fail to decrement after delivering commodities' },
          desc: { es: 'Tras descargar toneladas en la obra, la lista sigue pidiendo las cantidades iniciales.', en: 'After delivering cargo to construction depot, list keeps showing old quantities.' },
          steps: [
            { id: 'col-f1-1', es: 'Entregué mercancías en el Depósito de Construcción dentro del juego (`ColonisationContribution`)', en: 'Delivered commodities to the Construction Depot in-game (`ColonisationContribution`)' },
            { id: 'col-f1-2', es: 'Salí del menú del depósito o cambié de sesión y los pendientes quedaron sin restar', en: 'Exited depot menu or changed session and remaining counts stayed unchanged' },
          ],
        },
        {
          id: 'col-project-missing',
          label: { es: 'No aparece un proyecto activo o mezcla dos obras distintas en el mismo sistema', en: 'Active construction site is missing or merges two projects in the same system' },
          desc: { es: 'Construcciones primarias y secundarias se solapan o desaparecen al saltar fuera del sistema.', en: 'Primary and secondary sites overlap or vanish upon leaving the system.' },
          steps: [
            { id: 'col-f2-1', es: 'Reclamé un sistema o inicié una segunda construcción orbital/superficial en el mismo sistema', en: 'Claimed a system or started a second orbital/surface build in the same system' },
            { id: 'col-f2-2', es: 'Salté a otro sistema a comprar materiales y al mirar TMC el proyecto ya no estaba o cambió', en: 'Jumped away to buy materials and checked TMC to find the project missing or mixed' },
          ],
        },
        {
          id: 'col-cargo-diff',
          label: { es: 'El balance entre bodega actual, Fleet Carrier y faltantes marca negativo o erróneo', en: 'Balance between current ship cargo, Carrier stock, and needed amounts is wrong' },
          desc: { es: 'La columna «en bodega» o el cálculo de viajes restantes es incoherente.', en: '«In hold» column or estimated hauling trips calculation is wrong.' },
          steps: [
            { id: 'col-f3-1', es: 'Cargué la nave con parte de varios materiales requeridos para la obra', en: 'Loaded ship with mixed materials required by the construction site' },
            { id: 'col-f3-2', es: 'Abrí Colonización para comprobar cuántos viajes quedaban por material', en: 'Checked Colonisation panel for remaining haul counts per commodity' },
          ],
        },
      ],
    },
    {
      id: 'powerplay',
      category: 'module',
      icon: '🏛️',
      title: { es: 'Powerplay 2.0', en: 'Powerplay 2.0' },
      summary: {
        es: 'Rango y méritos con tu potencia, tareas semanales, sistemas en disputa y ventajas.',
        en: 'Power rank & merits, weekly assignments, contested/fortified systems, and perks.',
      },
      entrySteps: [
        { id: 'pp-open', es: 'Estando vinculado a una potencia en Powerplay 2.0, abrí el módulo «Powerplay 2.0»', en: 'While pledged to a Power in Powerplay 2.0, opened «Powerplay 2.0» module' },
        { id: 'pp-merits', es: 'Gané méritos, completé asignaciones semanales o entré al panel de Potencia en el juego', en: 'Earned merits, progressed weekly assignments, or opened the in-game Power screen' },
      ],
      failureModes: [
        {
          id: 'pp-merits-rank-stuck',
          label: { es: 'Los méritos acumulados o el rango actual no suben tras acciones Powerplay', en: 'Accumulated merits or Power rank fail to increment after PP actions' },
          desc: { es: 'Muestra 0 méritos o el rango antiguo aunque el juego ya registró `PowerplayMerits` / `PowerplayRank`.', en: 'Shows 0 merits or old rank even though game recorded `PowerplayMerits`/`PowerplayRank`.' },
          steps: [
            { id: 'pp-f1-1', es: 'Realicé actividades de Powerplay (escaneo, comercio, bonos o entregas) ganando méritos en HUD', en: 'Performed PP activities (scanning, trade, bounties, deliveries) gaining merits in HUD' },
            { id: 'pp-f1-2', es: 'Subí de rango o cambié de ciclo semanal (jueves UTC) sin que TMC actualizara el rango', en: 'Ranked up or crossed the Thursday weekly tick without TMC updating rank/totals' },
          ],
        },
        {
          id: 'pp-system-state-wrong',
          label: { es: 'El estado del sistema actual (Fortificado, Bastión, Adquisición, Disputa) es incorrecto', en: 'Current system Powerplay state (Fortified, Stronghold, Acquisition, Contested) is wrong' },
          desc: { es: 'Indica potencia controladora errónea o no muestra potencias en disputa en sistemas fronterizos.', en: 'Shows wrong controlling Power or misses contested powers.' },
          steps: [
            { id: 'pp-f2-1', es: 'Salté a un sistema controlado o disputado en Powerplay 2.0', en: 'Jumped into a Powerplay-controlled or contested star system' },
            { id: 'pp-f2-2', es: 'Comparé la tarjeta de estado de sistema en TMC con el mapa de Powerplay dentro del juego', en: 'Compared TMC system card state against the in-game Galaxy Map Powerplay filter' },
          ],
        },
        {
          id: 'pp-assignments-missing',
          label: { es: 'Las tareas semanales asignadas no aparecen o no se marcan como completadas', en: 'Weekly Powerplay assignments do not show up or fail to mark complete' },
          desc: { es: 'No refleja el progreso ni las tareas activas tras abrir el panel de potencia en cabina.', en: 'Assignment checklist remains empty or stuck at 0%.' },
          steps: [
            { id: 'pp-f3-1', es: 'Abrí «Potencia Galáctica» en el juego para consultar las misiones semanales tras el mantenimiento', en: 'Opened Galactic Powers in-game to inspect weekly assignments after server tick' },
            { id: 'pp-f3-2', es: 'Avancé en una tarea (p. ej. vender mercancía o recolectar datos) y TMC no cambió el progreso', en: 'Progressed an assignment (e.g. sell goods or scan wakes) and TMC stayed unchanged' },
          ],
        },
      ],
    },
    {
      id: 'bgs',
      category: 'module',
      icon: '⚖️',
      title: { es: 'Facciones y Política Local (BGS)', en: 'Factions & BGS' },
      summary: {
        es: 'Influencia de facciones del sistema, conflictos (guerra/elección), estados y tendencias.',
        en: 'Local system faction influence, conflicts (war/election), active states, and trends.',
      },
      entrySteps: [
        { id: 'bgs-open', es: 'Salté a un sistema habitado y abrí el módulo «Facciones y Política Local (BGS)»', en: 'Jumped into an inhabited system and opened «Factions & BGS»' },
        { id: 'bgs-fsd', es: 'Consulté los porcentajes de influencia, conflictos activos o estados de cada facción', en: 'Inspected faction influence shares, active conflicts, or faction states' },
      ],
      failureModes: [
        {
          id: 'bgs-empty-system',
          label: { es: 'No aparecen facciones en un sistema habitado o conserva las del sistema anterior', en: 'No factions appear in an inhabited system or previous system factions persist' },
          desc: { es: 'Al saltar a otro sistema poblado, las barras de influencia no se actualizan o quedan vacías.', en: 'Upon jumping to an inhabited star, faction bars stay blank or stuck on previous star.' },
          steps: [
            { id: 'bgs-f1-1', es: 'Salté de un sistema poblado a otro sistema poblado (con estaciones)', en: 'Jumped from one populated system into another inhabited system' },
            { id: 'bgs-f1-2', es: 'Atraqué en una estación del sistema o abrí los informes de facciones en cabina sin que cambiara BGS', en: 'Docked at a station or opened faction status panel without BGS module refreshing' },
          ],
        },
        {
          id: 'bgs-influence-conflicts',
          label: { es: 'Las influencias (%) o marcador de guerras/elecciones muestran valores antiguos', en: 'Influence % or war/election scoreboards show outdated ticks' },
          desc: { es: 'Los porcentajes difieren del panel derecho del juego o no reflejan estados pendientes/en recuperación.', en: 'Influence percentages differ from in-game right panel or ignore pending states.' },
          steps: [
            { id: 'bgs-f2-1', es: 'Abrí el panel de facciones local dentro del juego y comparé con los valores de TMC', en: 'Opened local faction status in-game and compared percentages with TMC' },
            { id: 'bgs-f2-2', es: 'Revisé conflictos activos (días ganados en guerra/elección) tras el tick diario del servidor', en: 'Checked active war/election day scores after the daily BGS tick' },
          ],
        },
        {
          id: 'bgs-mission-delta',
          label: { es: 'No registra o contabiliza mal mi aportación (+) en influencia/seguridad para una facción', en: 'Personal influence/security (+) tracking per faction fails to count actions' },
          desc: { es: 'Tras entregar misiones INF+++ o vender datos, el resumen personal de BGS no registra cambios.', en: 'INF+++ mission turn-ins or bounty redemptions do not show up in session BGS tracker.' },
          steps: [
            { id: 'bgs-f3-1', es: 'Entregué recompensas, datos cartográficos o misiones eligiendo recompensa de Influencia (INF)', en: 'Turned in bounties, carto data, or INF reward missions for a minor faction' },
            { id: 'bgs-f3-2', es: 'Revisé el registro de impacto por facción en la parte inferior o desplegable del módulo BGS', en: 'Inspected per-faction impact log in the BGS module details' },
          ],
        },
      ],
    },
    {
      id: 'crime',
      category: 'module',
      icon: '🚨',
      title: { es: 'Estado Legal y Contrabando', en: 'Legal Status & Smuggling' },
      summary: {
        es: 'Multas, recompensas activas, notoriedad y localizador de Interstellar Factors.',
        en: 'Fines, active bounties, notoriety meter, and nearest Interstellar Factors locator.',
      },
      entrySteps: [
        { id: 'crm-open', es: 'Abrí «Estado Legal y Contrabando» tras recibir multa/recompensa o llevar carga ilícita', en: 'Opened «Legal Status & Smuggling» after gaining a fine/bounty or carrying illicit cargo' },
        { id: 'crm-check', es: 'Revisé la lista de multas/recompensas, nivel de notoriedad o buscador de Interstellar Factors', en: 'Checked active fines/bounties, notoriety, or Interstellar Factors finder' },
      ],
      failureModes: [
        {
          id: 'crm-paid-still-shown',
          label: { es: 'Una multa o recompensa ya pagada (o limpiada tras morir) sigue apareciendo como activa', en: 'A paid fine or cleared bounty remains listed as active' },
          desc: { es: 'Tras abonar en Contactos o Interstellar Factors, la deuda sigue marcada en el panel.', en: 'After paying at Contacts or Interstellar Factors, item still shows up.' },
          steps: [
            { id: 'crm-f1-1', es: 'Pagué una multa o limpié una recompensa en Contactos / Interstellar Factors (o me entregué)', en: 'Paid a fine, cleared a bounty at Interstellar Factors, or handed myself in' },
            { id: 'crm-f1-2', es: 'La multa desapareció del panel izquierdo «Transacciones» del juego pero siguió visible en TMC', en: 'Cleared in-game Transactions panel while TMC still displayed it' },
          ],
        },
        {
          id: 'crm-notoriety-stuck',
          label: { es: 'La notoriedad (0-10) no baja o no avisa cuando es segura para usar Interstellar Factors', en: 'Notoriety (0-10) does not decay or shows wrong Interstellar Factors availability' },
          desc: { es: 'Marca notoriedad > 0 cuando en cabina ya bajó a 0 tras 2 horas en juego.', en: 'Displays notoriety > 0 after in-game decay already reached 0.' },
          steps: [
            { id: 'crm-f2-1', es: 'Estuve conectado > 2 horas para bajar 1 punto de notoriedad en el juego', en: 'Stayed logged in for 2+ hours so notoriety decayed by 1 in-game' },
            { id: 'crm-f2-2', es: 'Comprobé el panel derecho de Estadísticas/Crimen (ya era 0) frente al valor de TMC', en: 'Checked in-game Crime stats (at 0) against TMC value' },
          ],
        },
        {
          id: 'crm-if-finder-bad',
          label: { es: 'El localizador de Interstellar Factors recomienda una estación sin el servicio (o controlada por la facción que me busca)', en: 'Interstellar Factors locator suggests a station missing the service or controlled by the bounty faction' },
          desc: { es: 'Llego a la estación recomendada y no aparece Contacto Interestelar o sigue bloqueada por acceso anónimo.', en: 'Recommended station lacks IF or cannot pay local faction bounty there.' },
          steps: [
            { id: 'crm-f3-1', es: 'Usé el botón de «Interstellar Factors más cercano» teniendo una recompensa activa con cierta facción', en: 'Used «Nearest Interstellar Factors» while holding an active bounty for a faction' },
            { id: 'crm-f3-2', es: 'Salté al sistema/estación indicada y revisé la pestaña Contactos dentro del juego', en: 'Flew to the suggested station and opened Contacts in-game' },
          ],
        },
      ],
    },
    {
      id: 'accounts',
      category: 'module',
      icon: '🔑',
      title: { es: 'Cuentas y Flota Remota', en: 'Accounts & Remote Fleet' },
      summary: {
        es: 'Frontier CAPI OAuth 2.0, Inara (InAPI v1), EDSM, EDDN y explorador de flota remota.',
        en: 'Frontier CAPI OAuth 2.0, Inara (InAPI v1), EDSM, EDDN sync, and stored ship browser.',
      },
      entrySteps: [
        { id: 'acc-open', es: 'Abrí Ajustes -> Cuentas o el módulo «Cuentas y Flota Remota»', en: 'Opened Settings -> Accounts or the «Accounts & Remote Fleet» module' },
        { id: 'acc-service', es: 'Intenté autenticarme o sincronizar con Frontier CAPI, Inara, EDSM o EDDN', en: 'Attempted authentication or sync with Frontier CAPI, Inara, EDSM, or EDDN' },
      ],
      failureModes: [
        {
          id: 'acc-capi-oauth',
          label: { es: 'Falla el inicio de sesión OAuth 2.0 de Frontier CAPI o no vuelve del navegador', en: 'Frontier CAPI OAuth 2.0 login fails or callback never completes' },
          desc: { es: 'La ventana web da error de redirect URI, caduca o muestra estado sin vincular tras autorizar.', en: 'Browser callback returns URI error, times out, or stays disconnected after login.' },
          steps: [
            { id: 'acc-f1-1', es: 'Pulsé «Conectar con Frontier (OAuth)» y se abrió el navegador con `auth.frontierstore.net`', en: 'Clicked «Connect with Frontier (OAuth)» and completed login in the browser' },
            { id: 'acc-f1-2', es: 'Autoricé el acceso y la página de callback (`oauth/callback.html`) mostró error o no redirigió a localhost', en: 'Authorized access and callback page showed an error or failed forwarding to localhost' },
            { id: 'acc-f1-3', es: 'Estaba ejecutando TMC localmente o desde una sesión en otra red / navegador distinto', en: 'Was running TMC locally or across a different device / browser profile' },
          ],
        },
        {
          id: 'acc-inara-rejected',
          label: { es: 'Inara (InAPI v1) rechaza la clave o muestra estado de error en envío de eventos', en: 'Inara (InAPI v1) rejects API key or errors during event upload' },
          desc: { es: 'Aunque pegué mi API key personal de inara.cz, marca error o se acumulan mensajes en cola.', en: 'Valid personal API key is rejected or events pile up in queue.' },
          steps: [
            { id: 'acc-f2-1', es: 'Pegué mi clave personal generada en `inara.cz/elite/cmdr-settings-api/` y pulsé «Guardar y verificar»', en: 'Pasted personal key from `inara.cz/elite/cmdr-settings-api/` and clicked Save & Verify' },
            { id: 'acc-f2-2', es: 'El nombre de comandante o el modo Legacy / Live estaba activo al enviarse el paquete', en: 'Checked Commander name match and Live vs Legacy galaxy setting' },
            { id: 'acc-f2-3', es: 'Pulsé «Reintentar ahora» y volvió a devolver rechazo de servidor o límite de frecuencia (2/min)', en: 'Clicked «Retry now» and received server rejection or rate-limit failure' },
          ],
        },
        {
          id: 'acc-edsm-eddn',
          label: { es: 'EDSM o EDDN no reciben saltos/escaneos o envían en galaxia Legacy por error', en: 'EDSM or EDDN fail to upload jumps/scans or behave wrongly with Legacy' },
          desc: { es: 'El historial de cola marca errores de envío HTTP o el botón de prueba falla.', en: 'Upload queue reports HTTP errors or test button fails.' },
          steps: [
            { id: 'acc-f3-1', es: 'Activé subida a EDDN y/o configuré nombre de comandante + clave API en EDSM', en: 'Enabled EDDN relay and/or entered Commander name + API key for EDSM' },
            { id: 'acc-f3-2', es: 'Realicé un salto (`FSDJump`) o escaneo FSS/mercado en modo Live', en: 'Performed an `FSDJump`, FSS scan, or Market dock in Live mode' },
          ],
        },
        {
          id: 'acc-fleet-sync',
          label: { es: 'Las naves guardadas (Flota Remota) están desactualizadas respecto a las estaciones', en: 'Stored remote fleet list is outdated compared to actual shipyard locations' },
          desc: { es: 'Muestra una nave en un sistema antiguo tras haberla transferido o vendido.', en: 'Shows a transferred or sold ship at its old station.' },
          steps: [
            { id: 'acc-f4-1', es: 'Transferí o vendí una nave guardada desde el Astillero en el juego', en: 'Transferred or sold a stored ship via in-game Shipyard' },
            { id: 'acc-f4-2', es: 'Refresqué Cuentas / CAPI y seguía figurando la ubicación o módulo anterior', en: 'Refreshed Accounts / CAPI yet old location or loadout persisted' },
          ],
        },
      ],
    },

    // ======================== 5 ÁREAS DEL SISTEMA ========================
    {
      id: 'install',
      category: 'system',
      icon: '💿',
      title: { es: 'Instalación y Actualización', en: 'Installation & Auto-Updater' },
      summary: {
        es: 'Instalador TMC-Setup.exe, ejecutable portable, arranque en Windows/Linux o actualizaciones.',
        en: 'TMC-Setup.exe installer, portable exe, Windows/Linux launch, and release updater.',
      },
      entrySteps: [
        { id: 'ins-dl', es: 'Descargué `TMC-Setup.exe` o `TMC.exe` desde GitHub Releases o desde el avisador interno', en: 'Downloaded `TMC-Setup.exe` or `TMC.exe` from GitHub Releases or in-app updater' },
        { id: 'ins-run', es: 'Ejecuté el instalador o intenté iniciar la aplicación en mi sistema', en: 'Ran the installer or launched the application on my machine' },
      ],
      failureModes: [
        {
          id: 'ins-startup-crash',
          label: { es: 'TMC no abre, se queda en pantalla blanca/negra o se cierra al instante', en: 'TMC fails to open, shows a blank/black window, or crashes immediately' },
          desc: { es: 'Al hacer doble clic en el icono no aparece ventana o falta WebView2 Runtime.', en: 'Double-clicking fails to show window, hangs on blank screen, or hits WebView2 error.' },
          steps: [
            { id: 'ins-f1-1', es: 'Abrí TMC por primera vez (o tras actualizar desde una versión previa)', en: 'Launched TMC for the first time (or right after upgrading from a prior version)' },
            { id: 'ins-f1-2', es: 'Comprobé si Microsoft Edge WebView2 estaba instalado y si aparecía un proceso oculto en el Administrador de tareas', en: 'Checked whether Microsoft Edge WebView2 was present and if a process lingered in Task Manager' },
          ],
        },
        {
          id: 'ins-updater-fail',
          label: { es: 'El comprobador de actualizaciones da error al descargar o instalar la nueva versión', en: 'Built-in updater fails to detect, download, or apply a new release' },
          desc: { es: 'Descarga incompleta, error de permisos en Program Files o versión desactualizada tras reiniciar.', en: 'Download error, UAC/permission denial, or still on old version after restart.' },
          steps: [
            { id: 'ins-f2-1', es: 'Pulsé «Buscar actualizaciones» o «Instalar actualización» en Ajustes / Barra superior', en: 'Clicked «Check for updates» or «Install update» in Settings / Top bar' },
            { id: 'ins-f2-2', es: 'El instalador falló por antivirus/SmartScreen o volvió a abrir la versión antigua', en: 'Installer failed due to antivirus/SmartScreen or relaunched old version' },
          ],
        },
        {
          id: 'ins-smartscreen-av',
          label: { es: 'Bloqueo grave por Windows Defender / SmartScreen / antivirus o error de rutas con tildes', en: 'Blocked by SmartScreen/AV or crashes on user folder paths with accents/spaces' },
          desc: { es: 'Impide ejecutar o corrompe lectura en usuarios con acentos / caracteres especiales en %LOCALAPPDATA%.', en: 'False-positive quarantine or startup failure on non-ASCII profile paths.' },
          steps: [
            { id: 'ins-f3-1', es: 'Mi carpeta de usuario o instalación contiene espacios, tildes o está en una unidad secundaria', en: 'My Windows user folder or install path contains spaces, accents, or is on a secondary drive' },
            { id: 'ins-f3-2', es: 'Intenté ejecutar TMC como usuario estándar o desde Steam Deck / Proton / Wine', en: 'Launched TMC as a standard user or via Steam Deck / Proton / Wine' },
          ],
        },
        {
          id: 'ins-memory-cpu',
          label: { es: 'Consumo excesivo de memoria RAM (> 250 MB) o pico constante de CPU en reposo', en: 'High RAM usage (> 250 MB) or continuous idle CPU usage' },
          desc: { es: 'Con el juego minimizado o sin movimiento en cabina, el proceso WebView/TMC consume recursos altos.', en: 'While game is idle or minimized, TMC / WebView process keeps high CPU/memory.' },
          steps: [
            { id: 'ins-f4-1', es: 'Dejé TMC abierto durante más de 30 minutos o una sesión larga', en: 'Left TMC running for 30+ minutes or an extended session' },
            { id: 'ins-f4-2', es: 'Medí el consumo en el Administrador de tareas indicando cuál módulo estaba abierto', en: 'Checked Task Manager footprint noting which module was active at the time' },
          ],
        },
      ],
    },
    {
      id: 'journal',
      category: 'system',
      icon: '📓',
      title: { es: 'Lectura del Diario (Journal)', en: 'Game Journal Reader' },
      summary: {
        es: 'Detección automática de carpeta Saved Games, lectura incremental, Status.json y Replay.',
        en: 'Automatic Saved Games detection, byte-offset tailing, Status.json polling, and Replay.',
      },
      entrySteps: [
        { id: 'jrn-open', es: 'Inicié Elite Dangerous y abrí TMC esperando la lectura automática del diario', en: 'Started Elite Dangerous and opened TMC expecting live journal reading' },
        { id: 'jrn-cfg', es: 'Verifiqué la ruta del diario en Ajustes o reproduje un archivo con Replay/log personalizado', en: 'Verified journal folder path in Settings or tested a custom log' },
      ],
      failureModes: [
        {
          id: 'jrn-folder-not-found',
          label: { es: 'No detecta la carpeta del diario automáticamente o muestra estado «Desconectado»', en: 'Fails to auto-detect Saved Games folder or stays stuck on «Disconnected»' },
          desc: { es: 'La ruta personalizada en OneDrive/Steam/Epic/Linux no encuentra `Journal.*.log`.', en: 'Custom OneDrive/Steam/Epic/Linux Saved Games path cannot locate `Journal.*.log`.' },
          steps: [
            { id: 'jrn-f1-1', es: 'Tengo la carpeta `Saved Games\\Frontier Developments\\Elite Dangerous` movida (OneDrive, Linux, Proton o red)', en: 'My Elite Dangerous journal directory is relocated (OneDrive, Linux, Proton prefix, etc.)' },
            { id: 'jrn-f1-2', es: 'Comprobé en Ajustes qué ruta detectó TMC e intenté cambiarla manualmente', en: 'Checked detected path in Settings and tried pointing it to the right folder' },
          ],
        },
        {
          id: 'jrn-rotation-missed',
          label: { es: 'Deja de leer eventos cuando cambio de sesión o Elite crea un nuevo Journal.*.log', en: 'Stops tracking events when starting a new session or when Elite rotates Journal.*.log' },
          desc: { es: 'Funciona al iniciar el primer diario, pero si reinicio el juego sin cerrar TMC no detecta el nuevo archivo.', en: 'Works on initial file, misses newly rotated session file without app restart.' },
          steps: [
            { id: 'jrn-f2-1', es: 'TMC ya estaba abierto con un diario antiguo cuando inicié una nueva partida en Elite Dangerous', en: 'TMC was already open on an older log when I launched a fresh Elite Dangerous session' },
            { id: 'jrn-f2-2', es: 'Volví al menú principal y entré de nuevo en Open/Solo generando un fichero nuevo', en: 'Exited to Main Menu and re-entered Open/Solo creating a new `Journal.*.log`' },
          ],
        },
        {
          id: 'jrn-custom-hud-colors',
          label: { es: 'No adopta los colores de mi cabina (`GraphicsConfigurationOverride.xml` / EDHM)', en: 'Fails to adopt custom cockpit HUD colors (`GraphicsConfigurationOverride.xml` / EDHM)' },
          desc: { es: 'La aplicación sigue en naranja clásico a pesar de tener matriz personalizada activa en el juego.', en: 'UI stays default orange despite custom HUD matrix in game config.' },
          steps: [
            { id: 'jrn-f3-1', es: 'Tengo activada la sincronización automática de colores de cabina en Ajustes de TMC', en: 'Enabled automatic cockpit HUD color sync in TMC Settings' },
            { id: 'jrn-f3-2', es: 'Uso `GraphicsConfigurationOverride.xml` o `EDHM UI` y probé el botón de recargar paleta', en: 'Use `GraphicsConfigurationOverride.xml` or `EDHM UI` and tested reloading colors' },
          ],
        },
        {
          id: 'jrn-corrupt-line',
          label: { es: 'Error o pérdida de estado tras una línea incompleta o evento desconocido del juego', en: 'State error or missing data after an incomplete JSON line or new game event' },
          desc: { es: 'Tras un parche del juego o evento inédito dejan de actualizarse otros módulos.', en: 'After a game update or unusual event, downstream state stops updating.' },
          steps: [
            { id: 'jrn-f4-1', es: 'Ocurrió tras una actualización reciente de Elite Dangerous o desconexión de servidor (Mauve Adder / Scarlet Krait)', en: 'Happened right after an Elite Dangerous patch or server disconnect (Mauve Adder / etc.)' },
            { id: 'jrn-f4-2', es: 'Revisé `tmc.log` y observé advertencias de parseo o caída parcial de eventos', en: 'Checked `tmc.log` and spotted JSON/event parsing warnings' },
          ],
        },
      ],
    },
    {
      id: 'voice',
      category: 'system',
      icon: '🎙️',
      title: { es: 'Voz, Copiloto STT/TTS y VoiceAttack', en: 'Voice Copilot, STT/TTS & VoiceAttack' },
      summary: {
        es: 'Síntesis de voz (TTS local o IA), reconocimiento por micrófono (STT), 9 consultas y VoiceAttack.',
        en: 'Text-to-speech (local/AI TTS), microphone voice commands (STT), 9 queries, and VoiceAttack.',
      },
      entrySteps: [
        { id: 'voi-open', es: 'Activé el Copiloto de Voz / TTS o abrí la pestaña «Voz» en Ajustes', en: 'Enabled Voice Copilot / TTS or opened the «Voice» section in Settings' },
        { id: 'voi-test', es: 'Probé un aviso automático en vuelo o realicé una consulta hablada por micrófono / VoiceAttack', en: 'Tested an automatic flight announcement or spoke a voice query / VoiceAttack trigger' },
      ],
      failureModes: [
        {
          id: 'voi-tts-silent',
          label: { es: 'La voz (TTS) no habla o reproduce en un dispositivo de audio equivocado', en: 'TTS produces no audio or speaks through the wrong output device' },
          desc: { es: 'Los avisos están activados (y el botón Probar tampoco suena o da error de motor Windows/Edge/IA).', en: 'Announcements enabled, yet Test button or flight triggers produce silence or engine error.' },
          steps: [
            { id: 'voi-f1-1', es: 'Pulsé «Probar voz» en Ajustes seleccionando voz SAPI5 de Windows, Piper o proveedor configurado', en: 'Clicked «Test Voice» in Settings with Windows SAPI5, Piper, or configured TTS provider' },
            { id: 'voi-f1-2', es: 'Comprobé el mezclador de volumen de Windows, auriculares VR / inalámbricos y modo «No molestar en combate»', en: 'Checked Windows Volume Mixer, VR/USB headset routing, and quiet/DND mode' },
          ],
        },
        {
          id: 'voi-stt-not-recognizing',
          label: { es: 'El micrófono no entiende las 9 consultas de voz o se queda bloqueado escuchando', en: 'Microphone STT fails to recognize the 9 voice queries or gets stuck listening' },
          desc: { es: 'En Push-to-Talk o activación continua no detecta frase, falla el idioma o se dispara solo con ruido.', en: 'PTT or continuous mode misses speech, mismatches language, or triggers on background noise.' },
          steps: [
            { id: 'voi-f2-1', es: 'Seleccioné el micrófono de entrada y modo (Push-to-Talk / activación por palabra) en Ajustes', en: 'Selected input microphone and mode (PTT / wake-word) in Settings' },
            { id: 'voi-f2-2', es: 'Hablé en mi idioma configurado (ej. "¿Cuánto combustible me queda?") y el indicador de escucha no respondió o falló', en: 'Spoke a configured query (e.g. fuel/system status) and recognition missed or hung' },
          ],
        },
        {
          id: 'voi-wrong-answers',
          label: { es: 'La respuesta hablada del copiloto da un dato erróneo, cortado o en idioma equivocado', en: 'Spoken copilot reply reads stale data, clips mid-sentence, or uses wrong language' },
          desc: { es: 'Mezcla idioma, lee mal números decimales/créditos o se superponen dos frases a la vez.', en: 'Mixes languages, misreads decimals/credits, or overlaps consecutive speech queues.' },
          steps: [
            { id: 'voi-f3-1', es: 'Saltaron dos eventos seguidos (p. ej. salto + escaneo FSS) o pregunté por voz dos veces seguidas', en: 'Triggered two rapid events (e.g. jump + FSS scan) or issued consecutive voice queries' },
            { id: 'voi-f3-2', es: 'Escuché unidades mal pronunciadas, cifras truncadas o idioma distinto al seleccionado en TMC', en: 'Heard mispronounced numbers, clipped phrases, or mismatched language' },
          ],
        },
        {
          id: 'voi-voiceattack-ipc',
          label: { es: 'La integración con VoiceAttack / Atajos globales no responde cuando Elite tiene el foco', en: 'VoiceAttack integration or global PTT hotkeys fail while Elite Dangerous is focused' },
          desc: { es: 'El atajo global o el comando de archivo/API desde VoiceAttack no llega a TMC en pantalla completa.', en: 'Global hotkey or VoiceAttack call does not reach TMC during fullscreen play.' },
          steps: [
            { id: 'voi-f4-1', es: 'Con Elite Dangerous en primer plano (pantalla completa o sin bordes), pulsé la tecla Push-to-Talk', en: 'While Elite Dangerous had window focus (fullscreen/borderless), pressed PTT hotkey' },
            { id: 'voi-f4-2', es: 'Lancé un perfil de VoiceAttack apuntando al comando/endpoint de TMC y no ejecutó la consulta', en: 'Triggered a VoiceAttack profile command targeting TMC and saw no reaction' },
          ],
        },
      ],
    },
    {
      id: 'layout',
      category: 'system',
      icon: '🎛️',
      title: { es: 'Diseño y Disposición de Tarjetas', en: 'Layout & Glass HUD Cards' },
      summary: {
        es: '10 vistas combinadas de cabina, tarjetas cortadas, scroll doble/vacío e idioma i18n.',
        en: '10 combined cockpit views, card clipping, phantom/double scrollbars, and i18n text.',
      },
      entrySteps: [
        { id: 'lay-open', es: 'Cambié entre disposiciones combinadas (Exploración, Minería, Combate, etc.) o redimensioné la ventana', en: 'Switched combined cockpit layouts or resized the application window' },
        { id: 'lay-inspect', es: 'Observé el encuadre de tarjetas, desplegables de detalle, botones o barras de scroll', en: 'Inspected card alignment, collapsible details, buttons, or scrollbars' },
      ],
      failureModes: [
        {
          id: 'lay-clipping-overflow',
          label: { es: 'Texto, insignia o botón cortado / solapado / saliéndose del borde de su tarjeta', en: 'Text, badge, or button is clipped, overlapping, or overflowing its card border' },
          desc: { es: 'Algún badge largo o formulario empuja controles fuera del ancho visible en ventana o cuadrícula.', en: 'Long badge text or crowded row clips controls out of the visible card bounds.' },
          steps: [
            { id: 'lay-f1-1', es: 'Usé TMC en un tamaño concreto de ventana (ej. segundo monitor, 1366×768 o mitad de pantalla)', en: 'Used TMC at a specific viewport size (e.g. secondary monitor, 1366x768, or split-screen)' },
            { id: 'lay-f1-2', es: 'Identifiqué la tarjeta exacta y el idioma (es/en/fr/de/pt) donde se produce el recorte', en: 'Noted exact card name and active language (es/en/fr/de/pt) where clipping happens' },
          ],
        },
        {
          id: 'lay-phantom-scroll',
          label: { es: 'Aparece barra de scroll vacía, doble scroll anidado o scroll horizontal oculto abajo', en: 'Phantom scrollbar on short lists, nested double scrollbars, or buried horizontal scroll' },
          desc: { es: 'Con solo 1 o 2 elementos hay hueco negro enorme o no se puede hacer scroll horizontal desde arriba.', en: '1-2 row lists show large blank height or table requires vertical scroll to find horizontal bar.' },
          steps: [
            { id: 'lay-f2-1', es: 'Abrí una lista o tabla con pocos elementos (1-2 filas) o muchas columnas (> 1000 px ancho)', en: 'Opened a list/table with very few rows (1-2) or wide multi-column grid' },
            { id: 'lay-f2-2', es: 'Usé la rueda del ratón sobre el panel y el scroll exterior compitió con el interior', en: 'Scrolled with mouse wheel over the panel and saw outer/inner scrollbars fight' },
          ],
        },
        {
          id: 'lay-wrong-i18n',
          label: { es: 'Aparecen claves sin traducir (`key-name`), texto mezclado en otro idioma o formato numérico raro', en: 'Untranslated i18n keys, mixed languages, or broken number formatting appear' },
          desc: { es: 'Alguna etiqueta muestra el identificador técnico o faltan tildes/separadores de miles.', en: 'Labels display raw `.ftl` identifiers or mismatched locale separators.' },
          steps: [
            { id: 'lay-f3-1', es: 'Cambié el idioma de TMC en Ajustes (es / en / fr / de / pt) y recorrí el módulo afectado', en: 'Switched TMC language in Settings (es / en / fr / de / pt) and opened affected module' },
            { id: 'lay-f3-2', es: 'Detecté el texto literal en inglés/clave técnica dentro de una tarjeta traducida', en: 'Spotted the exact literal key or untranslated string in a specific card' },
          ],
        },
        {
          id: 'lay-layout-switch',
          label: { es: 'El cambio automático de disposición según actividad no cambia o elige la vista incorrecta', en: 'Automatic cockpit layout switching ignores flight activity or selects wrong view' },
          desc: { es: 'Al desplegar láseres de minería o saltar a explorar permanece fija una vista anterior.', en: 'Mining, exploring, or combat events fail to trigger auto layout switching.' },
          steps: [
            { id: 'lay-f4-1', es: 'Activé el modo automático de vista en la barra superior / Ajustes', en: 'Enabled automatic cockpit mode switching in top bar / Settings' },
            { id: 'lay-f4-2', es: 'Cambié de actividad (p. ej. de supercrucero a anillos con láseres o aterrizaje exobio) sin transición', en: 'Switched flight activity (supercruise -> mining ring -> exo landing) with no layout change' },
          ],
        },
      ],
    },
    {
      id: 'lan',
      category: 'system',
      icon: '📱',
      title: { es: 'Modo Tablet / Móvil por LAN', en: 'Tablet & Mobile LAN Mode' },
      summary: {
        es: 'Servidor local HTTP/WebSocket, código QR, PIN de acceso, reconexión y control táctil.',
        en: 'Local HTTP/WebSocket server, QR pairing, PIN guard, live sync, and touch ergonomics.',
      },
      entrySteps: [
        { id: 'lan-enable', es: 'Activé «Acceso por red local (LAN)» en Ajustes de TMC en el PC principal', en: 'Enabled «Local Network Access (LAN)» in TMC Settings on the main PC' },
        { id: 'lan-device', es: 'Escaneé el código QR o abrí la dirección `http://<IP>:47320` en mi tablet/móvil', en: 'Scanned the QR code or opened `http://<IP>:47320` on my tablet/phone' },
      ],
      failureModes: [
        {
          id: 'lan-unreachable',
          label: { es: 'La tablet o móvil no conecta con la URL del QR (timeout o página inaccesible)', en: 'Tablet/phone cannot reach the QR URL (timeout or connection refused)' },
          desc: { es: 'El navegador móvil queda cargando hasta expirar aunque ambos estén en la misma Wi-Fi/LAN.', en: 'Mobile browser times out despite both devices sharing the same Wi-Fi/LAN.' },
          steps: [
            { id: 'lan-f1-1', es: 'Comprobé que PC y tablet están en la misma red Wi-Fi / subred (no red de invitados aislada)', en: 'Verified PC and tablet share the same local Wi-Fi subnet (not isolated guest Wi-Fi)' },
            { id: 'lan-f1-2', es: 'Verifiqué qué IP eligió el QR de TMC (VPN, adaptadores virtuales Hyper-V/WSL o Firewall de Windows)', en: 'Checked which IP the QR displayed (VPN, Hyper-V/WSL virtual adapter, or Windows Firewall)' },
          ],
        },
        {
          id: 'lan-pin-loop',
          label: { es: 'El PIN de seguridad falla, pide PIN en bucle o no guarda sesión en el navegador táctil', en: 'Security PIN fails, loops endlessly, or fails to persist session in tablet browser' },
          desc: { es: 'Tras introducir el código correcto rechaza el acceso o lo vuelve a pedir al cambiar pestaña.', en: 'Correct PIN is rejected or prompted again on every page refresh.' },
          steps: [
            { id: 'lan-f2-1', es: 'Configuré o activé protección por PIN en Ajustes LAN e introduje el código desde el dispositivo móvil', en: 'Configured LAN PIN protection and entered the PIN on the mobile browser' },
            { id: 'lan-f2-2', es: 'Actualicé la pestaña, suspendí la pantalla de la tablet o abrí en modo privado/PWA', en: 'Refreshed the tab, woke tablet from sleep, or used private browsing / home-screen PWA' },
          ],
        },
        {
          id: 'lan-ws-lag-freeze',
          label: { es: 'El WebSocket en tablet deja de actualizarse tras bloquear la pantalla o cambiar de app', en: 'Tablet WebSocket stops syncing after screen lock or app backgrounding' },
          desc: { es: 'Al volver a la tablet pasados unos minutos muestra datos congelados hasta recargar a mano.', en: 'Returning to tablet shows frozen telemetry until manual browser reload.' },
          steps: [
            { id: 'lan-f3-1', es: 'Estaba conectado correctamente en la tablet con eventos en directo desde el juego', en: 'Was connected live on the tablet receiving game updates' },
            { id: 'lan-f3-2', es: 'Bloqueé la pantalla o cambié unos segundos a otra app (iOS Safari / Android Chrome) y volví', en: 'Locked device screen or backgrounded browser (iOS Safari / Android Chrome) then returned' },
          ],
        },
        {
          id: 'lan-touch-ui',
          label: { es: 'Botones táctiles pequeños, menú lateral bloqueado o diseño descuadrado en pantalla vertical/tablet', en: 'Small touch targets, stuck sidebar, or broken layout on portrait/tablet screen' },
          desc: { es: 'En tablet/móvil resulta difícil pulsar botones, cerrar el menú lateral o hacer scroll sin zoom.', en: 'Touch ergonomics, mobile drawer, or responsive grid breaks on narrow screen.' },
          steps: [
            { id: 'lan-f4-1', es: 'Usé el dispositivo en orientación vertical o tablet apaisada (especifica navegador/sistema)', en: 'Used the device in portrait or landscape tablet mode (note browser/OS)' },
            { id: 'lan-f4-2', es: 'Intenté pulsar interruptores, abrir Ajustes o usar el desplazamiento táctil en una tabla', en: 'Tried tapping switches, opening Settings, or touch-scrolling inside a data table' },
          ],
        },
      ],
    },
  ];

  // Fallback universal para cualquier función si ninguna opción previa encaja
  const OTHER_FAILURE_MODE = {
    id: 'other-custom',
    label: {
      es: 'Otro fallo distinto en esta función (lo detallo en el siguiente paso)',
      en: 'Other failure in this feature (I will describe it in the next step)',
    },
    desc: {
      es: 'Usa esta opción si el comportamiento inesperado no encaja en ninguno de los síntomas anteriores.',
      en: 'Pick this if the unexpected behavior does not match any of the presets above.',
    },
    steps: [
      { id: 'other-1', es: 'Reproduje el fallo directamente en el flujo normal de esta función', en: 'Reproduced the issue following standard usage of this feature' },
      { id: 'other-2', es: 'El error volvió a producirse tras recargar la vista o reiniciar la aplicación', en: 'Confirmed the issue persists after refreshing the view or restarting TMC' },
    ],
  };

  function resolveLang(raw) {
    const code = String(raw || 'es').toLowerCase().split(/[-_]/)[0];
    return ['es', 'en', 'fr', 'de', 'pt'].includes(code) ? code : 'es';
  }

  function getUiStrings(lang) {
    const norm = resolveLang(lang);
    return norm === 'es' ? REPORTER_I18N.es : REPORTER_I18N.en;
  }

  function pickLoc(obj, lang) {
    if (!obj) return '';
    if (typeof obj === 'string') return obj;
    const norm = resolveLang(lang);
    if (norm === 'es') return obj.es || obj.en || '';
    return obj[norm] || obj.en || obj.es || '';
  }

  function getFeatureById(id) {
    if (!id) return null;
    const clean = String(id).trim().replace(/^mod-/, '');
    return FEATURE_CATALOG.find((f) => f.id === clean) || null;
  }

  function getFailureModesForFeature(feature) {
    if (!feature) return [];
    return [...feature.failureModes, OTHER_FAILURE_MODE];
  }

  function getFailureModeById(feature, modeId) {
    if (!feature || !modeId) return null;
    return getFailureModesForFeature(feature).find((m) => m.id === modeId) || null;
  }

  function trimLogLines(rawText, maxLines, maxChars) {
    const normalized = String(rawText || '').replace(/\r\n/g, '\n').trim();
    if (!normalized) return { content: '', lineCount: 0, wasTrimmed: false };
    const lines = normalized.split('\n');
    let sliced = lines.slice(-maxLines).join('\n');
    let wasTrimmed = lines.length > maxLines;
    if (sliced.length > maxChars) {
      sliced = sliced.slice(sliced.length - maxChars);
      const nl = sliced.indexOf('\n');
      if (nl !== -1 && nl < sliced.length - 1) {
        sliced = sliced.slice(nl + 1);
      }
      wasTrimmed = true;
    }
    return {
      content: sliced,
      lineCount: sliced.split('\n').length,
      wasTrimmed,
    };
  }

  function parseLaunchContext(searchStr, hashStr) {
    const search = new URLSearchParams(String(searchStr || '').replace(/^\?/, ''));
    const rawHash = String(hashStr || '').replace(/^#/, '');
    let logContent = '';
    if (rawHash) {
      if (rawHash.startsWith('log=')) {
        try {
          logContent = decodeURIComponent(rawHash.slice(4));
        } catch {
          logContent = rawHash.slice(4);
        }
      } else {
        const hashParams = new URLSearchParams(rawHash);
        logContent = hashParams.get('log') || '';
      }
    }

    const platformRaw = search.get('platform') || '';
    let detectedPlatform = 'windows-desktop';
    const lowPlat = platformRaw.toLowerCase();
    if (search.get('from') === 'lan' || lowPlat.includes('android') || lowPlat.includes('ipad') || lowPlat.includes('iphone')) {
      detectedPlatform = 'tablet-lan';
    } else if (lowPlat.includes('linux') || lowPlat.includes('steam') || lowPlat.includes('mac')) {
      detectedPlatform = 'windows-steamdeck';
    }

    return {
      lang: resolveLang(search.get('lang') || 'es'),
      version: search.get('version') || '',
      platformRaw,
      platformPreset: detectedPlatform,
      fromApp: search.get('from') === 'app',
      moduleId: search.get('module') || '',
      logFile: search.get('logFile') || '',
      logContent: logContent.trim(),
    };
  }

  function buildIssueTitle(state) {
    const lang = resolveLang(state.lang);
    const feat = getFeatureById(state.featureId);
    const mode = getFailureModeById(feat, state.failureModeId);
    const featTitle = feat ? pickLoc(feat.title, lang) : 'TMC';
    let modeTitle = mode ? pickLoc(mode.label, lang) : '';
    if (mode && mode.id === OTHER_FAILURE_MODE.id && state.customFailureDetail) {
      modeTitle = state.customFailureDetail.trim();
    }
    if (modeTitle.length > 72) {
      modeTitle = `${modeTitle.slice(0, 69)}...`;
    }
    return `[Bug] [${featTitle}] ${modeTitle}`.trim();
  }

  function buildMarkdownBody(state, options) {
    const opts = options || {};
    const lang = resolveLang(state.lang);
    const strings = getUiStrings(lang);
    const sec = strings.issueSections;
    const feat = getFeatureById(state.featureId);
    const mode = getFailureModeById(feat, state.failureModeId);

    const featTitle = feat ? `${feat.icon} ${pickLoc(feat.title, lang)} (\`${feat.id}\`)` : state.featureId || '—';
    const catLabel = feat ? (feat.category === 'module' ? strings.categoryModule : strings.categorySystem) : '—';
    const modeLabel = mode ? pickLoc(mode.label, lang) : '—';

    const checkedEntryIds = new Set(state.checkedFeatureSteps || []);
    const checkedFailureIds = new Set(state.checkedFailureSteps || []);

    const featureStepLines = (feat ? feat.entrySteps : [])
      .filter((s) => checkedEntryIds.has(s.id))
      .map((s) => `- [x] ${pickLoc(s, lang)}`);

    const failureStepLines = (mode ? mode.steps : [])
      .filter((s) => checkedFailureIds.has(s.id))
      .map((s) => `- [x] ${pickLoc(s, lang)}`);

    const freqEntry = strings.freqOptions.find((f) => f.id === state.frequency);
    const platEntry = strings.platformOptions.find((p) => p.id === state.platformPreset);
    const gameEntry = strings.gameOptions.find((g) => g.id === state.gameEdition);

    const lines = [
      `## ${sec.feature}`,
      `${featTitle} — _${catLabel}_`,
      '',
      `## ${sec.failureMode}`,
      `**${modeLabel}**`,
    ];

    if (state.customFailureDetail && state.customFailureDetail.trim()) {
      lines.push('', state.customFailureDetail.trim());
    }

    lines.push('', `## ${sec.featurePath}`);
    if (featureStepLines.length > 0) {
      lines.push(...featureStepLines);
    } else {
      lines.push(sec.notSpecified);
    }

    lines.push('', `## ${sec.failurePath}`);
    if (failureStepLines.length > 0) {
      lines.push(...failureStepLines);
    } else {
      lines.push(sec.notSpecified);
    }

    if (state.customStep && state.customStep.trim()) {
      lines.push('', `### ${sec.customStep}`, `- [x] ${state.customStep.trim()}`);
    }

    if (state.extraNotes && state.extraNotes.trim()) {
      lines.push('', `## ${sec.extraNotes}`, state.extraNotes.trim());
    }

    const platformText = state.platformRaw
      ? `${platEntry ? platEntry.label : state.platformPreset} (\`${state.platformRaw}\`)`
      : (platEntry ? platEntry.label : state.platformPreset || 'Windows');

    lines.push(
      '',
      `## ${sec.environment}`,
      `- **${sec.version}:** ${state.version && state.version.trim() ? state.version.trim() : '—'}`,
      `- **${sec.platform}:** ${platformText}`,
      `- **${sec.gameEdition}:** ${gameEntry ? gameEntry.label : state.gameEdition || 'Odyssey'}`,
      `- **${sec.frequency}:** ${freqEntry ? freqEntry.label : state.frequency || 'always'}`,
    );

    if (state.systemContext && state.systemContext.trim()) {
      lines.push(`- **${sec.systemContext}:** ${state.systemContext.trim()}`);
    }
    if (state.fromApp) {
      lines.push(`- **${sec.origin}:** TMC Settings -> Guided Bug Reporter`);
    }

    lines.push('', `## ${sec.logSection}${state.logFile ? ` (\`${state.logFile}\`)` : ''}`);
    const logLimit = typeof opts.maxLogChars === 'number' ? opts.maxLogChars : MAX_LOG_CHARS_PREVIEW;
    const lineLimit = typeof opts.maxLogLines === 'number' ? opts.maxLogLines : MAX_LOG_LINES;
    const cleanedLog = String(state.logText || '').trim();
    if (!cleanedLog) {
      lines.push(sec.logNotAttached);
    } else if (logLimit <= 0) {
      lines.push(sec.logTruncatedNote);
    } else {
      const trimmed = trimLogLines(cleanedLog, lineLimit, logLimit);
      if (trimmed.wasTrimmed && opts.urlTrimNotice) {
        lines.push(sec.logTruncatedNote, '');
      }
      lines.push('```log', trimmed.content, '```');
    }

    return lines.join('\n');
  }

  function buildGitHubIssueUrl(state) {
    const title = buildIssueTitle(state);
    const fullBody = buildMarkdownBody(state, {
      maxLogChars: MAX_LOG_CHARS_FULL,
      maxLogLines: 160,
    });

    const makeUrl = (bodyText) => {
      const params = new URLSearchParams({
        labels: 'bug',
        title,
        body: bodyText,
      });
      return `${GITHUB_ISSUES_NEW}?${params.toString()}`;
    };

    let url = makeUrl(fullBody);
    let wasTruncatedForUrl = false;
    if (url.length > MAX_URL_LENGTH) {
      // Ir reduciendo progresivamente el fragmento de log hasta que quepa en la URL de GitHub
      for (const limit of [4000, 2600, 1400, 600, 0]) {
        const candidateBody = buildMarkdownBody(state, {
          maxLogChars: limit,
          urlTrimNotice: true,
        });
        const candidateUrl = makeUrl(candidateBody);
        if (candidateUrl.length <= MAX_URL_LENGTH || limit === 0) {
          url = candidateUrl;
          wasTruncatedForUrl = true;
          break;
        }
      }
    }

    return { url, title, fullBody, wasTruncatedForUrl };
  }

  // ==================== INTERFAZ DOM DEL ASISTENTE ====================
  function escapeHtml(str) {
    return String(str || '')
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  async function copyToClipboard(text) {
    if (typeof navigator !== 'undefined' && navigator.clipboard && navigator.clipboard.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
    if (typeof document !== 'undefined') {
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.style.position = 'fixed';
      ta.style.left = '-9999px';
      document.body.appendChild(ta);
      ta.select();
      const ok = document.execCommand('copy');
      document.body.removeChild(ta);
      return ok;
    }
    return false;
  }

  function initBugReporterApp() {
    if (typeof document === 'undefined') return;
    const rootEl = document.getElementById('bugReporterApp');
    if (!rootEl) return;

    const launch = parseLaunchContext(window.location.search, window.location.hash);

    // Sincronizar idioma si viene de ?lang= antes o después de i18n.js
    const initialLang = window.TMC_I18N && window.TMC_I18N.getCurrentLanguage
      ? window.TMC_I18N.getCurrentLanguage()
      : launch.lang;

    const state = {
      lang: resolveLang(initialLang || launch.lang),
      categoryFilter: 'all',
      searchQuery: '',
      featureId: getFeatureById(launch.moduleId)?.id || '',
      failureModeId: '',
      checkedFeatureSteps: [],
      checkedFailureSteps: [],
      customFailureDetail: '',
      customStep: '',
      extraNotes: '',
      version: launch.version,
      platformPreset: launch.platformPreset,
      platformRaw: launch.platformRaw,
      gameEdition: 'odyssey',
      frequency: 'always',
      systemContext: '',
      fromApp: launch.fromApp,
      logFile: launch.logFile,
      initialLogText: launch.logContent,
      logText: launch.logContent,
      toastMessage: '',
      toastKind: 'info',
    };

    const dom = {
      step1Pill: document.getElementById('stepperStep1'),
      step2Pill: document.getElementById('stepperStep2'),
      step3Pill: document.getElementById('stepperStep3'),
      summaryFeature: document.getElementById('summaryFeature'),
      summaryFailure: document.getElementById('summaryFailure'),
      summarySteps: document.getElementById('summarySteps'),
      filterBtns: document.querySelectorAll('[data-br-filter]'),
      searchInput: document.getElementById('featureSearchInput'),
      featureSelect: document.getElementById('featureSelect'),
      featureGrid: document.getElementById('featureCardsGrid'),
      step2Section: document.getElementById('step2Section'),
      step2Hint: document.getElementById('step2EmptyHint'),
      step2Body: document.getElementById('step2ModesContainer'),
      step2SelectedName: document.getElementById('step2FeatureBadge'),
      step3Section: document.getElementById('step3Section'),
      step3Hint: document.getElementById('step3EmptyHint'),
      step3Body: document.getElementById('step3DetailsContainer'),
      featureStepsBox: document.getElementById('featureStepsChecklist'),
      failureStepsBox: document.getElementById('failureStepsChecklist'),
      failureDetailInput: document.getElementById('failureDetailInput'),
      customStepInput: document.getElementById('customStepInput'),
      extraNotesInput: document.getElementById('extraNotesInput'),
      appNotice: document.getElementById('appContextNotice'),
      versionInput: document.getElementById('envVersionInput'),
      platformSelect: document.getElementById('envPlatformSelect'),
      gameSelect: document.getElementById('envGameSelect'),
      frequencySelect: document.getElementById('envFrequencySelect'),
      systemContextInput: document.getElementById('envSystemContextInput'),
      logStatusText: document.getElementById('logStatusText'),
      logTextarea: document.getElementById('logTextarea'),
      logFileInput: document.getElementById('logFileInput'),
      logClearBtn: document.getElementById('logClearBtn'),
      logRestoreBtn: document.getElementById('logRestoreBtn'),
      previewTitle: document.getElementById('previewIssueTitle'),
      previewPre: document.getElementById('previewMarkdownBody'),
      openGithubBtn: document.getElementById('openGithubIssueBtn'),
      copyReportBtn: document.getElementById('copyFullReportBtn'),
      toastBanner: document.getElementById('reporterToast'),
    };

    function showToast(msg, kind) {
      if (!dom.toastBanner) return;
      dom.toastBanner.textContent = msg;
      dom.toastBanner.className = `br-toast show br-toast-${kind || 'info'}`;
      clearTimeout(showToast._timer);
      showToast._timer = setTimeout(() => {
        dom.toastBanner.classList.remove('show');
      }, 6500);
    }

    function syncStaticLocalizedTexts() {
      const ui = getUiStrings(state.lang);
      if (dom.searchInput) dom.searchInput.placeholder = ui.searchPlaceholder;
      if (dom.failureDetailInput) dom.failureDetailInput.placeholder = ui.failureDetailPlaceholder;
      if (dom.customStepInput) dom.customStepInput.placeholder = ui.customStepPlaceholder;
      if (dom.extraNotesInput) dom.extraNotesInput.placeholder = ui.extraNotesPlaceholder;
      if (dom.systemContextInput) dom.systemContextInput.placeholder = ui.envSystemContextPlaceholder;
      if (dom.logTextarea) dom.logTextarea.placeholder = ui.logPlaceholder;

      const repopulateSelect = (selectEl, items, currentVal) => {
        if (!selectEl) return;
        selectEl.innerHTML = items
          .map((item) => `<option value="${escapeHtml(item.id)}">${escapeHtml(item.label)}</option>`)
          .join('');
        selectEl.value = currentVal;
      };
      repopulateSelect(dom.platformSelect, ui.platformOptions, state.platformPreset);
      repopulateSelect(dom.gameSelect, ui.gameOptions, state.gameEdition);
      repopulateSelect(dom.frequencySelect, ui.freqOptions, state.frequency);

      if (dom.appNotice) {
        dom.appNotice.style.display = state.fromApp || state.initialLogText ? 'flex' : 'none';
        const noticeText = dom.appNotice.querySelector('.br-notice-text');
        if (noticeText) noticeText.textContent = ui.envAppNotice;
      }

      if (dom.logRestoreBtn) {
        dom.logRestoreBtn.style.display = state.initialLogText ? 'inline-flex' : 'none';
      }

      const langBadge = document.querySelector('#langSelectBtn .lang-current');
      if (langBadge) {
        langBadge.textContent = state.lang.toUpperCase();
      }
    }

    function renderFeatureSelectOptions() {
      if (!dom.featureSelect) return;
      const ui = getUiStrings(state.lang);
      const modules = FEATURE_CATALOG.filter((f) => f.category === 'module');
      const system = FEATURE_CATALOG.filter((f) => f.category === 'system');
      const makeOpts = (arr) =>
        arr
          .map(
            (f) =>
              `<option value="${escapeHtml(f.id)}">${escapeHtml(f.icon)} ${escapeHtml(pickLoc(f.title, state.lang))}</option>`,
          )
          .join('');
      dom.featureSelect.innerHTML = `
        <option value="">${escapeHtml(ui.selectPlaceholder)}</option>
        <optgroup label="${escapeHtml(`${ui.modulesFilter} (${modules.length})`)}">${makeOpts(modules)}</optgroup>
        <optgroup label="${escapeHtml(`${ui.systemFilter} (${system.length})`)}">${makeOpts(system)}</optgroup>
      `;
      dom.featureSelect.value = state.featureId;
    }

    function renderStep1Grid() {
      if (!dom.featureGrid) return;
      const ui = getUiStrings(state.lang);
      const query = state.searchQuery.trim().toLowerCase();

      const filtered = FEATURE_CATALOG.filter((f) => {
        if (state.categoryFilter !== 'all' && f.category !== state.categoryFilter) return false;
        if (!query) return true;
        const text = [
          f.id,
          pickLoc(f.title, state.lang),
          pickLoc(f.summary, state.lang),
          f.title.es,
          f.title.en,
        ]
          .join(' ')
          .toLowerCase();
        return text.includes(query);
      });

      if (filtered.length === 0) {
        dom.featureGrid.innerHTML = `<div class="br-empty-box">${escapeHtml(ui.noFeatureMatch)}</div>`;
        return;
      }

      dom.featureGrid.innerHTML = filtered
        .map((feat) => {
          const isSelected = feat.id === state.featureId;
          const catText = feat.category === 'module' ? ui.categoryModule : ui.categorySystem;
          const catClass = feat.category === 'module' ? 'badge-cyan' : 'badge-purple';
          const countLabel = ui.failureModesCount.replace('{count}', String(feat.failureModes.length));
          return `
            <button type="button"
              class="br-feature-card${isSelected ? ' selected' : ''}"
              data-feature-id="${escapeHtml(feat.id)}"
              aria-pressed="${isSelected ? 'true' : 'false'}">
              <div class="br-feature-card-top">
                <span class="br-feature-icon" aria-hidden="true">${escapeHtml(feat.icon)}</span>
                <span class="badge ${catClass}">${escapeHtml(catText)}</span>
                ${isSelected ? `<span class="badge badge-orange br-selected-pill">${escapeHtml(ui.selectedBadge)}</span>` : ''}
              </div>
              <h3 class="br-feature-title">${escapeHtml(pickLoc(feat.title, state.lang))}</h3>
              <p class="br-feature-desc">${escapeHtml(pickLoc(feat.summary, state.lang))}</p>
              <span class="br-feature-meta">${escapeHtml(countLabel)}</span>
            </button>
          `;
        })
        .join('');

      dom.featureGrid.querySelectorAll('[data-feature-id]').forEach((btn) => {
        btn.addEventListener('click', () => {
          const id = btn.getAttribute('data-feature-id') || '';
          selectFeature(id, true);
        });
      });
    }

    function renderStep2Modes() {
      const ui = getUiStrings(state.lang);
      const feat = getFeatureById(state.featureId);

      if (!feat) {
        if (dom.step2Hint) {
          dom.step2Hint.textContent = ui.chooseFeatureFirst;
          dom.step2Hint.style.display = 'block';
        }
        if (dom.step2Body) dom.step2Body.innerHTML = '';
        if (dom.step2SelectedName) dom.step2SelectedName.style.display = 'none';
        return;
      }

      if (dom.step2Hint) dom.step2Hint.style.display = 'none';
      if (dom.step2SelectedName) {
        dom.step2SelectedName.textContent = `${feat.icon} ${pickLoc(feat.title, state.lang)}`;
        dom.step2SelectedName.style.display = 'inline-flex';
      }

      const modes = getFailureModesForFeature(feat);
      if (dom.step2Body) {
        dom.step2Body.innerHTML = modes
          .map((mode) => {
            const isSelected = mode.id === state.failureModeId;
            const isOther = mode.id === OTHER_FAILURE_MODE.id;
            return `
              <button type="button"
                class="br-mode-card${isSelected ? ' selected' : ''}${isOther ? ' is-other' : ''}"
                data-mode-id="${escapeHtml(mode.id)}"
                aria-pressed="${isSelected ? 'true' : 'false'}">
                <div class="br-mode-radio" aria-hidden="true">
                  <span class="br-radio-dot"></span>
                </div>
                <div class="br-mode-text">
                  <div class="br-mode-title">${escapeHtml(pickLoc(mode.label, state.lang))}</div>
                  <div class="br-mode-desc">${escapeHtml(pickLoc(mode.desc, state.lang))}</div>
                </div>
              </button>
            `;
          })
          .join('');

        dom.step2Body.querySelectorAll('[data-mode-id]').forEach((btn) => {
          btn.addEventListener('click', () => {
            const modeId = btn.getAttribute('data-mode-id') || '';
            selectFailureMode(modeId, true);
          });
        });
      }
    }

    function renderStep3Checklist() {
      const ui = getUiStrings(state.lang);
      const feat = getFeatureById(state.featureId);
      const mode = getFailureModeById(feat, state.failureModeId);

      if (!feat || !mode) {
        if (dom.step3Hint) {
          dom.step3Hint.textContent = feat ? ui.chooseFailureFirst : ui.chooseFeatureFirst;
          dom.step3Hint.style.display = 'block';
        }
        if (dom.step3Body) dom.step3Body.style.display = 'none';
        return;
      }

      if (dom.step3Hint) dom.step3Hint.style.display = 'none';
      if (dom.step3Body) dom.step3Body.style.display = 'flex';

      const renderCheckGroup = (container, heading, steps, selectedIds, onToggle) => {
        if (!container) return;
        const checkedSet = new Set(selectedIds);
        container.innerHTML = `
          <div class="br-check-group-title">${escapeHtml(heading)}</div>
          <div class="br-check-list">
            ${steps
              .map(
                (step) => `
                  <label class="br-check-item">
                    <input type="checkbox" value="${escapeHtml(step.id)}" ${checkedSet.has(step.id) ? 'checked' : ''}>
                    <span>${escapeHtml(pickLoc(step, state.lang))}</span>
                  </label>
                `,
              )
              .join('')}
          </div>
        `;
        container.querySelectorAll('input[type="checkbox"]').forEach((cb) => {
          cb.addEventListener('change', () => {
            const chosen = Array.from(container.querySelectorAll('input[type="checkbox"]:checked')).map(
              (el) => el.value,
            );
            onToggle(chosen);
          });
        });
      };

      renderCheckGroup(
        dom.featureStepsBox,
        ui.featureStepsHeading,
        feat.entrySteps,
        state.checkedFeatureSteps,
        (chosen) => {
          state.checkedFeatureSteps = chosen;
          renderStepperAndPreview();
        },
      );

      renderCheckGroup(
        dom.failureStepsBox,
        ui.failureStepsHeading,
        mode.steps,
        state.checkedFailureSteps,
        (chosen) => {
          state.checkedFailureSteps = chosen;
          renderStepperAndPreview();
        },
      );
    }

    function renderLogMetaAndPreview() {
      const ui = getUiStrings(state.lang);
      if (dom.logStatusText) {
        const cleaned = state.logText.trim();
        if (!cleaned) {
          dom.logStatusText.textContent = ui.logPrivacyNote;
        } else {
          const lineCount = cleaned.split('\n').length;
          if (state.logFile) {
            dom.logStatusText.textContent = ui.logLoadedFromApp
              .replace('{file}', state.logFile)
              .replace('{lines}', String(lineCount));
          } else {
            dom.logStatusText.textContent = `${lineCount} lines · ${cleaned.length} chars`;
          }
        }
      }
      renderStepperAndPreview();
    }

    function renderStepperAndPreview() {
      const ui = getUiStrings(state.lang);
      const feat = getFeatureById(state.featureId);
      const mode = getFailureModeById(feat, state.failureModeId);
      const totalStepsChecked =
        state.checkedFeatureSteps.length + state.checkedFailureSteps.length + (state.customStep.trim() ? 1 : 0);

      if (dom.step1Pill) {
        dom.step1Pill.classList.toggle('done', Boolean(feat));
        dom.step1Pill.classList.toggle('active', !feat);
      }
      if (dom.step2Pill) {
        dom.step2Pill.classList.toggle('done', Boolean(feat && mode));
        dom.step2Pill.classList.toggle('active', Boolean(feat && !mode));
      }
      if (dom.step3Pill) {
        dom.step3Pill.classList.toggle('active', Boolean(feat && mode));
        dom.step3Pill.classList.toggle('done', Boolean(feat && mode && totalStepsChecked > 0));
      }

      if (dom.summaryFeature) {
        dom.summaryFeature.textContent = feat ? `${feat.icon} ${pickLoc(feat.title, state.lang)}` : '—';
      }
      if (dom.summaryFailure) {
        dom.summaryFailure.textContent = mode ? pickLoc(mode.label, state.lang) : '—';
      }
      if (dom.summarySteps) {
        dom.summarySteps.textContent = totalStepsChecked > 0 ? String(totalStepsChecked) : ui.summaryGuidelinesNone;
      }

      if (dom.previewTitle) {
        dom.previewTitle.textContent = feat && mode ? buildIssueTitle(state) : '[Bug] —';
      }
      if (dom.previewPre) {
        dom.previewPre.textContent =
          feat && mode
            ? buildMarkdownBody(state, { maxLogChars: MAX_LOG_CHARS_PREVIEW })
            : `${ui.chooseFeatureFirst}\n\n${ui.chooseFailureFirst}`;
      }

      const ready = Boolean(feat && mode);
      if (dom.openGithubBtn) dom.openGithubBtn.disabled = !ready;
      if (dom.copyReportBtn) dom.copyReportBtn.disabled = !ready;
    }

    function selectFeature(id, shouldScroll) {
      if (state.featureId !== id) {
        state.featureId = id;
        state.failureModeId = '';
        state.checkedFeatureSteps = [];
        state.checkedFailureSteps = [];
      }
      if (dom.featureSelect) dom.featureSelect.value = state.featureId;
      renderStep1Grid();
      renderStep2Modes();
      renderStep3Checklist();
      renderStepperAndPreview();
      if (shouldScroll && dom.step2Section && typeof dom.step2Section.scrollIntoView === 'function') {
        dom.step2Section.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
      }
    }

    function selectFailureMode(modeId, shouldScroll) {
      if (state.failureModeId !== modeId) {
        state.failureModeId = modeId;
        state.checkedFailureSteps = [];
      }
      renderStep2Modes();
      renderStep3Checklist();
      renderStepperAndPreview();
      if (shouldScroll && dom.step3Section && typeof dom.step3Section.scrollIntoView === 'function') {
        dom.step3Section.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
      }
    }

    // Vincular filtros y buscador Paso 1
    dom.filterBtns.forEach((btn) => {
      btn.addEventListener('click', () => {
        dom.filterBtns.forEach((b) => b.classList.remove('active'));
        btn.classList.add('active');
        state.categoryFilter = btn.getAttribute('data-br-filter') || 'all';
        renderStep1Grid();
      });
    });

    if (dom.searchInput) {
      dom.searchInput.addEventListener('input', () => {
        state.searchQuery = dom.searchInput.value;
        renderStep1Grid();
      });
    }

    if (dom.featureSelect) {
      dom.featureSelect.addEventListener('change', () => {
        selectFeature(dom.featureSelect.value, Boolean(dom.featureSelect.value));
      });
    }

    // Vincular entradas Paso 3
    if (dom.failureDetailInput) {
      dom.failureDetailInput.addEventListener('input', () => {
        state.customFailureDetail = dom.failureDetailInput.value;
        renderStepperAndPreview();
      });
    }
    if (dom.customStepInput) {
      dom.customStepInput.addEventListener('input', () => {
        state.customStep = dom.customStepInput.value;
        renderStepperAndPreview();
      });
    }
    if (dom.extraNotesInput) {
      dom.extraNotesInput.addEventListener('input', () => {
        state.extraNotes = dom.extraNotesInput.value;
        renderStepperAndPreview();
      });
    }
    if (dom.versionInput) {
      dom.versionInput.value = state.version;
      dom.versionInput.addEventListener('input', () => {
        state.version = dom.versionInput.value;
        renderStepperAndPreview();
      });
    }
    if (dom.platformSelect) {
      dom.platformSelect.addEventListener('change', () => {
        state.platformPreset = dom.platformSelect.value;
        renderStepperAndPreview();
      });
    }
    if (dom.gameSelect) {
      dom.gameSelect.addEventListener('change', () => {
        state.gameEdition = dom.gameSelect.value;
        renderStepperAndPreview();
      });
    }
    if (dom.frequencySelect) {
      dom.frequencySelect.addEventListener('change', () => {
        state.frequency = dom.frequencySelect.value;
        renderStepperAndPreview();
      });
    }
    if (dom.systemContextInput) {
      dom.systemContextInput.addEventListener('input', () => {
        state.systemContext = dom.systemContextInput.value;
        renderStepperAndPreview();
      });
    }

    if (dom.logTextarea) {
      dom.logTextarea.value = state.logText;
      dom.logTextarea.addEventListener('input', () => {
        state.logText = dom.logTextarea.value;
        renderLogMetaAndPreview();
      });
    }

    if (dom.logClearBtn) {
      dom.logClearBtn.addEventListener('click', () => {
        state.logText = '';
        state.logFile = '';
        if (dom.logTextarea) dom.logTextarea.value = '';
        renderLogMetaAndPreview();
      });
    }

    if (dom.logRestoreBtn) {
      dom.logRestoreBtn.addEventListener('click', () => {
        state.logText = state.initialLogText;
        state.logFile = launch.logFile;
        if (dom.logTextarea) dom.logTextarea.value = state.logText;
        renderLogMetaAndPreview();
      });
    }

    if (dom.logFileInput) {
      dom.logFileInput.addEventListener('change', () => {
        const file = dom.logFileInput.files && dom.logFileInput.files[0];
        if (!file) return;
        const startOffset = file.size > MAX_LOG_UPLOAD_BYTES ? file.size - MAX_LOG_UPLOAD_BYTES : 0;
        const slice = file.slice(startOffset);
        const reader = new FileReader();
        reader.onload = () => {
          const trimmed = trimLogLines(String(reader.result || ''), MAX_LOG_LINES, MAX_LOG_CHARS_PREVIEW);
          state.logText = trimmed.content;
          state.logFile = file.name;
          if (dom.logTextarea) dom.logTextarea.value = state.logText;
          renderLogMetaAndPreview();
        };
        reader.readAsText(slice);
      });
    }

    // Acciones finales: Copiar y Abrir GitHub Issue
    if (dom.copyReportBtn) {
      dom.copyReportBtn.addEventListener('click', async () => {
        const ui = getUiStrings(state.lang);
        if (!state.featureId || !state.failureModeId) {
          showToast(ui.selectErrorToast, 'warn');
          return;
        }
        const { title, fullBody } = buildGitHubIssueUrl(state);
        try {
          await copyToClipboard(`# ${title}\n\n${fullBody}`);
          showToast(ui.copiedToast, 'success');
        } catch {
          showToast(ui.selectErrorToast, 'warn');
        }
      });
    }

    if (dom.openGithubBtn) {
      dom.openGithubBtn.addEventListener('click', async () => {
        const ui = getUiStrings(state.lang);
        if (!state.featureId || !state.failureModeId) {
          showToast(ui.selectErrorToast, 'warn');
          return;
        }
        const { url, fullBody, wasTruncatedForUrl } = buildGitHubIssueUrl(state);
        if (wasTruncatedForUrl) {
          try {
            await copyToClipboard(fullBody);
            showToast(ui.truncatedToast, 'info');
          } catch {
            // Seguir abriendo GitHub aunque falle el portapapeles
          }
        }
        window.open(url, '_blank', 'noopener');
      });
    }

    // Menú móvil propio en reportar-bug.html
    const toggleBtn = document.querySelector('.menu-toggle');
    const navLinks = document.querySelector('.nav-links');
    if (toggleBtn && navLinks) {
      toggleBtn.addEventListener('click', () => {
        const isShown = navLinks.style.display === 'flex';
        navLinks.style.display = isShown ? 'none' : 'flex';
        if (!isShown) {
          navLinks.style.position = 'absolute';
          navLinks.style.top = '100%';
          navLinks.style.left = '0';
          navLinks.style.width = '100%';
          navLinks.style.background = 'var(--hud-surface-solid)';
          navLinks.style.flexDirection = 'column';
          navLinks.style.padding = '1.25rem';
          navLinks.style.borderBottom = '1px solid var(--hud-border)';
          navLinks.style.boxShadow = 'var(--shadow-lg)';
        }
      });
    }

    // Si no venía ?version= desde TMC, consultar versión publicada sin bloquear UI
    if (!state.version && typeof fetch === 'function') {
      fetch(`https://api.github.com/repos/${GITHUB_REPO}/releases/latest`, {
        headers: { Accept: 'application/vnd.github.v3+json' },
      })
        .then((res) => (res.ok ? res.json() : null))
        .then((rel) => {
          if (rel && rel.tag_name && !state.version && dom.versionInput && !dom.versionInput.value) {
            state.version = rel.tag_name;
            dom.versionInput.value = rel.tag_name;
            renderStepperAndPreview();
          }
        })
        .catch(() => {});
    }

    window.addEventListener('languagechange', (ev) => {
      const next = ev && ev.detail && ev.detail.lang ? ev.detail.lang : 'es';
      state.lang = resolveLang(next);
      syncStaticLocalizedTexts();
      renderFeatureSelectOptions();
      renderStep1Grid();
      renderStep2Modes();
      renderStep3Checklist();
      renderLogMetaAndPreview();
    });

    // Primer renderizado completo
    syncStaticLocalizedTexts();
    renderFeatureSelectOptions();
    renderStep1Grid();
    renderStep2Modes();
    renderStep3Checklist();
    renderLogMetaAndPreview();
  }

  if (typeof document !== 'undefined') {
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', initBugReporterApp);
    } else {
      initBugReporterApp();
    }
  }

  return {
    FEATURE_CATALOG,
    OTHER_FAILURE_MODE,
    REPORTER_I18N,
    resolveLang,
    getUiStrings,
    getFeatureById,
    getFailureModesForFeature,
    getFailureModeById,
    trimLogLines,
    parseLaunchContext,
    buildIssueTitle,
    buildMarkdownBody,
    buildGitHubIssueUrl,
    initBugReporterApp,
  };
});
