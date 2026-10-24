//! Cola priorizada de locución con descarte de avisos obsoletos y cooldown.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tmc_model::tts::TtsPriority;

/// Elemento en cola listo para ser sintetizado.
#[derive(Debug, Clone)]
pub struct QueuedSpeech {
    pub id: u64,
    pub priority: TtsPriority,
    pub text: String,
    pub voice: Option<String>,
    pub volume: u8,
    pub rate: f32,
    pub cooldown_key: Option<String>,
    pub enqueued_at: Instant,
}

/// Cola priorizada de locuciones del copiloto auditivo.
#[derive(Debug)]
pub struct TtsQueue {
    /// Avisos de emergencia (máxima prioridad, interrumpen o van primeros).
    emergency: VecDeque<QueuedSpeech>,
    /// Avisos tácticos (combate, saltos, bio-señales).
    tactical: VecDeque<QueuedSpeech>,
    /// Avisos informativos generales.
    informational: VecDeque<QueuedSpeech>,
    /// Registro de última emisión por clave de cooldown.
    last_emitted: HashMap<String, Instant>,
    /// Antigüedad máxima en cola antes de descartar (por defecto 4 segundos).
    max_age: Duration,
    /// Cooldown predeterminado entre avisos con la misma clave (por defecto 15 segundos).
    default_cooldown: Duration,
    /// Contador secuencial de identificadores.
    next_id: u64,
}

impl Default for TtsQueue {
    fn default() -> Self {
        Self::new(Duration::from_secs(4), Duration::from_secs(15))
    }
}

impl TtsQueue {
    pub fn new(max_age: Duration, default_cooldown: Duration) -> Self {
        Self {
            emergency: VecDeque::new(),
            tactical: VecDeque::new(),
            informational: VecDeque::new(),
            last_emitted: HashMap::new(),
            max_age,
            default_cooldown,
            next_id: 1,
        }
    }

    /// Intenta encolar un aviso. Retorna `true` si fue aceptado o `false` si fue suprimido por cooldown.
    pub fn enqueue(
        &mut self,
        priority: TtsPriority,
        text: impl Into<String>,
        voice: Option<String>,
        volume: u8,
        rate: f32,
        cooldown_key: Option<String>,
    ) -> bool {
        let now = Instant::now();

        // Comprobación de cooldown si se especificó clave
        if let Some(key) = &cooldown_key
            && let Some(last) = self.last_emitted.get(key)
            && now.duration_since(*last) < self.default_cooldown
        {
            // Ignorado por cooldown
            return false;
        }

        let id = self.next_id;
        self.next_id += 1;

        let item = QueuedSpeech {
            id,
            priority,
            text: text.into(),
            voice,
            volume,
            rate,
            cooldown_key,
            enqueued_at: now,
        };

        match priority {
            TtsPriority::Emergency => self.emergency.push_back(item),
            TtsPriority::Tactical => self.tactical.push_back(item),
            TtsPriority::Informational => self.informational.push_back(item),
        }

        true
    }

    /// Extrae el siguiente aviso a locutar, descartando automáticamente los que hayan caducado (> max_age).
    pub fn pop_next(&mut self) -> Option<QueuedSpeech> {
        let now = Instant::now();

        // 1. Probar emergencias
        while let Some(item) = self.emergency.pop_front() {
            if now.duration_since(item.enqueued_at) <= self.max_age {
                self.record_emission(&item, now);
                return Some(item);
            }
        }

        // 2. Probar tácticos
        while let Some(item) = self.tactical.pop_front() {
            if now.duration_since(item.enqueued_at) <= self.max_age {
                self.record_emission(&item, now);
                return Some(item);
            }
        }

        // 3. Probar informativos
        while let Some(item) = self.informational.pop_front() {
            if now.duration_since(item.enqueued_at) <= self.max_age {
                self.record_emission(&item, now);
                return Some(item);
            }
        }

        None
    }

    /// Limpia todas las colas de locución pendientes.
    pub fn clear(&mut self) {
        self.emergency.clear();
        self.tactical.clear();
        self.informational.clear();
    }

    /// Retorna el número total de avisos pendientes en todas las colas.
    pub fn len(&self) -> usize {
        self.emergency.len() + self.tactical.len() + self.informational.len()
    }

    /// Retorna `true` si no hay avisos pendientes en ninguna cola.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn record_emission(&mut self, item: &QueuedSpeech, now: Instant) {
        if let Some(key) = &item.cooldown_key {
            self.last_emitted.insert(key.clone(), now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_ordering() {
        let mut queue = TtsQueue::default();

        queue.enqueue(TtsPriority::Informational, "Info 1", None, 80, 1.0, None);
        queue.enqueue(TtsPriority::Tactical, "Tactical 1", None, 80, 1.0, None);
        queue.enqueue(TtsPriority::Emergency, "Emergency 1", None, 80, 1.0, None);
        queue.enqueue(TtsPriority::Informational, "Info 2", None, 80, 1.0, None);

        // Emergency debe salir primero
        let first = queue.pop_next().expect("debe haber elemento");
        assert_eq!(first.text, "Emergency 1");

        // Luego Tactical
        let second = queue.pop_next().expect("debe haber elemento");
        assert_eq!(second.text, "Tactical 1");

        // Luego los Informational en orden FIFO
        let third = queue.pop_next().expect("debe haber elemento");
        assert_eq!(third.text, "Info 1");

        let fourth = queue.pop_next().expect("debe haber elemento");
        assert_eq!(fourth.text, "Info 2");

        assert!(queue.pop_next().is_none());
    }

    #[test]
    fn cooldown_suppresses_rapid_duplicates() {
        let mut queue = TtsQueue::new(Duration::from_secs(4), Duration::from_secs(10));

        let accepted1 = queue.enqueue(
            TtsPriority::Tactical,
            "Temperatura crítica 1",
            None,
            80,
            1.0,
            Some("heat_warning".into()),
        );
        assert!(accepted1);

        // Al extraerlo se registra la emisión
        let popped = queue.pop_next().unwrap();
        assert_eq!(popped.text, "Temperatura crítica 1");

        // Intentar meter otro de inmediato con la misma clave debe ser rechazado
        let accepted2 = queue.enqueue(
            TtsPriority::Tactical,
            "Temperatura crítica 2",
            None,
            80,
            1.0,
            Some("heat_warning".into()),
        );
        assert!(!accepted2);
        assert!(queue.is_empty());

        // Con otra clave sí es aceptado
        let accepted3 = queue.enqueue(
            TtsPriority::Emergency,
            "Escudos caídos",
            None,
            80,
            1.0,
            Some("shields_down".into()),
        );
        assert!(accepted3);
        assert_eq!(queue.pop_next().unwrap().text, "Escudos caídos");
    }

    #[test]
    fn stale_items_are_dropped() {
        // Cola con caducidad de 10 milisegundos
        let mut queue = TtsQueue::new(Duration::from_millis(10), Duration::from_secs(10));

        queue.enqueue(
            TtsPriority::Informational,
            "Mensaje viejo",
            None,
            80,
            1.0,
            None,
        );

        std::thread::sleep(Duration::from_millis(25));

        // Debe descartarse por caducado
        assert!(queue.pop_next().is_none());
        assert!(queue.is_empty());
    }
}
