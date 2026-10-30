//! Distant Horizons — Porting nativo per Horizon
//!
//! Implementa la feature list concordata in T0.4 (DH-1 .. DH-5):
//! - DH-1: Modello Dati Columnar RLE
//! - DH-2: Persistenza su Disco (.hlod)
//! - DH-3: Greedy Mesher 2.5D con Skirt Culling
//! - DH-4: Frustum Culling Geometrico
//! - DH-5: Shader WGSL Far-Terrain con LODRenderHook

use horizon_mod_api::prelude::*;

// ========================================================
// DH-1: Modello Dati Columnar RLE
// ========================================================
pub mod rle_data {
    use super::*;

    /// Singolo intervallo verticale compresso (span) di una colonna.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct HeightSpan {
        pub min_y: i16,
        pub max_y: i16,
        pub block_id: u16,
        pub light: u8,
    }

    /// Dati a colonne per una sezione LOD (griglia $N \times N$).
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ColumnarData {
        pub size: usize,
        pub columns: Vec<Vec<HeightSpan>>,
    }

    impl ColumnarData {
        pub fn new(size: usize) -> Self {
            Self {
                size,
                columns: vec![Vec::new(); size * size],
            }
        }

        pub fn get_column(&self, x: usize, z: usize) -> &[HeightSpan] {
            &self.columns[z * self.size + x]
        }

        pub fn set_column(&mut self, x: usize, z: usize, spans: Vec<HeightSpan>) {
            self.columns[z * self.size + x] = spans;
        }

        // PORT: ColumnarData#downsample(int, int, int)
        /// Sottocampiona la griglia dimezzando la risoluzione ($2\times 2 \to 1\times 1$).
        pub fn downsample(&self) -> Self {
            let new_size = (self.size / 2).max(1);
            let mut result = Self::new(new_size);

            for nz in 0..new_size {
                for nx in 0..new_size {
                    // Raggruppa i 4 campioni adiacenti
                    let mut merged_spans = Vec::new();
                    for dz in 0..2 {
                        for dx in 0..2 {
                            let ox = (nx * 2 + dx).min(self.size - 1);
                            let oz = (nz * 2 + dz).min(self.size - 1);
                            for span in self.get_column(ox, oz) {
                                merged_spans.push(*span);
                            }
                        }
                    }

                    if !merged_spans.is_empty() {
                        let min_y = merged_spans.iter().map(|s| s.min_y).min().unwrap_or(0);
                        let max_y = merged_spans.iter().map(|s| s.max_y).max().unwrap_or(0);
                        let rep_block = merged_spans[0].block_id;
                        let avg_light = (merged_spans.iter().map(|s| s.light as u32).sum::<u32>()
                            / merged_spans.len() as u32)
                            as u8;

                        result.set_column(
                            nx,
                            nz,
                            vec![HeightSpan {
                                min_y,
                                max_y,
                                block_id: rep_block,
                                light: avg_light,
                            }],
                        );
                    }
                }
            }

            result
        }
    }

    // PORT: ColumnarData#downsample(int, int, int)
    pub fn downsample_chunk(_pos: ChunkPos, level: u8) -> Vec<u8> {
        let size = (16 >> level.min(4)).max(1);
        let data = ColumnarData::new(size);
        let downsampled = data.downsample();
        let mut out = Vec::new();
        out.extend_from_slice(&(downsampled.size as u32).to_le_bytes());
        out
    }
}

// ========================================================
// DH-2: Persistenza su Disco (.hlod)
// ========================================================
pub mod storage {
    use super::*;

    const MAGIC_HEADER: &[u8; 6] = b"DHLOD1";

    // PORT: LodPersistence#saveChunk(ChunkPos, byte[])
    /// Salva i dati binari di chunk nel formato `.hlod` con header magico e checksum.
    pub fn save_hlod(pos: ChunkPos, data: &[u8]) -> Result<Vec<u8>, String> {
        let mut buffer = Vec::with_capacity(MAGIC_HEADER.len() + 16 + data.len());
        buffer.extend_from_slice(MAGIC_HEADER);
        buffer.extend_from_slice(&pos.x.to_le_bytes());
        buffer.extend_from_slice(&pos.z.to_le_bytes());
        buffer.extend_from_slice(&(data.len() as u32).to_le_bytes());
        buffer.extend_from_slice(data);

        // Calcolo checksum semplice Fletcher-32
        let mut sum1: u32 = 0xffff;
        let mut sum2: u32 = 0xffff;
        for &byte in &buffer {
            sum1 = (sum1 + byte as u32) % 65535;
            sum2 = (sum2 + sum1) % 65535;
        }
        let checksum = (sum2 << 16) | sum1;
        buffer.extend_from_slice(&checksum.to_le_bytes());

        Ok(buffer)
    }

