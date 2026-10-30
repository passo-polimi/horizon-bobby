//! # Horizon Mod Harness
//!
//! Modulo di verifica e differential testing Java <-> Rust (Fase 3).
//! Implementa i test differenziali sui vettori golden generati dalla JVM (T3.1, T3.2).

use serde::{Deserialize, Serialize};

// ========================================================
// Funzioni pure di Nvidium (da NvidiumCompactChunkVertex.java)
// ========================================================
pub mod nvidium_pure {
    const POSITION_MAX_VALUE: f32 = 65536.0;
    const TEXTURE_MAX_VALUE: f32 = 32768.0;
    const MODEL_ORIGIN: f32 = 8.0;
    const MODEL_RANGE: f32 = 32.0;
    const MODEL_SCALE_INV: f32 = POSITION_MAX_VALUE / MODEL_RANGE;

    // PORT: NvidiumCompactChunkVertex#encodePosition(float)
    #[inline]
    pub fn encode_position(v: f32) -> i32 {
        ((MODEL_ORIGIN + v) * MODEL_SCALE_INV) as i32
    }

    // PORT: NvidiumCompactChunkVertex#compactLight(int)
    #[inline]
    pub fn compact_light(light: i32) -> i32 {
        let u_light = light as u32;
        let sky = ((u_light >> 16) & 0xFF).clamp(8, 248);
        let block = (u_light & 0xFF).clamp(8, 248);
        (block | (sky << 8)) as i32
    }

    // PORT: NvidiumCompactChunkVertex#encodeTexture(float, float)
    #[inline]
    pub fn encode_texture(u: f32, v: f32) -> i32 {
        // Java Math.round arrotonda verso +infinito a metà: floor(val + 0.5f)
        let round_u = (u * TEXTURE_MAX_VALUE + 0.5).floor() as i64;
        let round_v = (v * TEXTURE_MAX_VALUE + 0.5).floor() as i64;
        ((round_u & 0xFFFF) as i32) | (((round_v & 0xFFFF) as i32) << 16)
    }
}

// ========================================================
// Funzioni pure di Distant Horizons (da DhSectionPos.java)
// ========================================================
pub mod dh_pure {
    pub const DETAIL_LEVEL_WIDTH: u32 = 8;
    pub const X_POS_WIDTH: u32 = 28;
    pub const Z_POS_WIDTH: u32 = 28;
    pub const X_POS_MISSING_WIDTH: u32 = 32 - 28;
    pub const Z_POS_MISSING_WIDTH: u32 = 32 - 28;

    pub const DETAIL_LEVEL_OFFSET: u32 = 0;
    pub const POS_X_OFFSET: u32 = DETAIL_LEVEL_OFFSET + DETAIL_LEVEL_WIDTH;
    pub const POS_Z_OFFSET: u32 = POS_X_OFFSET + X_POS_WIDTH;

    pub const DETAIL_LEVEL_MASK: u64 = 0x7F; // Byte.MAX_VALUE
    pub const POS_X_MASK: u64 = (1 << X_POS_WIDTH) - 1;
    pub const POS_Z_MASK: u64 = (1 << Z_POS_WIDTH) - 1;

    // PORT: DhSectionPos#encode(byte, int, int)
    #[inline]
    pub fn section_pos_encode(detail_level: u8, x: i32, z: i32) -> i64 {
        let mut data: u64 = 0;
        data |= (detail_level as u64) & DETAIL_LEVEL_MASK;
        data |= ((x as u64) & POS_X_MASK) << POS_X_OFFSET;
        data |= ((z as u64) & POS_Z_MASK) << POS_Z_OFFSET;
        data as i64
    }

    // PORT: DhSectionPos#getDetailLevel(long)
    #[inline]
    pub fn section_pos_get_level(pos: i64) -> u8 {
        let u_pos = pos as u64;
        ((u_pos >> DETAIL_LEVEL_OFFSET) & DETAIL_LEVEL_MASK) as u8
    }

