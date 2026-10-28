/**
 * TMC — The Multipurpose Companion
 * Scripts interactivos del portal de documentación y vista previa Glass HUD
 */

document.addEventListener('DOMContentLoaded', () => {
  initCockpitSimulator();
  initSearchModal();
  initModuleFilters();
  initFaqAccordion();
  initCodeCopyButtons();
  initScrollSpy();
  initMobileMenu();
});

// ===================================================================
// 1. VISTA PREVIA INTERACTIVA DE INTERFAZ («GLASS HUD»)
// ===================================================================

const SIM_DATA = {
  exploration: {
    badgeMode: 'SUPERCRUCERO',
    badgeClass: 'badge-cyan',
    hull: '100 %',
    hullFill: '100%',
    fuel: '88 %',
    fuelFill: '88%',
    shield: '100 %',
    shieldFill: '100%',
    credits: '14.280.000 Cr',
    routeTitle: 'Siguiente Salto (Spansh Exact Plotter)',
    routeTarget: 'Eol Prou SS-T d3-128',
    routeAction: 'Copiar',
    systemName: 'Synuefe GT-H b43-1',
    cardTitle: '🔭 Cuerpos Celestes & Cartografía FSS',
    rows: [
      { name: 'Synuefe GT-H b43-1 3 (Mundo Acuático)', badge: 'TERRAFORMABLE', badgeClass: 'badge-cyan', val: '3.250.000 Cr' },
      { name: 'Synuefe GT-H b43-1 4 a (Superficie)', badge: '4 BIOSEÑALES', badgeClass: 'badge-purple', val: '19.000.000 Cr' },
      { name: 'Synuefe GT-H b43-1 5 (Tipo Tierra)', badge: 'ELW · MAPEAR', badgeClass: 'badge-green', val: '4.800.000 Cr' }
    ],
    ttsAudio: 'Salto completado a Synuefe. Estrella repostable. Detectadas señales biológicas en un cuerpo.'
  },
  combat: {
    badgeMode: 'ZONA DE CONFLICTO AX',
    badgeClass: 'badge-danger',
    hull: '82 %',
    hullFill: '82%',
    fuel: '72 %',
    fuelFill: '72%',
    shield: '64 %',
    shieldFill: '64%',
    credits: '48.000.000 Cr',
    routeTitle: 'Objetivo Fijado (ShipTargeted)',
    routeTarget: 'Interceptor Clase Medusa',
    routeAction: 'Copiar Sistema',
    systemName: 'HIP 23716',
    cardTitle: '⚔️ Telemetría de Objetivo & Estado AX',
    rows: [
      { name: 'Corazón Thargoide 1 / 4', badge: 'DESTRUIDO', badgeClass: 'badge-green', val: '0 %' },
      { name: 'Corazón Thargoide 2 / 4', badge: 'EXPUESTO', badgeClass: 'badge-danger', val: '38 %' },
      { name: 'Enjambre Thargoide (32 unidades)', badge: 'ACTIVO', badgeClass: 'badge-orange', val: 'Flak' }
    ],
    ttsAudio: 'Advertencia de combate: escudos al sesenta y cuatro por ciento. Corazón número dos expuesto.'
  },
  mining: {
    badgeMode: 'ANILLO PLANETARIO',
    badgeClass: 'badge-orange',
    hull: '100 %',
    hullFill: '100%',
    fuel: '65 %',
    fuelFill: '65%',
    shield: '100 %',
    shieldFill: '100%',
    credits: '96,4 t/h',
    routeTitle: 'Estación de Venta en Radio (18 AL)',
    routeTarget: 'Jameson Memorial (Shinrarta Dezhra)',
    routeAction: 'Copiar Sistema',
    systemName: 'Delkar 7 (Anillo Metálico A)',
    cardTitle: '⛏️ Último Dron Prospector & Bodega',
    rows: [
      { name: 'Platino (Mineral de interés)', badge: '58,4 % · ALTO', badgeClass: 'badge-cyan', val: '282.000 Cr/t' },
      { name: 'Painita', badge: '34,2 % · MEDIO', badgeClass: 'badge-green', val: '240.000 Cr/t' },
      { name: 'Ocupación de Bodega', badge: '112 / 128 t', badgeClass: 'badge-orange', val: '87,5 %' }
    ],
    ttsAudio: 'Asteroide prospectado con cincuenta y ocho por ciento de Platino.'
  },
  trade: {
    badgeMode: 'ATRACADO',
    badgeClass: 'badge-green',
    hull: '100 %',
    hullFill: '100%',
    fuel: '100 %',
    fuelFill: '100%',
    shield: '100 %',
    shieldFill: '100%',
    credits: '86.200.000 Cr',
    routeTitle: 'Destino Comercial Seleccionado',
    routeTarget: 'Wally Bei (Malerba Orbital)',
    routeAction: 'Copiar Sistema',
    systemName: 'Sol (Abraham Lincoln)',
    cardTitle: '📦 Comparativa Market.json & Carga',
    rows: [
      { name: 'Carga en bodega: Plata (720 t)', badge: 'COMPRA: 34.800 Cr/t', badgeClass: 'badge-cyan', val: '720 / 720 t' },
      { name: 'Precio receptor (datos EDDN < 2 h)', badge: 'VENTA: 48.950 Cr/t', badgeClass: 'badge-green', val: '+14.150 Cr/t' },
      { name: 'Plataforma y Demanda en Destino', badge: 'GRANDE · 18.500 t', badgeClass: 'badge-orange', val: '14,2 AL' }
    ],
    ttsAudio: 'Datos de mercado local actualizados. Ruta comercial de un salto verificada para plataforma grande.'
  }
};

