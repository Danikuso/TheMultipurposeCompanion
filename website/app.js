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
  { title: 'Cualidades Principales de TMC', category: 'Filosofía', url: '#cualidades', snippet: 'Guía sin bloqueos, colores sincronizados con tu cabina (HUD automático), 10 disposiciones, voz, pantalla QR en tablet y código abierto.' },

  // Módulos (16)
  { title: 'Panel General (7 paneles)', category: 'Módulo', url: '#mod-dashboard', snippet: 'Créditos, 6 rangos de la Federación de Pilotos + Superpotencias, 7 testigos en cabina (SCO), ruta, sesión y alertas críticas.' },
  { title: 'Nave, Astilleros y Equipamiento', category: 'Módulo', url: '#mod-ship', snippet: 'Buscador de naves y componentes cercanos desde tu sistema actual (−15 % Li Yong-Rui y Stronghold Carriers de tu potencia), listas antes de despegar y mejoras.' },
  { title: 'Navegación y Planificador de Rutas', category: 'Módulo', url: '#mod-navigation', snippet: 'Ruta exacta con las características reales de tu nave, Autopista de neutrones, salida desde espacio profundo y copiado salto a salto.' },
  { title: 'Exploración Cartográfica', category: 'Módulo', url: '#mod-exploration', snippet: 'Estrellas repostables, avance de escaneo, recomendación ★ Mapear con meta de sondas y alerta de Datos en riesgo.' },
  { title: 'Exobiología (Odyssey)', category: 'Módulo', url: '#mod-exobiology', snippet: 'Bono ×5 en mundos sin registrar, posibles especies, distancia mínima y dirección relativa hacia la muestra anterior.' },
  { title: 'Combate y Defensa Anti-Xeno', category: 'Módulo', url: '#mod-combat', snippet: 'Aviso LIMPIO / BUSCADO, salud del subsistema apuntado, vales sin cobrar (−25 % en gestores neutrales), Zonas de Conflicto y Thargoides.' },
  { title: 'Minería y Venta Óptima', category: 'Módulo', url: '#mod-mining', snippet: 'Bucle triangular más corto, buscador de yacimientos (Láser, Núcleos y Bromellita), alerta de tasa de demanda, veredicto ✔ merece la pena, drones y venta de bodega combinada.' },
  { title: 'Comercio y Mercados', category: 'Módulo', url: '#mod-trade', snippet: 'Mercancías raras, precio medio pagado en bodega (VENDER AQUÍ / LLEVAR), rutas comerciales multiparada y buscador de productos.' },
  { title: 'Misiones, Turismo y Objetivos Comunitarios', category: 'Módulo', url: '#mod-missions', snippet: 'Pila de masacre por facción emisora, alerta de pasajero buscado, Ruta óptima de balizas turísticas y seguimiento de eventos CG.' },
  { title: 'Ingeniería y Materiales (Nave y Odyssey)', category: 'Módulo', url: '#mod-engineering', snippet: 'Almacén y Taquilla a pie, Guía en 3 pasos para ingenieros bloqueados, Lista de la compra con prioridades, trueques y Brokers.' },
  { title: 'Fleet Carrier', category: 'Módulo', url: '#mod-carrier', snippet: 'Tritio combinado en depósito y bodega, saltos posibles de 500 AL, cuenta atrás de 15 min y semanas cubiertas de mantenimiento.' },
  { title: 'Colonización de Sistemas', category: 'Módulo', url: '#mod-colonisation', snippet: 'Porcentaje de avance de la obra, materiales pendientes y cálculo automático de Viajes estimados según la bodega de tu nave.' },
  { title: 'Powerplay 2.0', category: 'Módulo', url: '#mod-powerplay', snippet: 'Potencia galáctica, semanas de lealtad, rango hasta 100, ritmo de méritos por hora y cuenta atrás al cierre semanal.' },
  { title: 'Facciones y Política Local (BGS)', category: 'Módulo', url: '#mod-bgs', snippet: 'Barras de influencia local, estados activos, gobierno y monitor del corte diario galáctico (Tick).' },
  { title: 'Estado Legal, Multas y Contrabando', category: 'Módulo', url: '#mod-crime', snippet: 'Reloj de notoriedad (pausado en estación), contacto Interstellar Factor más cercano y guía en 3 pasos de entrada sigilosa a mercados negros.' },
  { title: 'Cuentas y Flota Guardada', category: 'Módulo', url: '#mod-accounts', snippet: 'Visor de naves aparcadas en otros sistemas con Frontier, sincronización oficial con Inara (Whitelist), EDSM y red comunitaria EDDN.' },

  // Guía rápida (6 capítulos)
  { title: 'Instalación y Puesta en Marcha', category: 'Guía Rápida', url: '#manual-install', snippet: 'Cómo instalar TMC en Windows (y detección en Linux / Steam Deck), importar tu historial y usarlo junto al reloj del sistema.' },
  { title: 'Cómo Moverte por la Pantalla y Modo Automático (Auto-Companion Pro)', category: 'Guía Rápida', url: '#manual-hud', snippet: 'Indicadores superiores, candado táctico (Alt + L), prioridades a tu medida, anticipación y las 10 Disposiciones combinadas.' },
  { title: 'Colores HUD Automático, Preferencias y Sugerencias', category: 'Guía Rápida', url: '#manual-settings', snippet: 'Sincroniza los colores con la cabina de tu nave, 5 idiomas, umbrales por actividad, modo sin internet y buzón de sugerencias.' },
  { title: 'Usarlo como Pantalla Táctil en Tablet o Móvil', category: 'Guía Rápida', url: '#manual-lan', snippet: 'Escanea el código QR en Ajustes para manejar todos los paneles desde tu mismo Wi-Fi y copiar destinos al ordenador.' },
  { title: 'Avisos Hablados, 9 Preguntas por Micrófono y VoiceAttack', category: 'Guía Rápida', url: '#manual-voice', snippet: 'Voces del sistema o IA Kokoro, 5 canales de aviso, las 9 consultas habladas (incluso sin internet) y perfil automático .vap.' },
  { title: 'Conectar Frontier, Inara, EDSM o Red Comunitaria', category: 'Guía Rápida', url: '#manual-accounts', snippet: 'Pasos opcionales para localizar tu flota aparcada con Frontier o sincronizar tus viajes con Inara y EDSM.' },

  // Tutoriales (8 guías paso a paso)
  { title: 'Tutorial: Trazar un Viaje Largo Adaptado a tu Nave', category: 'Tutorial', url: '#tut-navigation', snippet: 'Cómo calcular una Ruta exacta o Autopista de neutrones en Spansh y pegar cada salto con Ctrl + V.' },
  { title: 'Tutorial: Explorar un Sistema y Mapear lo Valioso', category: 'Tutorial', url: '#tut-exploration', snippet: 'Cómo identificar estrellas repostables, planetas marcados con ★ Mapear y vigilar tus datos cartográficos.' },
  { title: 'Tutorial: Recoger las 3 Muestras de Exobiología', category: 'Tutorial', url: '#tut-exobiology', snippet: 'Cómo seguir los metros restantes y la indicación de dirección respecto a la muestra anterior.' },
  { title: 'Tutorial: Desbloquear un Ingeniero y Mejorar tu Nave', category: 'Tutorial', url: '#tut-engineering', snippet: 'Cómo completar los 3 pasos de acceso al taller, priorizar tu lista de la compra y canjear materiales sobrantes.' },
  { title: 'Tutorial: Minería de Asteroides y Venta Óptima', category: 'Tutorial', url: '#tut-mining', snippet: 'Cómo planificar el bucle triangular hacia el mejor anillo, evitar penalizaciones de demanda, prospectar y vender toda tu bodega al máximo precio.' },
  { title: 'Tutorial: Comercio Multiparada y Obras de Colonización', category: 'Tutorial', url: '#tut-trade', snippet: 'Cómo seguir rutas comerciales de varias escalas, vender mercancías raras y calcular los viajes restantes de una obra.' },
  { title: 'Tutorial: Caza por Facciones (Masacre) y Turismo', category: 'Tutorial', url: '#tut-massacre', snippet: 'Cómo solapar misiones piratas de distintas facciones y ordenar balizas turísticas en la ruta más corta.' },
  { title: 'Tutorial: Limpiar Multas y Entrar con Sigilo a Puerto', category: 'Tutorial', url: '#tut-crime', snippet: 'Cómo esperar a que baje la notoriedad para pagar en contactos neutrales o entrar sin escaneos a un mercado negro.' }
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
  const sections = Array.from(document.querySelectorAll('section[id]'));
  const chapters = Array.from(document.querySelectorAll('.manual-chapter[id]'));
  const navLinks = document.querySelectorAll('.nav-link');
  const manualNavItems = document.querySelectorAll('.manual-nav-item');

  function updateActiveItems() {
    const sectionTriggerY = Math.min(200, window.innerHeight * 0.25);
    const chapterTriggerY = Math.min(220, window.innerHeight * 0.28);

    // 1. Barra de navegación superior
    let activeSectionId = null;
    for (const section of sections) {
      const rect = section.getBoundingClientRect();
      if (rect.top <= sectionTriggerY && rect.bottom > sectionTriggerY) {
        activeSectionId = section.getAttribute('id');
      }
    }

    navLinks.forEach((link) => {
      const isMatch = activeSectionId && link.getAttribute('href') === `#${activeSectionId}`;
      link.classList.toggle('active', Boolean(isMatch));
    });

    // 2. Índice lateral de «Guía Rápida» (usando coordenadas reales del viewport,
    // inmune a contenedores padres con position: relative como .container)
    if (chapters.length > 0) {
      let activeChapterId = chapters[0].getAttribute('id');

      for (const chapter of chapters) {
        const rect = chapter.getBoundingClientRect();
        if (rect.top <= chapterTriggerY) {
          activeChapterId = chapter.getAttribute('id');
        }
      }

      manualNavItems.forEach((link) => {
        const isMatch = link.getAttribute('href') === `#${activeChapterId}`;
        link.classList.toggle('active', isMatch);
      });
    }
  }

  window.addEventListener('scroll', updateActiveItems, { passive: true });
  window.addEventListener('resize', updateActiveItems, { passive: true });
  updateActiveItems();
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