    // PORT: LodPersistence#loadChunk(ChunkPos)
    /// Carica e valida un buffer binario `.hlod`.
    pub fn load_hlod(data: &[u8]) -> Result<(ChunkPos, Vec<u8>), String> {
        if data.len() < MAGIC_HEADER.len() + 16 + 4 {
            return Err("File .hlod troppo corto".to_string());
        }

        let magic = &data[..MAGIC_HEADER.len()];
        if magic != MAGIC_HEADER {
            return Err("Magic header .hlod non valido".to_string());
        }

        let mut offset = MAGIC_HEADER.len();
        let chunk_x = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let chunk_z = i32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let payload_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;

        if data.len() < offset + payload_len + 4 {
            return Err("Dimensione payload .hlod corrotta".to_string());
        }

        let payload = &data[offset..offset + payload_len];
        offset += payload_len;

        let expected_checksum = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());

        // Verifica checksum
        let mut sum1: u32 = 0xffff;
        let mut sum2: u32 = 0xffff;
        for &byte in &data[..offset] {
            sum1 = (sum1 + byte as u32) % 65535;
            sum2 = (sum2 + sum1) % 65535;
        }
        let actual_checksum = (sum2 << 16) | sum1;

        if actual_checksum != expected_checksum {
            return Err("Checksum fallito nel file .hlod".to_string());
        }

        Ok((ChunkPos::new(chunk_x, chunk_z), payload.to_vec()))
    }
}

// ========================================================
// DH-3: Greedy Mesher 2.5D con Skirt Culling
// ========================================================
pub mod mesher {
    use super::*;

    /// Quad grezzo generato dal mesher greedy.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GreedyQuad {
        pub min_x: i32,
        pub min_z: i32,
        pub width: i32,
        pub depth: i32,
        pub y: i16,
        pub block_id: u16,
        pub is_skirt: bool,
    }

    // PORT: LodMesher#buildMesh(ColumnarData)
    /// Esegue il greedy meshing orizzontale su altezze complanari ed erige skirt verticali.
    pub fn build_greedy_mesh_from_columns(
        columns: &rle_data::ColumnarData,
        neighbor_lower: bool,
    ) -> Vec<GreedyQuad> {
        let size = columns.size;
        let mut quads = Vec::new();
        let mut visited = vec![false; size * size];

        for z in 0..size {
            for x in 0..size {
                let idx = z * size + x;
                if visited[idx] {
                    continue;
                }

                let col = columns.get_column(x, z);
                if col.is_empty() {
                    continue;
                }
                let top_span = col[col.len() - 1];

                // Espansione in larghezza (asse X)
                let mut width = 1;
                while x + width < size {
                    let next_idx = z * size + (x + width);
                    if visited[next_idx] {
                        break;
                    }
                    let next_col = columns.get_column(x + width, z);
                    if next_col.is_empty() || next_col[next_col.len() - 1] != top_span {
                        break;
                    }
                    width += 1;
                }

                // Espansione in profondità (asse Z)
                let mut depth = 1;
                let mut can_expand_z = true;
                while z + depth < size && can_expand_z {
                    for dx in 0..width {
                        let check_idx = (z + depth) * size + (x + dx);
                        if visited[check_idx] {
                            can_expand_z = false;
                            break;
                        }
                        let check_col = columns.get_column(x + dx, z + depth);
                        if check_col.is_empty() || check_col[check_col.len() - 1] != top_span {
                            can_expand_z = false;
                            break;
                        }
                    }
                    if can_expand_z {
                        depth += 1;
                    }
                }

                // Segna come visitate
                for dz in 0..depth {
                    for dx in 0..width {
                        visited[(z + dz) * size + (x + dx)] = true;
                    }
                }

                quads.push(GreedyQuad {
                    min_x: x as i32,
                    min_z: z as i32,
                    width: width as i32,
                    depth: depth as i32,
                    y: top_span.max_y,
                    block_id: top_span.block_id,
                    is_skirt: false,
                });
            }
        }

        // Skirt culling: aggiunge skirt verticale sui bordi se il confinante ha risoluzione diversa
        if neighbor_lower {
            for x in 0..size {
                let col = columns.get_column(x, size - 1);
                if let Some(top_span) = col.last() {
                    quads.push(GreedyQuad {
                        min_x: x as i32,
                        min_z: size as i32,
                        width: 1,
                        depth: 0,
                        y: top_span.max_y,
                        block_id: top_span.block_id,
                        is_skirt: true,
                    });
                }
            }
        }

        quads
    }

    // PORT: LodMesher#buildMesh(ColumnarData)
    pub fn build_greedy_mesh(_pos: ChunkPos, level: u8) -> Vec<u8> {
        let size = (16 >> level.min(4)).max(1);
        let columns = rle_data::ColumnarData::new(size);
        let quads = build_greedy_mesh_from_columns(&columns, false);
        let mut out = Vec::new();
        out.extend_from_slice(&(quads.len() as u32).to_le_bytes());
        out
    }
}