let currentSimMode = 'exploration';

function initCockpitSimulator() {
  const modeButtons = document.querySelectorAll('.cockpit-mode-btn');
  const copyBtn = document.getElementById('simCopyBtn');
  const audioBtn = document.getElementById('simAudioBtn');

  modeButtons.forEach((btn) => {
    btn.addEventListener('click', () => {
      const mode = btn.dataset.mode;
      if (!mode || mode === currentSimMode) return;

      modeButtons.forEach((b) => b.classList.remove('active'));
      btn.classList.add('active');
      currentSimMode = mode;
      renderSimMode(mode);
    });
  });

  if (copyBtn) {
    copyBtn.addEventListener('click', () => {
      const targetSystem = document.getElementById('simRouteTarget').textContent.trim();
      copyToClipboard(targetSystem, `Elemento «${targetSystem}» copiado al portapapeles.`);
    });
  }

  if (audioBtn) {
    audioBtn.addEventListener('click', () => {
      playCockpitVoice(SIM_DATA[currentSimMode].ttsAudio);
    });
  }

  renderSimMode('exploration');
}

function renderSimMode(mode) {
  const data = SIM_DATA[mode];
  if (!data) return;

  const badgeMode = document.getElementById('simModeBadge');
  const simHull = document.getElementById('simHull');
  const simFuel = document.getElementById('simFuel');
  const simShield = document.getElementById('simShield');
  const simCredits = document.getElementById('simCredits');
  const simRouteTitle = document.getElementById('simRouteTitle');
  const simRouteTarget = document.getElementById('simRouteTarget');
  const simCardTitle = document.getElementById('simCardTitle');
  const simRowsContainer = document.getElementById('simRowsContainer');

  if (badgeMode) {
    badgeMode.textContent = data.badgeMode;
    badgeMode.className = `badge ${data.badgeClass}`;
  }

  if (simHull) simHull.textContent = data.hull;
  if (simFuel) simFuel.textContent = data.fuel;
  if (simShield) simShield.textContent = data.shield;
  if (simCredits) simCredits.textContent = data.credits;

  if (simRouteTitle) simRouteTitle.textContent = data.routeTitle;
  if (simRouteTarget) simRouteTarget.textContent = data.routeTarget;
  if (simCardTitle) simCardTitle.textContent = data.cardTitle;

  if (simRowsContainer) {
    simRowsContainer.innerHTML = data.rows
      .map(
        (row) => `
      <div class="sim-item-row">
        <span>${row.name}</span>
        <div style="display: flex; align-items: center; gap: 0.65rem;">
          <span class="badge ${row.badgeClass}">${row.badge}</span>
          <span class="text-gold" style="font-weight: 700;">${row.val}</span>
        </div>
      </div>
    `
      )
      .join('');
  }
}

