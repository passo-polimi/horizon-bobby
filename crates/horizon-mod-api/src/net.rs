//! Networking custom e gestione dei pacchetti CustomPayload (Task T2.4).
//! Garantisce parità bit a bit con i formati Java (VarInt, UTF-8 con prefisso di lunghezza).

use std::{
    collections::HashMap,
    io::{self, Read, Write},
    sync::{Arc, Mutex},
};

/// Codifica un intero a 32 bit nel formato VarInt di Minecraft.
pub fn write_varint<W: Write>(w: &mut W, mut value: i32) -> io::Result<usize> {
    let mut bytes_written = 0;
    loop {
        let mut byte = (value & 0x7F) as u8;
        value = (value as u32 >> 7) as i32;
        if value != 0 {
            byte |= 0x80;
        }
        w.write_all(&[byte])?;
        bytes_written += 1;
        if value == 0 {
            break;
        }
    }
    Ok(bytes_written)
}

/// Decodifica un VarInt di Minecraft da uno stream di byte.
pub fn read_varint<R: Read>(r: &mut R) -> io::Result<i32> {
    let mut value = 0;
    let mut position = 0;
    let mut buf = [0u8; 1];

    loop {
        r.read_exact(&mut buf)?;
        let byte = buf[0];
        value |= ((byte & 0x7F) as i32) << position;

        if (byte & 0x80) == 0 {
            break;
        }

        position += 7;
        if position >= 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "VarInt troppo grande",
            ));
        }
    }

    Ok(value)
}

/// Scrive una stringa UTF-8 preceduta dalla sua lunghezza in VarInt.
pub fn write_string<W: Write>(w: &mut W, s: &str) -> io::Result<()> {
    write_varint(w, s.len() as i32)?;
    w.write_all(s.as_bytes())
}

/// Legge una stringa UTF-8 preceduta da VarInt.
pub fn read_string<R: Read>(r: &mut R) -> io::Result<String> {
    let len = read_varint(r)? as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub type ChannelReceiver = Arc<dyn Fn(&[u8]) + Send + Sync>;

/// Registro globale thread-safe per canali di rete CustomPayload.
#[derive(Default)]
pub struct NetworkRegistry {
    receivers: Mutex<HashMap<String, Vec<ChannelReceiver>>>,
}

impl NetworkRegistry {
    pub fn new() -> Self {
        Self {
            receivers: Mutex::new(HashMap::new()),
        }
    }

    /// Registra un ricevitore per un canale (es. "distanthorizons:data").
    pub fn register_receiver<F: Fn(&[u8]) + Send + Sync + 'static>(
        &self,
        channel: &str,
        receiver: F,
    ) {
        let mut lock = self.receivers.lock().unwrap();
        lock.entry(channel.to_string())
            .or_default()
            .push(Arc::new(receiver));
    }

    /// Invia un payload in ingresso a tutti i ricevitori registrati per quel canale.
    pub fn dispatch(&self, channel: &str, data: &[u8]) {
        let receivers = {
            let lock = self.receivers.lock().unwrap();
            match lock.get(channel) {
                Some(list) => list.clone(),
                None => return,
            }
        };

        for r in receivers {
            r(data);
        }
    }
}