    // PORT: DhSectionPos#getX(long)
    #[inline]
    pub fn section_pos_get_x(pos: i64) -> i32 {
        let u_pos = pos as u64;
        let x = ((u_pos >> POS_X_OFFSET) & POS_X_MASK) as i32;
        (x << X_POS_MISSING_WIDTH) >> X_POS_MISSING_WIDTH
    }

    // PORT: DhSectionPos#getZ(long)
    #[inline]
    pub fn section_pos_get_z(pos: i64) -> i32 {
        let u_pos = pos as u64;
        let z = ((u_pos >> POS_Z_OFFSET) & POS_Z_MASK) as i32;
        (z << Z_POS_MISSING_WIDTH) >> Z_POS_MISSING_WIDTH
    }

    // PORT: DhSectionPos#convertToDetailLevel(long, byte)
    #[inline]
    pub fn section_pos_convert_detail_level(pos: i64, new_detail_level: u8) -> i64 {
        let current_level = section_pos_get_level(pos);
        let mut x = section_pos_get_x(pos);
        let mut z = section_pos_get_z(pos);

        if new_detail_level >= current_level {
            let shift = new_detail_level - current_level;
            let divisor = 1i32 << shift;
            x = x.div_euclid(divisor);
            z = z.div_euclid(divisor);
        } else {
            let shift = current_level - new_detail_level;
            let multiplier = 1i32 << shift;
            x = x.wrapping_mul(multiplier);
            z = z.wrapping_mul(multiplier);
        }

        section_pos_encode(new_detail_level, x, z)
    }
}

// ========================================================
// Strutture di deserializzazione per i vettori di test JSON
// ========================================================
#[derive(Debug, Deserialize)]
pub struct PositionTestVector {
    pub function: String,
    pub cases: Vec<PositionTestCase>,
}

#[derive(Debug, Deserialize)]
pub struct PositionTestCase {
    pub input: PositionInput,
    pub expected: i32,
}

#[derive(Debug, Deserialize)]
pub struct PositionInput {
    pub v: f32,
}

#[derive(Debug, Deserialize)]
pub struct LightTestVector {
    pub function: String,
    pub cases: Vec<LightTestCase>,
}

#[derive(Debug, Deserialize)]
pub struct LightTestCase {
    pub input: LightInput,
    pub expected: i32,
}

#[derive(Debug, Deserialize)]
pub struct LightInput {
    pub light: i32,
}

#[derive(Debug, Deserialize)]
pub struct TextureTestVector {
    pub function: String,
    pub cases: Vec<TextureTestCase>,
}

#[derive(Debug, Deserialize)]
pub struct TextureTestCase {
    pub input: TextureInput,
    pub expected: i32,
}

#[derive(Debug, Deserialize)]
pub struct TextureInput {
    pub u: f32,
    pub v: f32,
}

#[derive(Debug, Deserialize)]
pub struct DhSectionPosEncodeVector {
    pub function: String,
    pub cases: Vec<DhSectionPosEncodeCase>,
}

#[derive(Debug, Deserialize)]
pub struct DhSectionPosEncodeCase {
    pub input: DhPosInput,
    pub packed: i64,
    pub decoded: DhPosDecoded,
}

#[derive(Debug, Deserialize)]
pub struct DhPosInput {
    pub level: u8,
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Deserialize)]
pub struct DhPosDecoded {
    pub level: u8,
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Deserialize)]
pub struct DhSectionPosConvertVector {
    pub function: String,
    pub cases: Vec<DhSectionPosConvertCase>,
}

#[derive(Debug, Deserialize)]
pub struct DhSectionPosConvertCase {
    pub input: DhConvertInput,
    pub expected_packed: i64,
    pub expected: DhPosDecoded,
}

#[derive(Debug, Deserialize)]
pub struct DhConvertInput {
    pub from_level: u8,
    pub to_level: u8,
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Deserialize)]
pub struct VarIntVector {
    pub function: String,
    pub cases: Vec<VarIntCase>,
}

#[derive(Debug, Deserialize)]
pub struct VarIntCase {
    pub value: i32,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NvidiumGoldenConfig {
    pub region_keep_distance: u32,
    pub async_occlusion_culling: bool,
    pub translucent_sorting: bool,
    pub max_draw_distance: u32,
    pub enable_hiz_culling: bool,
}