function playCockpitVoice(text) {
  if ('speechSynthesis' in window) {
    window.speechSynthesis.cancel();
    const utterance = new SpeechSynthesisUtterance(text);
    utterance.lang = 'es-ES';
    utterance.rate = 1.03;
    utterance.pitch = 0.98;

    const voices = window.speechSynthesis.getVoices();
    const esVoice = voices.find((v) => v.lang.startsWith('es'));
    if (esVoice) utterance.voice = esVoice;

    window.speechSynthesis.speak(utterance);
    showToast(`🔊 Locución TTS: «${text}»`);
  } else {
    showToast(`🔊 Locución TTS: ${text}`);
  }
}

// ===================================================================
// 2. BUSCADOR INSTANTÁNEO GLOBAL (CTRL + K)
// ===================================================================

const SEARCH_DATABASE = [
  // Arquitectura y código abierto
  { title: 'Arquitectura y Principios de Diseño', category: 'Ingeniería', url: '#arquitectura', snippet: 'Motor en Rust, ergonomía Glass HUD (regla del 33 %), SQLite WAL local, aislamiento Legacy y código abierto.' },

  // Módulos (16)
  { title: 'Panel General (Dashboard)', category: 'Módulo', url: '#mod-dashboard', snippet: 'Estado del comandante, créditos, rangos, banderas en vivo de Status.json, métricas de sesión y alertas.' },
  { title: 'Nave & Equipamiento (Loadout)', category: 'Módulo', url: '#mod-ship', snippet: 'Auditoría por rol (Exploración, Combate, Carga) y desglose de ranuras con integridad e ingeniería aplicada.' },
  { title: 'Navegación & Planificador Spansh', category: 'Módulo', url: '#mod-navigation', snippet: 'Ruta exacta (Exact Plotter con física leída del diario), Autopista de neutrones, coordenadas 3D y RouteStrip.' },
  { title: 'Exploración Cartográfica', category: 'Módulo', url: '#mod-exploration', snippet: 'Progreso FSS, tabla adaptativa de cuerpos celestes y umbral configurable de mapeo DSS.' },
  { title: 'Exobiología de Superficie', category: 'Módulo', url: '#mod-exobiology', snippet: 'Predicción de especies candidatas en Odyssey y cálculo GPS en tiempo real de distancias mínimas clonales (3 muestras).' },
  { title: 'Combate & Actividad Anti-Xeno', category: 'Módulo', url: '#mod-combat', snippet: 'Lectura de objetivo fijado (ShipTargeted), subsistemas, Zonas de Conflicto, bonos y seguimiento AX.' },
  { title: 'Minería & Prospección', category: 'Módulo', url: '#mod-mining', snippet: 'Análisis de drones prospectores, cadencia en toneladas/hora, minerales de interés y radio de venta.' },
  { title: 'Comercio & Mercados', category: 'Módulo', url: '#mod-trade', snippet: 'Inventario de carga, comparativa de Market.json frente a datos EDDN y filtros por plataforma.' },
  { title: 'Misiones & Contratos', category: 'Módulo', url: '#mod-missions', snippet: 'Seguimiento de expiración, referencia de turismo y agrupación de misiones de masacre por facción emisora.' },
  { title: 'Ingeniería & Materiales (EngineerFlow)', category: 'Módulo', url: '#mod-engineering', snippet: 'Guía secuencial de desbloqueo de ingenieros, almacén, taquilla Odyssey, lista de la compra y Material Traders.' },
  { title: 'Fleet Carrier', category: 'Módulo', url: '#mod-carrier', snippet: 'Reserva y consumo de Tritio, cuenta atrás de saltos programados, ocupación y presupuesto semanal.' },
  { title: 'Colonización de Sistemas', category: 'Módulo', url: '#mod-colonisation', snippet: 'Control de entregas de materias primas de construcción y localización de oferta en mercados cercanos.' },
  { title: 'Powerplay 2.0', category: 'Módulo', url: '#mod-powerplay', snippet: 'Potencia vinculada, rango actual, méritos acumulados en el ciclo semanal y estado estratégico del sistema.' },
  { title: 'Simulación de Fondo (BGS)', category: 'Módulo', url: '#mod-bgs', snippet: 'Tabla de facciones menores, porcentajes de influencia, estados activos/pendientes y acciones de sesión.' },
  { title: 'Crimen & Jurisdicción', category: 'Módulo', url: '#mod-crime', snippet: 'Multas, recompensas, decaimiento de notoriedad (2 h/punto), carga ilegal e Interstellar Factors cercanos.' },
  { title: 'Cuentas & Sincronización', category: 'Módulo', url: '#mod-accounts', snippet: 'Frontier CAPI (OAuth2 PKCE y flota remota), Inara InAPI v1 (whitelist oficial), EDSM y EDDN.' },

  // Guía de instrucciones
  { title: 'Instalación, SmartScreen y Archivos Locales', category: 'Instrucciones', url: '#manual-install', snippet: 'Instalador NSIS, nota sobre Windows SmartScreen, config.toml, tmc.db, bandeja del sistema y flag --journal-dir.' },
  { title: 'Anatomía de Interfaz, Disposiciones y Auto-companion', category: 'Instrucciones', url: '#manual-hud', snippet: 'TopBar, diferencia entre Disposiciones combinadas y Módulos individuales, y paneles desplegables.' },
  { title: 'Pantalla Auxiliar en Red Local (Código QR)', category: 'Instrucciones', url: '#manual-lan', snippet: 'Cómo abrir la interfaz en una tablet o teléfono en la misma red Wi-Fi escaneando el código QR.' },
  { title: 'Síntesis de Voz (TTS), Reconocimiento y VoiceAttack', category: 'Instrucciones', url: '#manual-voice', snippet: 'Voces SAPI5/OneCore y KokoroTTS, reconocimiento Whisper AI Local y exportación del perfil TMC-Copiloto.vap.' },
  { title: 'Vinculación Opcional: CAPI, Inara, EDSM y EDDN', category: 'Instrucciones', url: '#manual-accounts', snippet: 'Configuración paso a paso de servicios externos y modo de funcionamiento 100 % local.' },
  { title: 'Compilación desde el Código Fuente', category: 'Código Abierto', url: '#manual-build', snippet: 'Comandos git clone, npm build, cargo test y cargo build --release para compilar TMC localmente.' },

  // Flujos de trabajo
  { title: 'Flujo: Muestreo Genético en Exobiología', category: 'Procedimiento', url: '#tut-exobiology', snippet: 'Secuencia de escaneo FSS/DSS y toma de las 3 muestras respetando el radio mínimo clonal.' },
  { title: 'Flujo: Desbloqueo de Ingenieros e Intercambio', category: 'Procedimiento', url: '#tut-engineering', snippet: 'Cálculo de blueprint, verificación de pasos de desbloqueo, lista de la compra y trueque de excedentes.' },
  { title: 'Flujo: Seguimiento de Misiones por Facción', category: 'Procedimiento', url: '#tut-massacre', snippet: 'Control de columnas por facción emisora para computar bajas en paralelo y plazos de entrega.' },
  { title: 'Flujo: Gestión de Notoriedad e Interstellar Factors', category: 'Procedimiento', url: '#tut-crime', snippet: 'Comprobación de notoriedad 0 y localización de contactos neutrales para liquidar sanciones.' }
];

