//! Modulo Parser Anvil `.mca` Nativo (Memory-Mapped & Parallel).
//!
//! Implementa la lettura ad alte prestazioni di file di regione Anvil `.mca`,
//! estrazione dei chunk con `memmap2`, decompressione multi-thread con `rayon`,
//! `flate2` e `lz4_flex`, e deserializzazione NBT veloce.

use flate2::read::{GzDecoder, ZlibDecoder};
use memmap2::Mmap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub const SECTOR_BYTES: usize = 4096;
pub const HEADER_SECTORS: usize = 2;
pub const CHUNKS_PER_REGION: usize = 1024; // 32 x 32

/// Schemi di compressione standard per chunk Anvil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionScheme {
    Gzip = 1,
    Zlib = 2,
    Uncompressed = 3,
    Lz4 = 4,
}

impl TryFrom<u8> for CompressionScheme {
    type Error = io::Error;
    fn try_from(val: u8) -> io::Result<Self> {
        match val {
            1 => Ok(Self::Gzip),
            2 => Ok(Self::Zlib),
            3 => Ok(Self::Uncompressed),
            4 => Ok(Self::Lz4),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Schema compressione sconosciuto: {val}"),
            )),
        }
    }
}

// -------------------------------------------------------------
// Strutture NBT Minecraft 1.18+ / 26.x
// -------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockStateNbt {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Properties")]
    pub properties: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BlockStatesNbt {
    pub palette: Option<Vec<BlockStateNbt>>,
    pub data: Option<fastnbt::LongArray>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BiomesNbt {
    pub palette: Option<Vec<String>>,
    pub data: Option<fastnbt::LongArray>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionNbt {
    #[serde(rename = "Y")]
    pub y: i8,
    pub block_states: Option<BlockStatesNbt>,
    pub biomes: Option<BiomesNbt>,
    #[serde(rename = "BlockLight")]
    pub block_light: Option<fastnbt::ByteArray>,
    #[serde(rename = "SkyLight")]
    pub sky_light: Option<fastnbt::ByteArray>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkNbt {
    #[serde(rename = "DataVersion")]
    pub data_version: Option<i32>,
    #[serde(rename = "xPos")]
    pub x_pos: i32,
    #[serde(rename = "yPos")]
    pub y_pos: Option<i32>,
    #[serde(rename = "zPos")]
    pub z_pos: i32,
    #[serde(rename = "Status")]
    pub status: Option<String>,
    #[serde(rename = "InhabitedTime")]
    pub inhabited_time: Option<i64>,
    #[serde(rename = "LastUpdate")]
    pub last_update: Option<i64>,
    pub sections: Option<Vec<SectionNbt>>,
    #[serde(rename = "Heightmaps")]
    pub heightmaps: Option<HashMap<String, fastnbt::LongArray>>,
}

/// Chunk deserializzato e pronto per l'elaborazione.
#[derive(Debug, Clone)]
pub struct ParsedChunk {
    pub x: i32,
    pub z: i32,
    pub min_section_y: i8,
    pub inhabited_time: i64,
    pub last_update: i64,
    pub sections: Vec<UnpackedSection>,
    pub heightmap_surface: Option<[i16; 256]>,
    pub heightmap_ocean: Option<[i16; 256]>,
}

#[derive(Debug, Clone)]
pub struct UnpackedSection {
    pub y: i8,
    pub palette: Vec<BlockStateNbt>,
    pub blocks: [u16; 4096],
    pub biome_palette: Vec<String>,
    pub biomes: [u16; 64],
    pub block_light: Option<Vec<u8>>,
    pub sky_light: Option<Vec<u8>>,
}

// -------------------------------------------------------------
// Unpacking Bit-Packed Minecraft
// -------------------------------------------------------------

/// Decomprime i 4096 indici di blocchi salvati in `block_states.data`.
///
/// In Minecraft 1.16+ / 1.18+ / 26.x gli indici della palette NON attraversano
/// i confini delle word a 64 bit (`entries_per_word = 64 / bits`).
pub fn unpack_block_states(palette: &[BlockStateNbt], data: Option<&[i64]>) -> [u16; 4096] {
    let mut out = [0u16; 4096];
    if palette.is_empty() {
        return out;
    }
    if palette.len() == 1 || data.is_none() {
        return out; // Tutti i blocchi usano palette[0]
    }
    let data = data.unwrap();
    let num_bits = usize::max(
        4,
        (usize::BITS - (palette.len() - 1).leading_zeros()) as usize,
    );
    let entries_per_word = 64 / num_bits;
    if entries_per_word == 0 {
        return out;
    }
    let mask = (1u64 << num_bits) - 1;
    for i in 0..4096 {
        let word_idx = i / entries_per_word;
        if word_idx >= data.len() {
            break;
        }
        let bit_offset = (i % entries_per_word) * num_bits;
        let palette_idx = ((data[word_idx] as u64 >> bit_offset) & mask) as usize;
        out[i] = if palette_idx < palette.len() {
            palette_idx as u16
        } else {
            0
        };
    }
    out
}

/// Decomprime i 64 indici di biomi di una sezione (4x4x4 celle).
pub fn unpack_biomes(palette: &[String], data: Option<&[i64]>) -> [u16; 64] {
    let mut out = [0u16; 64];
    if palette.is_empty() {
        return out;
    }
    if palette.len() == 1 || data.is_none() {
        return out;
    }
    let data = data.unwrap();
    let num_bits = usize::max(
        1,
        (usize::BITS - (palette.len() - 1).leading_zeros()) as usize,
    );
    let entries_per_word = 64 / num_bits;
    if entries_per_word == 0 {
        return out;
    }
    let mask = (1u64 << num_bits) - 1;
    for i in 0..64 {
        let word_idx = i / entries_per_word;
        if word_idx >= data.len() {
            break;
        }
        let bit_offset = (i % entries_per_word) * num_bits;
        let palette_idx = ((data[word_idx] as u64 >> bit_offset) & mask) as usize;
        out[i] = if palette_idx < palette.len() {
            palette_idx as u16
        } else {
            0
        };
    }
    out
}

/// Decomprime una heightmap di 256 colonne (9 bit ciascuna) in coordinate assolute.
pub fn unpack_heightmap(data: &[i64], min_y: i32) -> [i16; 256] {
    let mut heights = [min_y as i16; 256];
    let bits = 9;
    let entries_per_word = 64 / bits; // 7 entries per 64-bit word
    let mask = (1u64 << bits) - 1;
    for i in 0..256 {
        let word_idx = i / entries_per_word;
        if word_idx >= data.len() {
            break;
        }
        let bit_offset = (i % entries_per_word) * bits;
        let val = ((data[word_idx] as u64 >> bit_offset) & mask) as i32;
        heights[i] = (min_y + val) as i16;
    }
    heights
}

// -------------------------------------------------------------
// Parser Region Anvil
// -------------------------------------------------------------

pub struct MmapAnvilRegion {
    pub path: PathBuf,
    pub region_x: i32,
    pub region_z: i32,
    mmap: Mmap,
}

impl MmapAnvilRegion {
    /// Mappa in memoria un file `.mca` ed estrae le coordinate dal nome `r.X.Z.mca`.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let (region_x, region_z) = parse_region_coords(&path)?;
        let file = File::open(&path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        if mmap.len() < SECTOR_BYTES * HEADER_SECTORS {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("File Anvil troppo corto ({} byte)", mmap.len()),
            ));
        }
        Ok(Self {
            path,
            region_x,
            region_z,
            mmap,
        })
    }

    /// Ritorna la tabella degli offset (in settori da 4096 byte) e la dimensione di ciascun chunk.
    pub fn chunk_offsets(&self) -> [(usize, usize); CHUNKS_PER_REGION] {
        let mut result = [(0, 0); CHUNKS_PER_REGION];
        for i in 0..CHUNKS_PER_REGION {
            let offset_bytes = &self.mmap[i * 4..i * 4 + 4];
            let sector_offset = ((offset_bytes[0] as usize) << 16)
                | ((offset_bytes[1] as usize) << 8)
                | (offset_bytes[2] as usize);
            let sector_count = offset_bytes[3] as usize;
            result[i] = (sector_offset, sector_count);
        }
        result
    }

    /// Decomprime ed estrae il payload NBT grezzo per il chunk allo slot locale (0..1024).
    pub fn extract_chunk_raw(&self, slot: usize) -> io::Result<Option<Vec<u8>>> {
        let (sector_offset, sector_count) = self.chunk_offsets()[slot];
        if sector_offset == 0 || sector_count == 0 {
            return Ok(None);
        }
        let byte_offset = sector_offset * SECTOR_BYTES;
        if byte_offset + 5 > self.mmap.len() {
            return Ok(None);
        }
        let length = u32::from_be_bytes([
            self.mmap[byte_offset],
            self.mmap[byte_offset + 1],
            self.mmap[byte_offset + 2],
            self.mmap[byte_offset + 3],
        ]) as usize;
        if length == 0 || byte_offset + 4 + length > self.mmap.len() {
            return Ok(None);
        }
        let scheme = CompressionScheme::try_from(self.mmap[byte_offset + 4])?;
        let payload = &self.mmap[byte_offset + 5..byte_offset + 4 + length];

        let decompressed = match scheme {
            CompressionScheme::Uncompressed => payload.to_vec(),
            CompressionScheme::Gzip => {
                let mut decoder = GzDecoder::new(payload);
                let mut buf = Vec::new();
                decoder.read_to_end(&mut buf)?;
                buf
            }
            CompressionScheme::Zlib => {
                let mut decoder = ZlibDecoder::new(payload);
                let mut buf = Vec::new();
                decoder.read_to_end(&mut buf)?;
                buf
            }
            CompressionScheme::Lz4 => lz4_flex::decompress_size_prepended(payload)
                .or_else(|_| lz4_flex::block::decompress(payload, 1024 * 1024 * 4))
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?,
        };

        Ok(Some(decompressed))
    }

    /// Estrae e parsa tutti i chunk validi della regione in parallelo con Rayon.
    pub fn parse_all_chunks_parallel(&self) -> Vec<ParsedChunk> {
        (0..CHUNKS_PER_REGION)
            .into_par_iter()
            .filter_map(|slot| {
                let raw = self.extract_chunk_raw(slot).ok()??;
                let chunk_nbt: ChunkNbt = fastnbt::from_bytes(&raw).ok()?;
                Some(convert_chunk_nbt(chunk_nbt))
            })
            .collect()
    }
}

/// Converte il DTO NBT nella struttura ottimizzata `ParsedChunk`.
pub fn convert_chunk_nbt(nbt: ChunkNbt) -> ParsedChunk {
    let min_y = nbt.y_pos.unwrap_or(-4) * 16;
    let mut unpacked_sections = Vec::new();

    if let Some(sections) = nbt.sections {
        for sec in sections {
            let palette = sec
                .block_states
                .as_ref()
                .and_then(|bs| bs.palette.clone())
                .unwrap_or_default();
            let data = sec
                .block_states
                .as_ref()
                .and_then(|bs| bs.data.as_ref())
                .map(|d| &d[..]);
            let blocks = unpack_block_states(&palette, data);

            let biome_palette = sec
                .biomes
                .as_ref()
                .and_then(|b| b.palette.clone())
                .unwrap_or_default();
            let biome_data = sec
                .biomes
                .as_ref()
                .and_then(|b| b.data.as_ref())
                .map(|d| &d[..]);
            let biomes = unpack_biomes(&biome_palette, biome_data);

            let block_light = sec
                .block_light
                .map(|b| b.into_inner().into_iter().map(|x| x as u8).collect());
            let sky_light = sec
                .sky_light
                .map(|b| b.into_inner().into_iter().map(|x| x as u8).collect());

            unpacked_sections.push(UnpackedSection {
                y: sec.y,
                palette,
                blocks,
                biome_palette,
                biomes,
                block_light,
                sky_light,
            });
        }
    }

    let heightmap_surface = nbt
        .heightmaps
        .as_ref()
        .and_then(|hm| hm.get("WORLD_SURFACE"))
        .map(|data| unpack_heightmap(&data[..], min_y));

    let heightmap_ocean = nbt
        .heightmaps
        .as_ref()
        .and_then(|hm| hm.get("OCEAN_FLOOR"))
        .map(|data| unpack_heightmap(&data[..], min_y));

    ParsedChunk {
        x: nbt.x_pos,
        z: nbt.z_pos,
        min_section_y: nbt.y_pos.unwrap_or(-4) as i8,
        inhabited_time: nbt.inhabited_time.unwrap_or(0),
        last_update: nbt.last_update.unwrap_or(0),
        sections: unpacked_sections,
        heightmap_surface,
        heightmap_ocean,
    }
}

/// Estrae le coordinate (rx, rz) dal nome file standard Anvil `r.X.Z.mca`.
pub fn parse_region_coords(path: &Path) -> io::Result<(i32, i32)> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Nome file invalido"))?;
    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() != 4 || parts[0] != "r" || parts[3] != "mca" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Formato nome file regione non valido (atteso r.X.Z.mca): {name}"),
        ));
    }
    let rx = parts[1]
        .parse::<i32>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    let rz = parts[2]
        .parse::<i32>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    Ok((rx, rz))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unpack_single_palette_block() {
        let palette = vec![BlockStateNbt {
            name: "minecraft:stone".to_string(),
            properties: None,
        }];
        let blocks = unpack_block_states(&palette, None);
        assert_eq!(blocks[0], 0);
        assert_eq!(blocks[4095], 0);
    }

    #[test]
    fn test_unpack_two_states_bit_packing() {
        let palette = vec![
            BlockStateNbt {
                name: "minecraft:air".to_string(),
                properties: None,
            },
            BlockStateNbt {
                name: "minecraft:dirt".to_string(),
                properties: None,
            },
        ];
        // 4 bit per entry, 16 entries per 64-bit word
        // Word 0: alternanza 0, 1, 0, 1... (0x1010101010101010)
        let word: i64 = 0x1010101010101010u64 as i64;
        let data = vec![word; 256];
        let blocks = unpack_block_states(&palette, Some(&data));
        assert_eq!(blocks[0], 0);
        assert_eq!(blocks[1], 1);
        assert_eq!(blocks[2], 0);
        assert_eq!(blocks[3], 1);
    }

    #[test]
    fn test_parse_region_coords() {
        let path = Path::new("dimensions/minecraft/overworld/region/r.-2.5.mca");
        let (rx, rz) = parse_region_coords(path).unwrap();
        assert_eq!(rx, -2);
        assert_eq!(rz, 5);
    }

    #[test]
    fn test_parse_real_mca_file() {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let path = std::path::PathBuf::from(appdata)
            .join("horizon")
            .join("saves")
            .join("prova1")
            .join("dimensions")
            .join("minecraft")
            .join("overworld")
            .join("region")
            .join("r.-1.-1.mca");
        if path.exists() {
            let region = MmapAnvilRegion::open(&path).expect("Failed to open real mca");
            assert_eq!(region.region_x, -1);
            assert_eq!(region.region_z, -1);
            let start = std::time::Instant::now();
            let chunks = region.parse_all_chunks_parallel();
            let elapsed = start.elapsed();
            assert!(!chunks.is_empty(), "Expected chunks in r.-1.-1.mca");
            println!(
                "SUCCESS: Parsed {} chunks from r.-1.-1.mca in {:.2?} ({:.0} chunks/sec)",
                chunks.len(),
                elapsed,
                chunks.len() as f64 / elapsed.as_secs_f64()
            );
            assert_eq!(chunks[0].x, -32);
            assert_eq!(chunks[0].z, -32);
        }
    }
}
