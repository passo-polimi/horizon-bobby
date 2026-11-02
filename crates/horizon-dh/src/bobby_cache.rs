//! Livello 1: Fake-Chunk Cache (Stile "Bobby").
//!
//! Permette di memorizzare su disco e in RAM i chunk completi a pieno dettaglio,
//! indicizzati per (server_id, dimensione, x, z). Quando il client è connesso a un server
//! con view distance ridotta (es. 6-10 chunk), questa cache serve al `WorldManager`
//! di Horizon i chunk memorizzati localmente fino alla render distance del client (es. 24-32 chunk).

use super::anvil_importer::ParsedChunk;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

const BOBBY_MAGIC: &[u8; 4] = b"BBY1";
const MAX_RAM_CHUNKS: usize = 512; // Massimo 512 chunk in RAM (~100-150 MB max)

/// Gestore della cache client-side dei chunk completi.
#[derive(Clone)]
pub struct BobbyCache {
    base_dir: PathBuf,
    pub server_id: String,
    pub dimension: String,
    memory_cache: Arc<RwLock<HashMap<(i32, i32), Arc<ParsedChunk>>>>,
}

impl BobbyCache {
    pub fn new(root: impl AsRef<Path>, server_id: &str, dimension: &str) -> io::Result<Self> {
        let base_dir = root
            .as_ref()
            .join("horizon_chunk_cache")
            .join(server_id)
            .join(dimension);
        fs::create_dir_all(&base_dir)?;
        Ok(Self {
            base_dir,
            server_id: server_id.to_string(),
            dimension: dimension.to_string(),
            memory_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn chunk_path(&self, x: i32, z: i32) -> PathBuf {
        let rx = x.div_euclid(32);
        let rz = z.div_euclid(32);
        self.base_dir
            .join(format!("r.{rx}.{rz}"))
            .join(format!("c.{x}.{z}.bin"))
    }

    /// Salva un chunk nella cache su disco e in memoria (con limite di sicurezza sulla RAM).
    pub fn put(&self, chunk: Arc<ParsedChunk>) -> io::Result<()> {
        let path = self.chunk_path(chunk.x, chunk.z);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serialize_bobby_chunk(&chunk)?;
        fs::write(&path, bytes)?;

        let mut mem = self.memory_cache.write().unwrap();
        if mem.len() >= MAX_RAM_CHUNKS {
            // Rimuovi una voce arbitraria per mantenere la RAM bassa e costante
            if let Some(&oldest_key) = mem.keys().next() {
                mem.remove(&oldest_key);
            }
        }
        mem.insert((chunk.x, chunk.z), chunk);
        Ok(())
    }

    /// Salva il chunk SOLO su disco senza occupare RAM (fondamentale durante il pre-caricamento bulk).
    pub fn put_disk_only(&self, chunk: &ParsedChunk) -> io::Result<()> {
        let path = self.chunk_path(chunk.x, chunk.z);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serialize_bobby_chunk(chunk)?;
        fs::write(&path, bytes)?;
        Ok(())
    }

    /// Salva in batch una lista di chunk solo su disco (zero accumulo in RAM).
    pub fn bulk_put_disk_only(&self, chunks: &[ParsedChunk]) -> io::Result<usize> {
        let mut count = 0;
        for c in chunks {
            self.put_disk_only(c)?;
            count += 1;
        }
        Ok(count)
    }

    /// Svuota completamente la cache in memoria RAM (i dati restano al sicuro su disco).
    pub fn clear_memory(&self) {
        self.memory_cache.write().unwrap().clear();
    }

    /// Recupera un chunk: prima verifica la memoria, poi carica da disco.
    pub fn get(&self, x: i32, z: i32) -> Option<Arc<ParsedChunk>> {
        if let Some(cached) = self.memory_cache.read().unwrap().get(&(x, z)) {
            return Some(cached.clone());
        }
        let path = self.chunk_path(x, z);
        if !path.exists() {
            return None;
        }
        let bytes = fs::read(path).ok()?;
        let chunk = Arc::new(deserialize_bobby_chunk(&bytes).ok()?);
        self.memory_cache
            .write()
            .unwrap()
            .insert((x, z), chunk.clone());
        Some(chunk)
    }

    /// Verifica rapida se il chunk esiste nella cache (in memoria o su disco).
    pub fn contains(&self, x: i32, z: i32) -> bool {
        if self.memory_cache.read().unwrap().contains_key(&(x, z)) {
            return true;
        }
        self.chunk_path(x, z).exists()
    }

    /// Salva in batch una lista di chunk.
    pub fn bulk_put(&self, chunks: &[ParsedChunk]) -> io::Result<usize> {
        let mut count = 0;
        for c in chunks {
            self.put(Arc::new(c.clone()))?;
            count += 1;
        }
        Ok(count)
    }

    /// Numero di chunk attualmente caricati in RAM.
    pub fn memory_count(&self) -> usize {
        self.memory_cache.read().unwrap().len()
    }

    /// Ritorna le coordinate di tutti i chunk memorizzati in memoria.
    pub fn cached_coords(&self) -> Vec<(i32, i32)> {
        self.memory_cache.read().unwrap().keys().copied().collect()
    }
}

// -------------------------------------------------------------
// Serializzazione / Deserializzazione Veloce Bobby Binary Format
// -------------------------------------------------------------

fn serialize_bobby_chunk(chunk: &ParsedChunk) -> io::Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(8192);
    buf.extend_from_slice(BOBBY_MAGIC);
    buf.extend_from_slice(&chunk.x.to_le_bytes());
    buf.extend_from_slice(&chunk.z.to_le_bytes());
    buf.push(chunk.min_section_y as u8);
    buf.extend_from_slice(&chunk.inhabited_time.to_le_bytes());
    buf.extend_from_slice(&chunk.last_update.to_le_bytes());
    buf.extend_from_slice(&(chunk.sections.len() as u16).to_le_bytes());

    for sec in &chunk.sections {
        buf.push(sec.y as u8);
        buf.extend_from_slice(&(sec.palette.len() as u16).to_le_bytes());
        for p in &sec.palette {
            let name_bytes = p.name.as_bytes();
            buf.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            buf.extend_from_slice(name_bytes);
            if let Some(props) = &p.properties {
                buf.extend_from_slice(&(props.len() as u16).to_le_bytes());
                for (k, v) in props {
                    let k_bytes = k.as_bytes();
                    buf.extend_from_slice(&(k_bytes.len() as u16).to_le_bytes());
                    buf.extend_from_slice(k_bytes);
                    let v_bytes = v.as_bytes();
                    buf.extend_from_slice(&(v_bytes.len() as u16).to_le_bytes());
                    buf.extend_from_slice(v_bytes);
                }
            } else {
                buf.extend_from_slice(&0u16.to_le_bytes());
            }
        }
        for b in &sec.blocks {
            buf.extend_from_slice(&b.to_le_bytes());
        }
        buf.extend_from_slice(&(sec.biome_palette.len() as u16).to_le_bytes());
        for bp in &sec.biome_palette {
            let bp_bytes = bp.as_bytes();
            buf.extend_from_slice(&(bp_bytes.len() as u16).to_le_bytes());
            buf.extend_from_slice(bp_bytes);
        }
        for bio in &sec.biomes {
            buf.extend_from_slice(&bio.to_le_bytes());
        }
    }

    if let Some(h) = &chunk.heightmap_surface {
        buf.push(1);
        for &val in h {
            buf.extend_from_slice(&val.to_le_bytes());
        }
    } else {
        buf.push(0);
    }

    Ok(buf)
}

fn deserialize_bobby_chunk(data: &[u8]) -> io::Result<ParsedChunk> {
    if data.len() < 4 + 4 + 4 + 1 + 8 + 8 + 2 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "File troppo corto",
        ));
    }
    if &data[0..4] != BOBBY_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Magic non valido",
        ));
    }
    let mut offset = 4;
    let x = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
    offset += 4;
    let z = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
    offset += 4;
    let min_section_y = data[offset] as i8;
    offset += 1;
    let inhabited_time = i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    let last_update = i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    let num_sections = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
    offset += 2;

    let mut sections = Vec::with_capacity(num_sections);
    for _ in 0..num_sections {
        let sec_y = data[offset] as i8;
        offset += 1;
        let pal_len = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
        offset += 2;
        let mut palette = Vec::with_capacity(pal_len);
        for _ in 0..pal_len {
            let n_len = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
            offset += 2;
            let name = String::from_utf8_lossy(&data[offset..offset + n_len]).to_string();
            offset += n_len;
            let prop_len =
                u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
            offset += 2;
            let properties = if prop_len > 0 {
                let mut map = HashMap::with_capacity(prop_len);
                for _ in 0..prop_len {
                    let k_len =
                        u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
                    offset += 2;
                    let k = String::from_utf8_lossy(&data[offset..offset + k_len]).to_string();
                    offset += k_len;
                    let v_len =
                        u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
                    offset += 2;
                    let v = String::from_utf8_lossy(&data[offset..offset + v_len]).to_string();
                    offset += v_len;
                    map.insert(k, v);
                }
                Some(map)
            } else {
                None
            };
            palette.push(super::anvil_importer::BlockStateNbt { name, properties });
        }
        let mut blocks = [0u16; 4096];
        for b in &mut blocks {
            *b = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
            offset += 2;
        }
        let bio_pal_len = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
        offset += 2;
        let mut biome_palette = Vec::with_capacity(bio_pal_len);
        for _ in 0..bio_pal_len {
            let b_len = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap()) as usize;
            offset += 2;
            let b_name = String::from_utf8_lossy(&data[offset..offset + b_len]).to_string();
            offset += b_len;
            biome_palette.push(b_name);
        }
        let mut biomes = [0u16; 64];
        for bio in &mut biomes {
            *bio = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
            offset += 2;
        }
        sections.push(super::anvil_importer::UnpackedSection {
            y: sec_y,
            palette,
            blocks,
            biome_palette,
            biomes,
            block_light: None,
            sky_light: None,
        });
    }

    let heightmap_surface = if offset < data.len() && data[offset] == 1 {
        offset += 1;
        let mut h = [0i16; 256];
        for val in &mut h {
            *val = i16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
            offset += 2;
        }
        Some(h)
    } else {
        None
    };

    Ok(ParsedChunk {
        x,
        z,
        min_section_y,
        inhabited_time,
        last_update,
        sections,
        heightmap_surface,
        heightmap_ocean: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_bobby_cache_roundtrip() {
        let dir = tempdir().unwrap();
        let cache = BobbyCache::new(dir.path(), "test.server.net", "overworld").unwrap();

        let chunk = ParsedChunk {
            x: 10,
            z: -25,
            min_section_y: -4,
            inhabited_time: 12345,
            last_update: 67890,
            sections: Vec::new(),
            heightmap_surface: Some([100; 256]),
            heightmap_ocean: None,
        };

        cache.put(Arc::new(chunk.clone())).unwrap();
        assert!(cache.contains(10, -25));
        let loaded = cache.get(10, -25).unwrap();
        assert_eq!(loaded.x, 10);
        assert_eq!(loaded.z, -25);
        assert_eq!(loaded.heightmap_surface.unwrap()[0], 100);
    }
}