function initSearchModal() {
  const modal = document.getElementById('searchModal');
  const triggerBtn = document.getElementById('searchTriggerBtn');
  const input = document.getElementById('searchModalInput');
  const resultsContainer = document.getElementById('searchResultsList');

  if (!modal || !input) return;

  function openSearch() {
    modal.classList.add('open');
    input.value = '';
    renderSearchResults('');
    setTimeout(() => input.focus(), 50);
  }

  function closeSearch() {
    modal.classList.remove('open');
  }

  if (triggerBtn) {
    triggerBtn.addEventListener('click', openSearch);
  }

  modal.addEventListener('click', (e) => {
    if (e.target === modal) closeSearch();
  });

  window.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      modal.classList.contains('open') ? closeSearch() : openSearch();
    } else if (e.key === 'Escape' && modal.classList.contains('open')) {
      closeSearch();
    }
  });

  input.addEventListener('input', (e) => {
    renderSearchResults(e.target.value.trim().toLowerCase());
  });

  function renderSearchResults(query) {
    if (!resultsContainer) return;

    const filtered = query === ''
      ? SEARCH_DATABASE.slice(0, 8)
      : SEARCH_DATABASE.filter(item =>
          item.title.toLowerCase().includes(query) ||
          item.category.toLowerCase().includes(query) ||
          item.snippet.toLowerCase().includes(query)
        );

    if (filtered.length === 0) {
      resultsContainer.innerHTML = `
        <div style="padding: 2rem; text-align: center; color: var(--hud-muted); font-family: var(--font-mono); font-size: 0.9rem;">
          No se han encontrado resultados para «${query}».
        </div>
      `;
      return;
    }

    resultsContainer.innerHTML = filtered.map(item => `
      <a href="${item.url}" class="search-result-item" onclick="document.getElementById('searchModal').classList.remove('open')">
        <div class="search-result-title">
          <span>${item.title}</span>
          <span class="search-result-category">${item.category}</span>
        </div>
        <div class="search-result-snippet">${item.snippet}</div>
      </a>
    `).join('');
  }
}

