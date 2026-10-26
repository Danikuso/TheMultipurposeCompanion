/**
 * TMC — The Multipurpose Companion
 * Motor Interactivo de la Web Oficial
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
// 1. SIMULADOR INTERACTIVO DE CABINA («LIVE COCKPIT HUD»)
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
    routeTitle: 'Siguiente Salto (Neutrones Spansh)',
    routeTarget: 'Eol Prou SS-T d3-128',
    routeAction: 'Copiar',
    systemName: 'Synuefe GT-H b43-1',
    cardTitle: '🔭 Cuerpos Celestes & Cartografía FSS',
    rows: [
      { name: 'Synuefe GT-H b43-1 3 (Mundo Acuático)', badge: '🌊 TERRAFORMABLE', badgeClass: 'badge-cyan', val: '3.250.000 Cr' },
      { name: 'Synuefe GT-H b43-1 4 a (Biológico 4)', badge: '🌿 EXOBIOLOGÍA', badgeClass: 'badge-purple', val: '19.500.000 Cr' },
      { name: 'Synuefe GT-H b43-1 5 (Tierra ELW)', badge: '🌍 HABITABLE', badgeClass: 'badge-green', val: '4.800.000 Cr' }
    ],
    ttsAudio: 'Comandante, entrada en sistema Synuefe. Tres cuerpos de alto valor detectados. Siguiente salto alineado a Eol Prou.'
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
    routeTitle: 'Alerta Táctica Thargoide',
    routeTarget: 'Medusa Clase Interceptor',
    routeAction: 'Fijar Objetivo',
    systemName: 'HIP 23716 (Ataque Activo)',
    cardTitle: '⚔️ Monitor de Interceptores & Corazones',
    rows: [
      { name: 'Corazón Cáustico 1/4', badge: 'DESTRUIDO', badgeClass: 'badge-green', val: '0 %' },
      { name: 'Corazón Cáustico 2/4', badge: 'EXPUESTO', badgeClass: 'badge-danger', val: '38 %' },
      { name: 'Enjambre Thargoide (32)', badge: 'AMENAZA ALTA', badgeClass: 'badge-orange', val: 'Flak Req.' }
    ],
    ttsAudio: '¡Atención comandante! Corazón cáustico número dos expuesto. Escudos del interceptor al cuarenta por ciento.'
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
    credits: '28.400.000 Cr/h',
    routeTitle: 'Mejor Mercado de Venta Cercano',
    routeTarget: 'Jameson Memorial (18 AL)',
    routeAction: 'Trazar Ruta',
    systemName: 'Delkar 7 (Anillo Metálico A)',
    cardTitle: '⛏️ Prospección de Asteroides & Bodega',
    rows: [
      { name: 'Platino Puro', badge: '💎 58.4 % NÚCLEO', badgeClass: 'badge-cyan', val: '282.000 Cr/t' },
      { name: 'Painita de Alta Pureza', badge: '💎 34.2 %', badgeClass: 'badge-green', val: '240.000 Cr/t' },
      { name: 'Bodega de Carga (Krait Mk II)', badge: '112/128 TONELADAS', badgeClass: 'badge-orange', val: '87.5 % Lleno' }
    ],
    ttsAudio: 'Prospección completada: asteroide de Platino con cincuenta y ocho por ciento de pureza. Rendimiento óptimo.'
  },
  trade: {
    badgeMode: 'ATRACADO EN ESTACIÓN',
    badgeClass: 'badge-green',
    hull: '100 %',
    hullFill: '100%',
    fuel: '100 %',
    fuelFill: '100%',
    shield: '100 %',
    shieldFill: '100%',
    credits: '86.200.000 Cr',
    routeTitle: 'Ruta Comercial de Alto Margen',
    routeTarget: 'Wally Sei (Isótopos H3)',
    routeAction: 'Copiar Destino',
    systemName: 'Sol (Estación Gorbachov)',
    cardTitle: '💰 Mercado Galáctico & Bodega Cutter',
    rows: [
      { name: 'Compra: Isótopos H3 (720 t)', badge: 'PRECIO: 4.800 Cr', badgeClass: 'badge-cyan', val: 'Inversión 3.4M' },
      { name: 'Venta esperada en Wally Sei', badge: 'MARGEN +122.000 Cr/t', badgeClass: 'badge-green', val: '+87.840.000 Cr' },
      { name: 'Demanda de la Estación Destino', badge: 'ALTA (18.500 t)', badgeClass: 'badge-orange', val: 'Seguro 1 Salto' }
    ],
    ttsAudio: 'Margen comercial confirmado: beneficio estimado de ochenta y siete millones en el siguiente salto.'
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
      copyToClipboard(targetSystem, `Sistema «${targetSystem}» copiado al portapapeles.`);
    });
  }

  if (audioBtn) {
    audioBtn.addEventListener('click', () => {
      playCockpitVoice(SIM_DATA[currentSimMode].ttsAudio);
    });
  }

  // Inicializar vista
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
    utterance.rate = 1.05;
    utterance.pitch = 0.95;
    
    // Intenta usar una voz natural si está disponible
    const voices = window.speechSynthesis.getVoices();
    const esVoice = voices.find((v) => v.lang.startsWith('es'));
    if (esVoice) utterance.voice = esVoice;

    window.speechSynthesis.speak(utterance);
    showToast(`🎙️ Copiloto de Cabina: «${text}»`);
  } else {
    showToast(`🎙️ Simulación de Voz: ${text}`);
  }
}

// ===================================================================
// 2. BUSCADOR INSTANTÁNEO GLOBAL (CTRL + K O ATRIBUTO SEARCH)
// ===================================================================

const SEARCH_DATABASE = [
  // Módulos
  { title: 'Dashboard & Telemetría', category: 'Módulo de Cabina', url: '#mod-dashboard', snippet: 'Estado del comandante, créditos, rango, alertas de sobrecalentamiento, SCO y salud de casco.' },
  { title: 'Nave & Módulos (Ship Outfitting)', category: 'Módulo de Nave', url: '#mod-ship', snippet: 'Inspección de equipamiento, integridad de módulos, badges de ingeniería aplicados y balance de energía.' },
  { title: 'Navegación & Route Strip', category: 'Módulo de Viaje', url: '#mod-navigation', snippet: 'Planificador Spansh de galaxia y neutrones, banner pegajoso de ruta y autocopiado de saltos.' },
  { title: 'Exploración Cartográfica', category: 'Módulo Científico', url: '#mod-exploration', snippet: 'Cuerpos celestes FSS/DSS, detección de mundos acuáticos, terraformables, ELW y scroll pegajoso.' },
  { title: 'Exobiología (Odyssey)', category: 'Módulo Científico', url: '#mod-exobiology', snippet: 'Semáforo de distancias mínimas entre muestras biológicas, catálogo por planeta y bono x5.' },
  { title: 'Combate & Guerra Thargoide (AX)', category: 'Módulo Táctico', url: '#mod-combat', snippet: 'Caza de recompensas, Zonas de Conflicto (CZ) y monitor de interceptores AX con corazones.' },
  { title: 'Minería de Alto Rendimiento', category: 'Módulo de Economía', url: '#mod-mining', snippet: 'Rendimiento en t/h, lectura en tiempo real de prospectores y recomendador de mercados de venta.' },
  { title: 'Comercio & Mercados', category: 'Módulo de Economía', url: '#mod-trade', snippet: 'Inventario de bodega de carga, visor de mercado local vs media galáctica y rutas multi-hop.' },
  { title: 'Misiones & Massacre Stacking', category: 'Módulo de Misiones', url: '#mod-missions', snippet: 'Apilamiento inteligente de misiones de masacre por facción, pasajeros VIP y Objetivos Comunitarios.' },
  { title: 'Ingeniería & Asistente Proactivo', category: 'Módulo de Artesanía', url: '#mod-engineering', snippet: 'Flujo guiado de desbloqueo (EngineerFlow), inventario de materiales G1-G5 y Material Traders.' },
  { title: 'Fleet Carrier (Mega-naves)', category: 'Módulo de Flota', url: '#mod-carrier', snippet: 'Gestión de carrier, consumo y stock de Tritio, cuenta atrás de saltos y finanzas de mantenimiento.' },
  { title: 'Colonización de Sistemas', category: 'Módulo de Expansión', url: '#mod-colonisation', snippet: 'Seguimiento de proyectos de colonización, suministro de materias primas y puertos espaciales.' },
  { title: 'Powerplay 2.0', category: 'Módulo Galáctico', url: '#mod-powerplay', snippet: 'Lealtad a líderes galácticos, méritos semanales, fortificación de sistemas y módulos exclusivos.' },
  { title: 'Simulación de Fondo (BGS)', category: 'Módulo Galáctico', url: '#mod-bgs', snippet: 'Monitor de influencia de facciones menores, estados de guerra, elecciones y registro de impacto.' },
  { title: 'Crimen, Notoriedad & Piratería', category: 'Módulo de Seguridad', url: '#mod-crime', snippet: 'Temporizador de notoriedad en tiempo real, Interstellar Factors, contrabando y mercados negros.' },
  { title: 'Cuentas & Sincronización', category: 'Módulo de Conectividad', url: '#mod-accounts', snippet: 'Sincronización con EDDN, EDSM e Inara respetando la privacidad y galaxia Legacy.' },

  // Manual de Vuelo
  { title: 'Cómo Empezar a Volar en 10 Segundos', category: 'Guía de Inicio', url: '#manual-install', snippet: 'Abre TMC.exe y detecta tu nave, comandante y sistema al instante sin configuraciones complejas.' },
  { title: 'Pantalla Táctil para Tablet o Móvil (QR)', category: 'Guía de Cabina', url: '#manual-lan', snippet: 'Usa tu tablet o smartphone como monitor auxiliar MFD de cabina escaneando un simple código QR.' },
  { title: 'Modo Auto-Companion: TMC Piensa por Ti', category: 'Guía de Cabina', url: '#manual-hud', snippet: 'Cambio de pantalla automático según lo que estés haciendo: saltar, minar, combatir o recolectar plantas.' },
  { title: 'Copiloto de Voz Hablada & VoiceAttack', category: 'Guía de Audio', url: '#manual-voice', snippet: 'Tu nave te avisa con voz en español de estrellas para repostar, bioseñales y daños críticos.' },
  { title: 'Conexión con Inara, EDSM y EDDN', category: 'Guía de Comunidad', url: '#manual-accounts', snippet: 'Mantén tu flota, créditos y perfil actualizados en la comunidad de Elite Dangerous en tiempo real.' },

  // Tutoriales
  { title: 'Tutorial: Ruta de Exobiología Multimillonaria', category: 'Tutorial Práctico', url: '#tut-exobiology', snippet: 'Cómo ganar más de 100M Cr por sistema aprovechando el semáforo de distancias y el bono de primer descubrimiento.' },
  { title: 'Tutorial: Desbloqueo de Ingenieros desde Cero', category: 'Tutorial Práctico', url: '#tut-engineering', snippet: 'Uso de EngineerFlow para completar los pasos de conocimiento, invitación y donación sin bloqueos.' },
  { title: 'Tutorial: Caza de Piratas con Massacre Stacking', category: 'Tutorial Práctico', url: '#tut-massacre', snippet: 'Cómo cobrar múltiples recompensas por una sola baja apilando misiones de distintas facciones.' },
  { title: 'Tutorial: Limpiar Notoriedad y Vender Contrabando', category: 'Tutorial Práctico', url: '#tut-crime', snippet: 'Gestión del decaimiento en vuelo, localización de Interstellar Factors y evasión policial.' }
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
      ? SEARCH_DATABASE.slice(0, 7)
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

      const codeText = snippet.textContent.replace(btn.textContent, '').trim();
      copyToClipboard(codeText, 'Comando copiado al portapapeles');
      
      const originalText = btn.textContent;
      btn.textContent = '¡Copiado!';
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
    showToast('Error al copiar automáticamente');
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
    <span style="color: var(--hud-primary);">⚡</span>
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
  const navLinks = document.querySelectorAll('.nav-link');
  const manualNavItems = document.querySelectorAll('.manual-nav-item');

  window.addEventListener('scroll', () => {
    const scrollPos = window.scrollY + 120;

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
