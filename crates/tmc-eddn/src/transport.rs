//! Transporte: abstracción de fuente de fotogramas y adaptador ZeroMQ real.

use std::time::Duration;
use zeromq::{Socket, SocketRecv, SubSocket};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

pub const RELAY_URL: &str = "tcp://eddn.edcd.io:9500";

/// Cuándo mantiene EDDN la conexión.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EddnMode {
    Always,
    /// Conecta cuando el filtro está activo y se desconecta al desactivarlo.
    OnDemand,
}

#[derive(Debug, thiserror::Error)]
pub enum EddnError {
    #[error("transporte EDDN: {0}")]
    Transport(String),
    #[error("no conectado")]
    NotConnected,
}

/// Tope de un mensaje comprimido recibido (los de EDDN miden unos pocos KB).
pub const MAX_COMPRESSED_BYTES: usize = 256 * 1024;
/// Partes máximas aceptadas por mensaje.
pub const MAX_PARTS: usize = 2;

/// Une las partes de un mensaje. Si excede los topes devuelve un fotograma vacío
/// (que el decodificador cuenta como corrupto) sin reservar la concatenación.
pub fn frame_from_parts(parts: &[&[u8]]) -> Vec<u8> {
    let total: usize = parts.iter().map(|p| p.len()).sum();
    if parts.len() > MAX_PARTS || total > MAX_COMPRESSED_BYTES {
        return Vec::new();
    }
    parts.concat()
}

pub trait FrameSource: Send {
    /// Puede bloquear hasta 5 s (tope de conexión de `ZmqSource`).
    fn connect(&mut self, url: &str) -> Result<(), EddnError>;
    /// `Ok(None)` si vence `timeout` sin mensaje.
    fn recv(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, EddnError>;
    fn disconnect(&mut self);
}

/// SUB sin filtro de tema sobre `zeromq` (Rust puro), con un runtime tokio propio
/// de un solo hilo que se usa solo desde el hilo del suscriptor.
pub struct ZmqSource {
    rt: tokio::runtime::Runtime,
    sock: Option<SubSocket>,
}

impl ZmqSource {
    pub fn new() -> Result<Self, EddnError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| EddnError::Transport(e.to_string()))?;
        Ok(Self { rt, sock: None })
    }
}

fn terr(e: impl std::fmt::Display) -> EddnError {
    EddnError::Transport(e.to_string())
}

impl FrameSource for ZmqSource {
    fn connect(&mut self, url: &str) -> Result<(), EddnError> {
        self.disconnect();
        let sock = self.rt.block_on(async {
            let mut s = SubSocket::new();
            // Sin tope propio, `zeromq` espera 30 s a un relay caído.
            tokio::time::timeout(CONNECT_TIMEOUT, s.connect(url))
                .await
                .map_err(|_| EddnError::Transport("tiempo de conexión agotado".into()))?
                .map_err(terr)?;
            s.subscribe("").await.map_err(terr)?;
            Ok::<_, EddnError>(s)
        })?;
        self.sock = Some(sock);
        Ok(())
    }

    fn recv(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, EddnError> {
        let sock = self.sock.as_mut().ok_or(EddnError::NotConnected)?;
        match self
            .rt
            .block_on(async { tokio::time::timeout(timeout, sock.recv()).await })
        {
            Err(_) => Ok(None),
            Ok(Err(e)) => Err(terr(e)),
            Ok(Ok(msg)) => {
                let parts: Vec<&[u8]> = msg.iter().map(|b| b.as_ref()).collect();
                Ok(Some(frame_from_parts(&parts)))
            }
        }
    }

    fn disconnect(&mut self) {
        if let Some(s) = self.sock.take() {
            let _ = self.rt.block_on(async { s.close().await });
        }
    }
}

impl Drop for ZmqSource {
    fn drop(&mut self) {
        self.disconnect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeromq::{PubSocket, SocketSend, ZmqMessage};

    #[test]
    fn oversized_or_multipart_messages_are_rejected_without_concat() {
        let big = vec![1u8; MAX_COMPRESSED_BYTES + 1];
        assert!(frame_from_parts(&[&big]).is_empty());
        assert!(frame_from_parts(&[&big[..10], &big[..10], &big[..10]]).is_empty());
        let ok = vec![1u8; MAX_COMPRESSED_BYTES];
        assert_eq!(frame_from_parts(&[&ok]).len(), MAX_COMPRESSED_BYTES);
        assert_eq!(frame_from_parts(&[b"ab", b"cd"]), b"abcd");
        assert_eq!(
            crate::decode(&frame_from_parts(&[&big]), chrono::Utc::now()),
            crate::Decoded::Ignored(crate::DropReason::Corrupt)
        );
    }

    #[test]
    fn zmq_loopback_roundtrip() {
        // Multihilo: la tarea de aceptación del PUB debe avanzar mientras el SUB bloquea.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        let (mut publisher, port) = rt.block_on(async {
            let mut p = PubSocket::new();
            let ep = p.bind("tcp://127.0.0.1:0").await.unwrap();
            let port = match ep {
                zeromq::Endpoint::Tcp(_, port) => port,
                other => panic!("{other:?}"),
            };
            (p, port)
        });
        let payload: Vec<u8> = (0u8..=255).collect();
        let mut src = ZmqSource::new().unwrap();
        src.connect(&format!("tcp://127.0.0.1:{port}")).unwrap();
        let start = std::time::Instant::now();
        let got = loop {
            // PUB descarta lo enviado antes de la suscripción: se repite.
            rt.block_on(async {
                publisher
                    .send(ZmqMessage::from(payload.clone()))
                    .await
                    .unwrap()
            });
            if let Some(f) = src.recv(Duration::from_millis(100)).unwrap() {
                break f;
            }
            assert!(start.elapsed() < Duration::from_secs(2), "sin frame en 2 s");
        };
        assert_eq!(got, payload);
        src.disconnect();
    }
}