// ===================================================================
// 3. FILTRADO Y BÚSQUEDA DE MÓDULOS
// ===================================================================

function initModuleFilters() {
  const filterButtons = document.querySelectorAll('.filter-btn');
  const moduleCards = document.querySelectorAll('.module-card');
  const searchInput = document.getElementById('moduleSearchInput');

  let activeCategory = 'all';
  let activeSearchTerm = '';

  filterButtons.forEach((btn) => {
    btn.addEventListener('click', () => {
      filterButtons.forEach((b) => b.classList.remove('active'));
      btn.classList.add('active');
      activeCategory = btn.dataset.category || 'all';
      applyFilters();
    });
  });

  if (searchInput) {
    searchInput.addEventListener('input', (e) => {
      activeSearchTerm = e.target.value.trim().toLowerCase();
      applyFilters();
    });
  }

  function applyFilters() {
    moduleCards.forEach((card) => {
      const category = card.dataset.category || '';
      const text = card.textContent.toLowerCase();

      const matchesCategory = activeCategory === 'all' || category === activeCategory;
      const matchesSearch = activeSearchTerm === '' || text.includes(activeSearchTerm);

      if (matchesCategory && matchesSearch) {
        card.style.display = 'flex';
      } else {
        card.style.display = 'none';
      }
    });
  }
}

// ===================================================================
// 4. PREGUNTAS FRECUENTES (FAQ ACORDEÓN)
// ===================================================================

function initFaqAccordion() {
  const faqItems = document.querySelectorAll('.faq-item');

  faqItems.forEach((item) => {
    const question = item.querySelector('.faq-question');
    if (!question) return;

    question.addEventListener('click', () => {
      const isActive = item.classList.contains('active');
      faqItems.forEach((other) => other.classList.remove('active'));
      if (!isActive) {
        item.classList.add('active');
      }
    });
  });
}

// ===================================================================
// 5. BOTONES DE COPIAR CÓDIGO Y RUTAS CON FEEDBACK
// ===================================================================

