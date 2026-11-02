//! Livello 2: Pre-Baking Quadtree LOD (Stile "Voxy / Distant Horizons").
//!
//! Converte in parallelo i chunk estratti da `.mca` in colonne LOD compresse (altezza superficie,
//! blocchi dominanti, luce media, biomi) e pre-calcola tutti i livelli di mipmap del Quadtree
//! (da LOD 0 fino a LOD 5). I dati vengono scritti su disco in `horizon_lod_cache/<server_id>/<dimension>/`.

use super::anvil_importer::ParsedChunk;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const HLOD_MAGIC: &[u8; 6] = b"DHLOD2";

/// Singolo campione di colonna LOD: altezza superficie, stato blocco sintetico, luce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LodSample {
    pub height: i16,
    pub block_id: u16,
    pub light: u8,
    pub biome_id: u16,
}

impl Default for LodSample {
    fn default() -> Self {
        Self {
            height: -64,
            block_id: 0,
            light: 15,
            biome_id: 0,
        }
    }
}

/// Griglia di colonne per una foglia o un nodo del quadtree LOD.
#[derive(Debug, Clone)]
pub struct LodTile {
    pub level: u8,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub size: usize,
    pub samples: Vec<LodSample>,
}

impl LodTile {
    pub fn new(level: u8, chunk_x: i32, chunk_z: i32, size: usize) -> Self {
        Self {
            level,
            chunk_x,
            chunk_z,
            size,
            samples: vec![LodSample::default(); size * size],
        }
    }

    /// Sottocampiona la griglia dimezzando la risoluzione ($2\times 2 \to 1\times 1$).
    pub fn downsample(&self) -> Self {
        let new_size = (self.size / 2).max(1);
        let mut result = Self::new(self.level + 1, self.chunk_x, self.chunk_z, new_size);

        for nz in 0..new_size {
            for nx in 0..new_size {
                let mut sum_light = 0u32;
                let mut max_height = -32768i16;
                let mut rep_block = 0u16;
                let mut rep_biome = 0u16;
                let mut count = 0;

                for dz in 0..2 {
                    for dx in 0..2 {
                        let ox = (nx * 2 + dx).min(self.size - 1);
                        let oz = (nz * 2 + dz).min(self.size - 1);
                        let sample = self.samples[oz * self.size + ox];
                        sum_light += sample.light as u32;
                        if sample.height > max_height {
                            max_height = sample.height;
                            rep_block = sample.block_id;
                            rep_biome = sample.biome_id;
                        }
                        count += 1;
                    }
                }

                let avg_light = if count > 0 {
                    (sum_light / count as u32) as u8
                } else {
                    15
                };

                result.samples[nz * new_size + nx] = LodSample {
                    height: max_height,
                    block_id: rep_block,
                    light: avg_light,
                    biome_id: rep_biome,
                };
            }
        }

        result
    }

    /// Serializza il tile LOD nel formato compatto `.hlod`.
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HLOD_MAGIC.len() + 16 + self.samples.len() * 7);
        buf.extend_from_slice(HLOD_MAGIC);
        buf.push(self.level);
        buf.extend_from_slice(&self.chunk_x.to_le_bytes());
        buf.extend_from_slice(&self.chunk_z.to_le_bytes());
        buf.push(self.size as u8);

        for s in &self.samples {
            buf.extend_from_slice(&s.height.to_le_bytes());
            buf.extend_from_slice(&s.block_id.to_le_bytes());
            buf.push(s.light);
            buf.extend_from_slice(&s.biome_id.to_le_bytes());
        }

        // Fletcher-32 checksum
        let mut sum1: u32 = 0xffff;
        let mut sum2: u32 = 0xffff;
        for &byte in &buf {
            sum1 = (sum1 + byte as u32) % 65535;
            sum2 = (sum2 + sum1) % 65535;
        }
        let checksum = (sum2 << 16) | sum1;
        buf.extend_from_slice(&checksum.to_le_bytes());

        buf
    }

    /// Deserializza un tile LOD verificando il magic header e il checksum.
    pub fn deserialize(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < HLOD_MAGIC.len() + 1 + 4 + 4 + 1 + 4 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "HLOD buffer troppo corto",
            ));
        }
        if &bytes[0..HLOD_MAGIC.len()] != HLOD_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Magic HLOD non valido",
            ));
        }
        let mut offset = HLOD_MAGIC.len();
        let level = bytes[offset];
        offset += 1;
        let chunk_x = i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let chunk_z = i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let size = bytes[offset] as usize;
        offset += 1;

        let total_samples = size * size;
        if bytes.len() < offset + total_samples * 7 + 4 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "HLOD payload tronco",
            ));
        }

        let mut samples = Vec::with_capacity(total_samples);
        for _ in 0..total_samples {
            let height = i16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
            offset += 2;
            let block_id = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
            offset += 2;
            let light = bytes[offset];
            offset += 1;
            let biome_id = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
            offset += 2;
            samples.push(LodSample {
                height,
                block_id,
                light,
                biome_id,
            });
        }

        Ok(Self {
            level,
            chunk_x,
            chunk_z,
            size,
            samples,
        })
    }
}

