//! Relay EDDN falso para pruebas: PUB local que re-publica un archivo comprimido.
//!
//! Uso: `fake_relay <puerto|0> <archivo.jsonl>`. Cada línea es un mensaje; se comprime
//! con zlib y se publica. Tras 1 s de calentamiento se repite el archivo cada 1 s hasta
//! que se mate el proceso (PUB pierde lo enviado antes de la suscripción; repetir es
//! idempotente). En el texto, `@NOW@` se sustituye por la hora actual (UTC).

use chrono::{SecondsFormat, Utc};
use flate2::{Compression, write::ZlibEncoder};
use std::io::Write;
use std::time::Duration;
use zeromq::{Endpoint, PubSocket, Socket, SocketSend, ZmqMessage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let port = args.next().unwrap_or_else(|| "0".into());
    let file = args
        .next()
        .ok_or("uso: fake_relay <puerto|0> <archivo.jsonl>")?;
    let text = std::fs::read_to_string(file)?;

    let mut sock = PubSocket::new();
    let ep = sock.bind(&format!("tcp://127.0.0.1:{port}")).await?;
    if let Endpoint::Tcp(_, p) = ep {
        println!("fake_relay escuchando en tcp://127.0.0.1:{p}");
    }
    let _ = std::io::stdout().flush();

    tokio::time::sleep(Duration::from_secs(1)).await;
    loop {
        let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            let mut e = ZlibEncoder::new(Vec::new(), Compression::default());
            e.write_all(line.replace("@NOW@", &now).as_bytes())?;
            sock.send(ZmqMessage::from(e.finish()?)).await?;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