function initCodeCopyButtons() {
  const copyButtons = document.querySelectorAll('.code-copy-btn');

  copyButtons.forEach((btn) => {
    btn.addEventListener('click', () => {
      const snippet = btn.closest('.code-snippet');
      if (!snippet) return;

      const codeText = snippet.innerText.replace(btn.innerText, '').trim();
      copyToClipboard(codeText, 'Comandos copiados al portapapeles');

      const originalText = btn.textContent;
      btn.textContent = 'Copiado';
      btn.style.background = 'var(--hud-success)';
      btn.style.color = '#000';

      setTimeout(() => {
        btn.textContent = originalText;
        btn.style.background = '';
        btn.style.color = '';
      }, 2000);
    });
  });
}

function copyToClipboard(text, successMessage) {
  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(text).then(() => {
      showToast(successMessage);
    }).catch(() => {
      fallbackCopy(text, successMessage);
    });
  } else {
    fallbackCopy(text, successMessage);
  }
}

function fallbackCopy(text, message) {
  const textarea = document.createElement('textarea');
  textarea.value = text;
  textarea.style.position = 'fixed';
  textarea.style.opacity = '0';
  document.body.appendChild(textarea);
  textarea.select();
  try {
    document.execCommand('copy');
    showToast(message);
  } catch (e) {
    showToast('No se pudo copiar automáticamente');
  }
  document.body.removeChild(textarea);
}

function showToast(message) {
  let container = document.querySelector('.toast-container');
  if (!container) {
    container = document.createElement('div');
    container.className = 'toast-container';
    document.body.appendChild(container);
  }

  const toast = document.createElement('div');
  toast.className = 'toast';
  toast.innerHTML = `
    <span style="color: var(--hud-primary);">●</span>
    <span>${message}</span>
  `;

  container.appendChild(toast);

  setTimeout(() => {
    toast.style.opacity = '0';
    toast.style.transform = 'translateX(100%)';
    toast.style.transition = 'all 0.3s ease';
    setTimeout(() => toast.remove(), 300);
  }, 3200);
}

// ===================================================================
// 6. SCROLL SPY Y NAVEGACIÓN ACTIVA
// ===================================================================

function initScrollSpy() {
  const sections = document.querySelectorAll('section[id]');
  const chapters = document.querySelectorAll('.manual-chapter[id]');
  const navLinks = document.querySelectorAll('.nav-link');
  const manualNavItems = document.querySelectorAll('.manual-nav-item');

  window.addEventListener('scroll', () => {
    const scrollPos = window.scrollY + 140;

    sections.forEach((section) => {
      const top = section.offsetTop;
      const height = section.offsetHeight;
      const id = section.getAttribute('id');

      if (scrollPos >= top && scrollPos < top + height) {
        navLinks.forEach((link) => {
          link.classList.remove('active');
          if (link.getAttribute('href') === `#${id}`) {
            link.classList.add('active');
          }
        });
      }
    });

    chapters.forEach((chapter) => {
      const top = chapter.offsetTop;
      const height = chapter.offsetHeight;
      const id = chapter.getAttribute('id');

      if (scrollPos >= top && scrollPos < top + height) {
        manualNavItems.forEach((link) => {
          link.classList.remove('active');
          if (link.getAttribute('href') === `#${id}`) {
            link.classList.add('active');
          }
        });
      }
    });
  });
}

// ===================================================================
// 7. MENÚ RESPONSIVE MÓVIL
// ===================================================================

function initMobileMenu() {
  const toggleBtn = document.querySelector('.menu-toggle');
  const navLinks = document.querySelector('.nav-links');

  if (!toggleBtn || !navLinks) return;

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
      navLinks.style.padding = '1.5rem';
      navLinks.style.borderBottom = '1px solid var(--hud-border)';
      navLinks.style.boxShadow = 'var(--shadow-lg)';
    }
  });

  navLinks.querySelectorAll('a').forEach((link) => {
    link.addEventListener('click', () => {
      if (window.innerWidth <= 768) {
        navLinks.style.display = 'none';
      }
    });
  });
}
