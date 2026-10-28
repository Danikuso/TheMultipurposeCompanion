/**
 * TMC — The Multipurpose Companion
 * Interactividad de la página de presentación
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
// 1. VISTA PREVIA INTERACTIVA DE CABINA
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
    routeTitle: 'Siguiente Salto Planificado',
    routeTarget: 'Eol Prou SS-T d3-128',
    routeAction: 'Copiar',
    systemName: 'Synuefe GT-H b43-1',
    cardTitle: '🔭 Cuerpos Descubiertos en el Sistema',
    rows: [
      { name: 'Synuefe GT-H b43-1 3 (Mundo Acuático)', badge: 'TERRAFORMABLE', badgeClass: 'badge-cyan', val: '3.250.000 Cr' },
      { name: 'Synuefe GT-H b43-1 4 a (Superficie)', badge: '4 BIOSEÑALES', badgeClass: 'badge-purple', val: '19.000.000 Cr' },
      { name: 'Synuefe GT-H b43-1 5 (Tipo Tierra)', badge: 'MAPEAR CON SONDAS', badgeClass: 'badge-green', val: '4.800.000 Cr' }
    ],
    ttsAudio: 'Salto completado a Synuefe. Estrella apta para repostar. Detectadas señales biológicas en un cuerpo.'
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
    routeTitle: 'Objetivo en el Punto de Mira',
    routeTarget: 'Interceptor Clase Medusa',
    routeAction: 'Copiar Destino',
    systemName: 'HIP 23716',
    cardTitle: '⚔️ Estado del Combate Anti-Xeno',
    rows: [
      { name: 'Corazón Thargoide 1 de 4', badge: 'DESTRUIDO', badgeClass: 'badge-green', val: '0 %' },
      { name: 'Corazón Thargoide 2 de 4', badge: 'EXPUESTO', badgeClass: 'badge-danger', val: '38 %' },
      { name: 'Enjambre Thargoide (32 unidades)', badge: 'ACTIVO', badgeClass: 'badge-orange', val: 'Atención' }
    ],
    ttsAudio: 'Atención en cabina: escudos al sesenta y cuatro por ciento. Segundo corazón expuesto.'
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
    routeTitle: 'Estación Cercana Recomendada (18 AL)',
    routeTarget: 'Jameson Memorial (Shinrarta Dezhra)',
    routeAction: 'Copiar Destino',
    systemName: 'Delkar 7 (Anillo Metálico A)',
    cardTitle: '⛏️ Último Asteroide Analizado y Bodega',
    rows: [
      { name: 'Platino (Marcado como favorito)', badge: 'PUREZA 58,4 %', badgeClass: 'badge-cyan', val: '282.000 Cr/t' },
      { name: 'Painita', badge: 'PUREZA 34,2 %', badgeClass: 'badge-green', val: '240.000 Cr/t' },
      { name: 'Espacio en Bodega de Carga', badge: '112 / 128 t', badgeClass: 'badge-orange', val: '87,5 %' }
    ],
    ttsAudio: 'Asteroide analizado con cincuenta y ocho por ciento de Platino.'
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
    routeTitle: 'Siguiente Estación Comercial',
    routeTarget: 'Wally Bei (Malerba Orbital)',
    routeAction: 'Copiar Destino',
    systemName: 'Sol (Abraham Lincoln)',
    cardTitle: '📦 Precios del Mercado y Bodega',
    rows: [
      { name: 'Carga a bordo: Plata (720 t)', badge: 'COMPRA: 34.800 Cr/t', badgeClass: 'badge-cyan', val: '720 / 720 t' },
      { name: 'Precio en estación de destino', badge: 'VENTA: 48.950 Cr/t', badgeClass: 'badge-green', val: '+14.150 Cr/t' },
      { name: 'Tamaño de aterrizaje en destino', badge: 'PLATAFORMA GRANDE', badgeClass: 'badge-orange', val: '14,2 AL' }
    ],
    ttsAudio: 'Precios del mercado local actualizados. Ruta de un salto lista para plataforma grande.'
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
      copyToClipboard(targetSystem, `Destino «${targetSystem}» copiado al portapapeles.`);
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
    showToast(`🔊 Voz de cabina: «${text}»`);
  } else {
    showToast(`🔊 Voz de cabina: ${text}`);
  }
}

// ===================================================================
// 2. BUSCADOR INSTANTÁNEO GLOBAL (CTRL + K)
// ===================================================================

const SEARCH_DATABASE = [
  // Cualidades
  { title: 'Cualidades Principales de TMC', category: 'Filosofía', url: '#cualidades', snippet: 'Guía paso a paso sin bloqueos, ligereza en segundo plano, lectura clara, voz, uso en tablet y código abierto.' },

  // Módulos (16)
  { title: 'Panel General', category: 'Módulo', url: '#mod-dashboard', snippet: 'Resumen de tu comandante, créditos, rangos, estado general de la nave, sesión actual y avisos importantes.' },
  { title: 'Nave y Equipamiento', category: 'Módulo', url: '#mod-ship', snippet: 'Revisión de piezas instaladas, estado de salud, mejoras aplicadas y listas rápidas antes de despegar.' },
  { title: 'Navegación y Planificador de Rutas', category: 'Módulo', url: '#mod-navigation', snippet: 'Rutas exactas adaptadas a tu nave actual o por estrellas de neutrones, paradas de repostaje y copia de saltos.' },
  { title: 'Exploración Cartográfica', category: 'Módulo', url: '#mod-exploration', snippet: 'Estrellas donde repostar, planetas descubiertos en el sistema y recomendación de cuáles mapear con sondas.' },
  { title: 'Exobiología', category: 'Módulo', url: '#mod-exobiology', snippet: 'Especies posibles en la superficie del planeta y medidor visual de distancia para completar las tres muestras.' },
  { title: 'Combate y Defensa Anti-Xeno', category: 'Módulo', url: '#mod-combat', snippet: 'Datos claros de la nave objetivo, Zonas de Conflicto, recompensas acumuladas y combates contra Thargoides.' },
  { title: 'Minería', category: 'Módulo', url: '#mod-mining', snippet: 'Composición de asteroides al prospectar, ritmo de extracción, minerales favoritos y estaciones de venta cercanas.' },
  { title: 'Comercio y Mercados', category: 'Módulo', url: '#mod-trade', snippet: 'Contenido de tu bodega, comparación de precios en la estación actual y rutas según tamaño de aterrizaje.' },
  { title: 'Misiones', category: 'Módulo', url: '#mod-missions', snippet: 'Control de tiempos de entrega y organización por facciones en misiones de caza de piratas.' },
  { title: 'Ingeniería y Materiales', category: 'Módulo', url: '#mod-engineering', snippet: 'Pasos para desbloquear cada ingeniero, almacén, taquilla a pie, lista de la compra e intercambios con sobrantes.' },
  { title: 'Fleet Carrier', category: 'Módulo', url: '#mod-carrier', snippet: 'Reservas de Tritio, cuenta atrás de saltos programados, espacio disponible y gastos semanales del portaaviones.' },
  { title: 'Colonización de Sistemas', category: 'Módulo', url: '#mod-colonisation', snippet: 'Progreso de materiales entregados para levantar nuevas estaciones y mercados cercanos donde comprarlos.' },
  { title: 'Powerplay 2.0', category: 'Módulo', url: '#mod-powerplay', snippet: 'Potencia galáctica a la que apoyas, rango actual, méritos semanales y situación de los sistemas visitados.' },
  { title: 'Facciones y Política Local (BGS)', category: 'Módulo', url: '#mod-bgs', snippet: 'Porcentajes de influencia de las facciones del sistema, estados activos y registro de tus aportaciones.' },
  { title: 'Estado Legal y Multas', category: 'Módulo', url: '#mod-crime', snippet: 'Multas pendientes, nivel de notoriedad, carga ilegal y localización de contactos neutrales Interstellar Factors.' },
  { title: 'Cuentas de la Comunidad', category: 'Módulo', url: '#mod-accounts', snippet: 'Consulta dónde tienes guardadas tus otras naves con Frontier y sincroniza opcionalmente con Inara y EDSM.' },

  // Primeros pasos
  { title: 'Instalación y Puesta en Marcha', category: 'Primeros Pasos', url: '#manual-install', snippet: 'Cómo instalar TMC en Windows, abrirlo junto al juego y encontrarlo en el icono junto al reloj.' },
  { title: 'Moverse por la Aplicación y Modo Automático', category: 'Primeros Pasos', url: '#manual-hud', snippet: 'Vistas combinadas, módulos individuales, detalles desplegables y cambio automático Auto-companion.' },
  { title: 'Usarlo en tu Tablet o Móvil', category: 'Primeros Pasos', url: '#manual-lan', snippet: 'Escanea el código QR en Ajustes para manejar los paneles desde una pantalla táctil en tu mismo Wi-Fi.' },
  { title: 'Avisos Hablados y Control por Voz', category: 'Primeros Pasos', url: '#manual-voice', snippet: 'Cómo activar la voz del copiloto, hacer consultas con micrófono o descargar el perfil para VoiceAttack.' },
  { title: 'Conectar Inara, EDSM o Frontier', category: 'Primeros Pasos', url: '#manual-accounts', snippet: 'Pasos opcionales para sincronizar tu perfil comunitario o ver tu flota aparcada en otras estaciones.' },

  // Guías de uso
  { title: 'Guía: Recoger Muestras de Exobiología', category: 'Guía de Uso', url: '#tut-exobiology', snippet: 'Cómo usar el indicador de distancia para saber cuándo escanear la segunda y tercera muestra.' },
  { title: 'Guía: Desbloquear un Ingeniero y Mejorar tu Nave', category: 'Guía de Uso', url: '#tut-engineering', snippet: 'Cómo seguir los pasos de acceso al taller, guardar tu lista de la compra y canjear materiales repetidos.' },
  { title: 'Guía: Organizar Misiones de Caza por Facciones', category: 'Guía de Uso', url: '#tut-massacre', snippet: 'Cómo ver en columnas cuántos objetivos te faltan por cada facción que te ha contratado.' },
  { title: 'Guía: Saldar Multas y Gestionar la Notoriedad', category: 'Guía de Uso', url: '#tut-crime', snippet: 'Cómo comprobar cuándo tu notoriedad baja a cero y acudir al contacto neutral más cercano.' }
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
// 5. BOTONES DE COPIADO CON AVISO VISUAL
// ===================================================================

function initCodeCopyButtons() {
  const copyButtons = document.querySelectorAll('.code-copy-btn');

  copyButtons.forEach((btn) => {
    btn.addEventListener('click', () => {
      const snippet = btn.closest('.code-snippet');
      if (!snippet) return;

      const codeText = snippet.innerText.replace(btn.innerText, '').trim();
      copyToClipboard(codeText, 'Texto copiado al portapapeles');

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