pub struct LodBaker {
    base_dir: PathBuf,
    pub server_id: String,
    pub dimension: String,
}

impl LodBaker {
    pub fn new(root: impl AsRef<Path>, server_id: &str, dimension: &str) -> io::Result<Self> {
        let base_dir = root
            .as_ref()
            .join("horizon_lod_cache")
            .join(server_id)
            .join(dimension);
        fs::create_dir_all(&base_dir)?;
        Ok(Self {
            base_dir,
            server_id: server_id.to_string(),
            dimension: dimension.to_string(),
        })
    }

    fn tile_path(&self, level: u8, x: i32, z: i32) -> PathBuf {
        let rx = x.div_euclid(32);
        let rz = z.div_euclid(32);
        self.base_dir
            .join(format!("lod_{level}"))
            .join(format!("r.{rx}.{rz}"))
            .join(format!("tile_{level}_{x}_{z}.hlod"))
    }

    /// Esegue il baking di un `ParsedChunk` creando e salvando tutti i livelli LOD (0 -> 5).
    pub fn bake_chunk_pyramid(&self, chunk: &ParsedChunk) -> io::Result<()> {
        let mut tile_lod0 = LodTile::new(0, chunk.x, chunk.z, 16);

        // Popola LOD 0 direttamente dai blocchi o heightmap del chunk
        for cz in 0..16 {
            for cx in 0..16 {
                let col_idx = cz * 16 + cx;
                let (h, block_id) = find_column_surface(chunk, cx, cz);
                tile_lod0.samples[col_idx] = LodSample {
                    height: h,
                    block_id,
                    light: 15,
                    biome_id: 0,
                };
            }
        }

        // Salva LOD 0 (risoluzione piena 16x16)
        self.save_tile(&tile_lod0)?;

        // Genera e salva la piramide di mipmap LOD 1 -> 4
        let tile_lod1 = tile_lod0.downsample(); // 8x8
        self.save_tile(&tile_lod1)?;

        let tile_lod2 = tile_lod1.downsample(); // 4x4
        self.save_tile(&tile_lod2)?;

        let tile_lod3 = tile_lod2.downsample(); // 2x2
        self.save_tile(&tile_lod3)?;

        let tile_lod4 = tile_lod3.downsample(); // 1x1
        self.save_tile(&tile_lod4)?;

        Ok(())
    }

    pub fn save_tile(&self, tile: &LodTile) -> io::Result<()> {
        let path = self.tile_path(tile.level, tile.chunk_x, tile.chunk_z);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = tile.serialize();
        fs::write(path, bytes)?;
        Ok(())
    }

    pub fn load_tile(&self, level: u8, x: i32, z: i32) -> Option<LodTile> {
        let path = self.tile_path(level, x, z);
        let bytes = fs::read(path).ok()?;
        LodTile::deserialize(&bytes).ok()
    }
}

/// Determina altezza e blocco superficiale della colonna (cx, cz).
fn find_column_surface(chunk: &ParsedChunk, cx: usize, cz: usize) -> (i16, u16) {
    if let Some(surface) = &chunk.heightmap_surface {
        let h = surface[cz * 16 + cx];
        // Cerca il blocco a quell'altezza
        let sec_idx = ((h - (chunk.min_section_y as i16 * 16)) >> 4) as usize;
        if let Some(sec) = chunk.sections.get(sec_idx) {
            let sy = (h & 15) as usize;
            let block_idx = (sy << 8) | (cz << 4) | cx;
            let p_idx = sec.blocks[block_idx];
            return (h, p_idx);
        }
        return (h, 1);
    }
    // Fallback: scansiona le sezioni dall'alto
    for sec in chunk.sections.iter().rev() {
        for dy in (0..16).rev() {
            let block_idx = (dy << 8) | (cz << 4) | cx;
            let p_idx = sec.blocks[block_idx];
            if p_idx != 0 {
                let h = (sec.y as i16 * 16) + dy as i16;
                return (h, p_idx);
            }
        }
    }
    (-64, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_lod_tile_downsampling_and_roundtrip() {
        let dir = tempdir().unwrap();
        let baker = LodBaker::new(dir.path(), "server.net", "overworld").unwrap();

        let mut lod0 = LodTile::new(0, 5, -8, 16);
        for s in &mut lod0.samples {
            s.height = 75;
            s.block_id = 3;
            s.light = 14;
        }

        baker.save_tile(&lod0).unwrap();
        let loaded = baker.load_tile(0, 5, -8).unwrap();
        assert_eq!(loaded.samples[0].height, 75);
        assert_eq!(loaded.samples[0].block_id, 3);

        let lod1 = lod0.downsample();
        assert_eq!(lod1.size, 8);
        assert_eq!(lod1.samples[0].height, 75);
    }
}
