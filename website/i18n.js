/**
 * TMC — The Multipurpose Companion
 * Motor de Internacionalización (i18n) para la Web Oficial
 */

const SUPPORTED_LANGS = ['es', 'en', 'fr', 'de', 'pt'];
const DEFAULT_LANG = 'es';

const LANG_CONFIG = {
  es: { code: 'ES', label: 'Español' },
  en: { code: 'EN', label: 'English' },
  fr: { code: 'FR', label: 'Français' },
  de: { code: 'DE', label: 'Deutsch' },
  pt: { code: 'PT', label: 'Português' },
};

let currentLang = DEFAULT_LANG;

/**
 * Obtiene la traducción de una clave en el idioma actual o en el indicado
 */
function t(key, lang = currentLang) {
  if (typeof TRANSLATIONS === 'undefined') return key;
  const langDict = TRANSLATIONS[lang] || TRANSLATIONS[DEFAULT_LANG];
  return langDict[key] !== undefined ? langDict[key] : (TRANSLATIONS[DEFAULT_LANG][key] || key);
}

/**
 * Detecta el idioma inicial según URL, localStorage o navegador
 */
function detectInitialLanguage() {
  // 1. Parámetro en URL (?lang=en)
  try {
    const params = new URLSearchParams(window.location.search);
    const urlLang = params.get('lang')?.toLowerCase();
    if (urlLang && SUPPORTED_LANGS.includes(urlLang)) {
      return urlLang;
    }
  } catch {
    // ignorar
  }

  // 2. LocalStorage guardado previamente
  try {
    const saved = localStorage.getItem('tmc_website_lang')?.toLowerCase();
    if (saved && SUPPORTED_LANGS.includes(saved)) {
      return saved;
    }
  } catch {
    // ignorar
  }

  // 3. Idioma del navegador
  try {
    const navLang = (navigator.language || navigator.userLanguage || '').toLowerCase();
    for (const code of SUPPORTED_LANGS) {
      if (navLang.startsWith(code)) {
        return code;
      }
    }
  } catch {
    // ignorar
  }

  return DEFAULT_LANG;
}

/**
 * Aplica las traducciones a todos los elementos del DOM marcados con atributos data-i18n
 */
function applyTranslations(lang) {
  document.documentElement.lang = lang;

  // Actualizar título de la página y metadatos
  const pageTitle = t('page_title', lang);
  if (pageTitle) document.title = pageTitle;

  const metaDesc = document.querySelector('meta[name="description"]');
  if (metaDesc) {
    const descText = t('meta_desc', lang);
    if (descText) metaDesc.setAttribute('content', descText);
  }

  // 1. Textos y contenidos HTML con [data-i18n]
  document.querySelectorAll('[data-i18n]').forEach((el) => {
    const key = el.getAttribute('data-i18n');
    if (!key) return;
    const text = t(key, lang);
    if (text === undefined) return;

    if (text.includes('<') && text.includes('>')) {
      el.innerHTML = text;
    } else {
      el.textContent = text;
    }
  });

  // 2. Placeholders en inputs
  document.querySelectorAll('[data-i18n-placeholder]').forEach((el) => {
    const key = el.getAttribute('data-i18n-placeholder');
    if (key) el.setAttribute('placeholder', t(key, lang));
  });

  // 3. Atributos title
  document.querySelectorAll('[data-i18n-title]').forEach((el) => {
    const key = el.getAttribute('data-i18n-title');
    if (key) el.setAttribute('title', t(key, lang));
  });

  // 4. Actualizar botones del selector de idioma
  updateSelectorUI(lang);
}

/**
 * Actualiza la apariencia del selector de idioma en la barra de navegación
 */
function updateSelectorUI(lang) {
  const currentSpan = document.getElementById('langCurrentCode');

  if (currentSpan && LANG_CONFIG[lang]) {
    currentSpan.textContent = LANG_CONFIG[lang].code;
  }

  // Opciones activas en el dropdown
  document.querySelectorAll('.lang-option').forEach((opt) => {
    const optLang = opt.getAttribute('data-lang');
    if (optLang === lang) {
      opt.classList.add('active');
      opt.setAttribute('aria-selected', 'true');
    } else {
      opt.classList.remove('active');
      opt.setAttribute('aria-selected', 'false');
    }
  });
}

/**
 * Conmuta el idioma activo y emite el evento global languagechange
 */
function setLanguage(lang) {
  if (!SUPPORTED_LANGS.includes(lang)) lang = DEFAULT_LANG;
  currentLang = lang;

  try {
    localStorage.setItem('tmc_website_lang', lang);
  } catch {
    // ignorar
  }

  applyTranslations(lang);

  // Emitir evento para componentes dinámicos (Simulador, Buscador, etc.)
  window.dispatchEvent(new CustomEvent('languagechange', { detail: { lang } }));
}

/**
 * Inicializa el selector de idioma y los manejadores de eventos
 */
function initLanguageSelector() {
  const selectBtn = document.getElementById('langSelectBtn');
  const dropdown = document.getElementById('langDropdownMenu');

  if (selectBtn && dropdown) {
    selectBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      const isOpen = dropdown.classList.toggle('open');
      dropdown.style.display = isOpen ? 'flex' : 'none';
      selectBtn.setAttribute('aria-expanded', String(isOpen));
    });

    // Cerrar al hacer clic fuera
    document.addEventListener('click', (e) => {
      if (!dropdown.contains(e.target) && e.target !== selectBtn) {
        dropdown.classList.remove('open');
        dropdown.style.display = 'none';
        selectBtn.setAttribute('aria-expanded', 'false');
      }
    });

    // Cerrar con Escape
    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && dropdown.classList.contains('open')) {
        dropdown.classList.remove('open');
        dropdown.style.display = 'none';
        selectBtn.setAttribute('aria-expanded', 'false');
      }
    });
  }

  // Event listeners para todas las opciones de idioma
  document.querySelectorAll('.lang-option').forEach((opt) => {
    opt.addEventListener('click', (e) => {
      e.preventDefault();
      const targetLang = opt.getAttribute('data-lang');
      if (targetLang) {
        setLanguage(targetLang);
        if (dropdown) {
          dropdown.classList.remove('open');
          dropdown.style.display = 'none';
          if (selectBtn) selectBtn.setAttribute('aria-expanded', 'false');
        }
      }
    });
  });

  // Idioma inicial
  const initial = detectInitialLanguage();
  setLanguage(initial);
}

// Inicializar al cargar el DOM
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initLanguageSelector);
} else {
  initLanguageSelector();
}
