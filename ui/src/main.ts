import './styles/base.css';
import { mount } from 'svelte';
import App from './App.svelte';

// F18/F21: Limpiar el token de la barra de direcciones y del historial tras cargar
try {
  if (typeof window !== 'undefined' && window.location) {
    const url = new URL(window.location.href);
    if (url.searchParams.has('token')) {
      url.searchParams.delete('token');
      window.history.replaceState({}, '', url.pathname + (url.search ? url.search : '') + url.hash);
    }
  }
} catch {
  // Ignorar en entornos sin window/URL
}

const app = mount(App, {
  target: document.getElementById('app')!,
});

export default app;