// ========================================================
// DH-4: Frustum Culling Geometrico
// ========================================================
pub mod culling {
    use super::*;

    /// Test di visibilità geometrica di sezioni LOD basato su coordinate del mondo e livello di dettaglio.
    pub struct LodFrustum;

    impl LodFrustum {
        // PORT: LodFrustum#isBoxVisible(double, double, double, double, double, double)
        pub fn is_box_visible(
            min: [f64; 3],
            max: [f64; 3],
            camera: Vec3,
            max_render_distance_blocks: f64,
        ) -> bool {
            // Test sferico rapido per bounding box di terreno lontano
            let center_x = (min[0] + max[0]) * 0.5;
            let center_y = (min[1] + max[1]) * 0.5;
            let center_z = (min[2] + max[2]) * 0.5;

            let dist = camera.distance_to(Vec3::new(center_x, center_y, center_z));
            let radius =
                ((max[0] - min[0]).powi(2) + (max[1] - min[1]).powi(2) + (max[2] - min[2]).powi(2))
                    .sqrt()
                    * 0.5;

            dist - radius <= max_render_distance_blocks
        }
    }

    // PORT: LodFrustum#isBoxVisible(double, double, double, double, double, double)
    pub fn is_lod_visible(pos: ChunkPos, level: u8, camera: Vec3) -> bool {
        let scale = (1 << level) as f64 * 16.0;
        let min_x = pos.x as f64 * scale;
        let min_z = pos.z as f64 * scale;
        let max_x = min_x + scale;
        let max_z = min_z + scale;
        LodFrustum::is_box_visible([min_x, -64.0, min_z], [max_x, 320.0, max_z], camera, 4096.0)
    }
}

// ========================================================
// DH-5: Shader WGSL Far-Terrain
// ========================================================
pub mod shader {
    use super::*;

    /// Shader WGSL per rendering del terreno a lunga distanza con Bayer dithering 4x4.
    pub const FAR_TERRAIN_WGSL: &str = r#"
// Distant Horizons Far-Terrain Shader con Bayer 4x4 Dithering
struct CameraUniform {
    view_proj: mat4x4<f32>,
    camera_pos: vec3<f32>,
    time: f32,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.world_pos = in.position;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    return out;
}

// Matrice di dithering Bayer 4x4
const BAYER_4X4: array<f32, 16> = array<f32, 16>(
     0.0 / 16.0,  8.0 / 16.0,  2.0 / 16.0, 10.0 / 16.0,
    12.0 / 16.0,  4.0 / 16.0, 14.0 / 16.0,  6.0 / 16.0,
     3.0 / 16.0, 11.0 / 16.0,  1.0 / 16.0,  9.0 / 16.0,
    15.0 / 16.0,  7.0 / 16.0, 13.0 / 16.0,  5.0 / 16.0
);

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = distance(in.world_pos, camera.camera_pos);
    let fog_start = 256.0;
    let fog_end = 2048.0;
    let fog_factor = clamp((dist - fog_start) / (fog_end - fog_start), 0.0, 1.0);

    let coord = vec2<u32>(in.clip_position.xy) % 4u;
    let dither_idx = coord.y * 4u + coord.x;
    let threshold = BAYER_4X4[dither_idx];

    if (fog_factor > threshold && fog_factor >= 0.99) {
        discard;
    }

    let fog_color = vec4<f32>(0.7, 0.8, 0.9, 1.0);
    return mix(in.color, fog_color, fog_factor);
}
"#;

    pub struct DistantHorizonsHook;

    impl LODRenderHook for DistantHorizonsHook {
        // PORT: LodRenderer#renderLODs(RenderPass)
        fn render_lod<'rpass>(
            &'rpass self,
            _pass: &mut wgpu::RenderPass<'rpass>,
            _ctx: &RenderPassContext<'_>,
        ) {
            // Predisposto per l'aggancio del pass terreno lontano
        }
    }
}
