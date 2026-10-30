//! Nvidium — Porting nativo per Horizon
//!
//! Implementa la feature list concordata in T0.4 (NV-1 .. NV-5):
//! - NV-1: Bounding Box & Frustum Culling GPU
//! - NV-2: Hierarchical Z-Buffer (Hi-Z) Occlusion Culling
//! - NV-3: Meshlet Packaging (formato compresso 16-byte)
//! - NV-4: Multi-Draw Indirect (MDI)
//! - NV-5: Integrazione RenderPass wgpu con GPUCullingHook

use horizon_mod_api::prelude::*;

// ========================================================
// NV-1: Bounding Box & Frustum Culling GPU
// ========================================================
pub mod frustum {
    use super::*;

    /// Bounding box assiale (AABB) in coordinate assolute del mondo.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct BoundingBox {
        pub min: [f32; 3],
        pub max: [f32; 3],
    }

    impl BoundingBox {
        pub fn from_section(section_pos: BlockPos) -> Self {
            let min_x = (section_pos.x * 16) as f32;
            let min_y = (section_pos.y * 16) as f32;
            let min_z = (section_pos.z * 16) as f32;
            Self {
                min: [min_x, min_y, min_z],
                max: [min_x + 16.0, min_y + 16.0, min_z + 16.0],
            }
        }

        pub fn center(&self) -> [f32; 3] {
            [
                (self.min[0] + self.max[0]) * 0.5,
                (self.min[1] + self.max[1]) * 0.5,
                (self.min[2] + self.max[2]) * 0.5,
            ]
        }

        pub fn extents(&self) -> [f32; 3] {
            [
                (self.max[0] - self.min[0]) * 0.5,
                (self.max[1] - self.min[1]) * 0.5,
                (self.max[2] - self.min[2]) * 0.5,
            ]
        }
    }

    /// Piano di frustum espresso come equazione: $ax + by + cz + d = 0$.
    #[derive(Debug, Clone, Copy)]
    pub struct Plane {
        pub normal: [f32; 3],
        pub d: f32,
    }

    impl Plane {
        pub fn new(a: f32, b: f32, c: f32, d: f32) -> Self {
            let len = (a * a + b * b + c * c).sqrt();
            if len > 1e-6 {
                Self {
                    normal: [a / len, b / len, c / len],
                    d: d / len,
                }
            } else {
                Self {
                    normal: [a, b, c],
                    d,
                }
            }
        }

        pub fn distance_to(&self, point: [f32; 3]) -> f32 {
            self.normal[0] * point[0]
                + self.normal[1] * point[1]
                + self.normal[2] * point[2]
                + self.d
        }
    }

    /// Frustum di vista composto dai 6 piani di delimitazione.
    #[derive(Debug, Clone)]
    pub struct Frustum {
        pub planes: [Plane; 6],
    }

    impl Frustum {
        // PORT: RegionCuller#fromMatrix(Matrix4f)
        /// Estrae i 6 piani del frustum da una matrice view-projection (row-major o column-major 4x4).
        pub fn from_view_projection(m: &[f32; 16]) -> Self {
            // Estrazione standard Gribb-Hartmann
            let planes = [
                // Left
                Plane::new(m[3] + m[0], m[7] + m[4], m[11] + m[8], m[15] + m[12]),
                // Right
                Plane::new(m[3] - m[0], m[7] - m[4], m[11] - m[8], m[15] - m[12]),
                // Bottom
                Plane::new(m[3] + m[1], m[7] + m[5], m[11] + m[9], m[15] + m[13]),
                // Top
                Plane::new(m[3] - m[1], m[7] - m[5], m[11] - m[9], m[15] - m[13]),
                // Near
                Plane::new(m[3] + m[2], m[7] + m[6], m[11] + m[10], m[15] + m[14]),
                // Far
                Plane::new(m[3] - m[2], m[7] - m[6], m[11] - m[10], m[15] - m[14]),
            ];
            Self { planes }
        }

        // PORT: RegionCuller#cullFrustum(Camera)
        /// Verifica se una bounding box interseca o è contenuta nel frustum.
        pub fn is_box_visible(&self, bbox: &BoundingBox) -> bool {
            let center = bbox.center();
            let extents = bbox.extents();

            for plane in &self.planes {
                let r = extents[0] * plane.normal[0].abs()
                    + extents[1] * plane.normal[1].abs()
                    + extents[2] * plane.normal[2].abs();
                if plane.distance_to(center) < -r {
                    return false;
                }
            }
            true
        }
    }

    // PORT: RegionCuller#cullFrustum(Camera)
    pub fn cull_frustum_gpu(section_pos: BlockPos, view_proj: [f32; 16]) -> bool {
        let frustum = Frustum::from_view_projection(&view_proj);
        let bbox = BoundingBox::from_section(section_pos);
        frustum.is_box_visible(&bbox)
    }
}

// ========================================================
// NV-2: Hierarchical Z-Buffer (Hi-Z) Occlusion Culling
// ========================================================
pub mod hiz {
    /// Piramide gerarchica delle profondità (Hi-Z Buffer) in memoria GPU/CPU.
    #[derive(Debug)]
    pub struct HiZPyramid {
        pub width: u32,
        pub height: u32,
        pub levels: Vec<Vec<f32>>,
    }

    impl HiZPyramid {
        pub fn new(width: u32, height: u32) -> Self {
            let mut levels = Vec::new();
            let mut w = width.max(1);
            let mut h = height.max(1);

            let num_levels = (w.max(h) as f32).log2().floor() as usize + 1;
            for _ in 0..num_levels {
                levels.push(vec![1.0; (w * h) as usize]);
                w = (w / 2).max(1);
                h = (h / 2).max(1);
            }

            Self {
                width,
                height,
                levels,
            }
        }

        // PORT: OcclusionCuller#buildDepthPyramid(Texture)
        /// Ricostruisce la piramide campionando con filtro conservativo (MAX per Reverse-Z o MIN per standard).
        pub fn update_hiz_from_depth(&mut self, base_depth: &[f32]) {
            if let Some(level0) = self.levels.get_mut(0) {
                let count = level0.len().min(base_depth.len());
                level0[..count].copy_from_slice(&base_depth[..count]);
            }

            for l in 1..self.levels.len() {
                let prev_w = (self.width >> (l - 1)).max(1);
                let cur_w = (self.width >> l).max(1);
                let cur_h = (self.height >> l).max(1);

                for y in 0..cur_h {
                    for x in 0..cur_w {
                        let px = x * 2;
                        let py = y * 2;

                        let mut max_depth = f32::MIN;
                        for dy in 0..2 {
                            for dx in 0..2 {
                                let sx = px + dx;
                                let sy = py + dy;
                                let prev_idx = (sy * prev_w + sx) as usize;
                                if let Some(&d) = self.levels[l - 1].get(prev_idx) {
                                    max_depth = max_depth.max(d);
                                }
                            }
                        }

                        let cur_idx = (y * cur_w + x) as usize;
                        if cur_idx < self.levels[l].len() {
                            self.levels[l][cur_idx] = max_depth;
                        }
                    }
                }
            }
        }

        // PORT: OcclusionCuller#testOcclusion(BoundingBox)
        /// Esegue il test di occlusione verificando la profondità conservativa alla risoluzione di mip ottimale.
        pub fn test_occlusion(&self, bbox_depth: f32, screen_size_pixels: f32) -> bool {
            let mip = ((screen_size_pixels.max(1.0)).log2().floor() as usize)
                .min(self.levels.len().saturating_sub(1));

            if let Some(level) = self.levels.get(mip) {
                // Campionamento conservativo sul mip
                let min_pyramid_depth = level.iter().copied().fold(f32::INFINITY, f32::min);
                // Se la profondità minima del chunk è maggiore della profondità occlusa, il chunk è nascosto
                bbox_depth > min_pyramid_depth
            } else {
                false
            }
        }
    }
}

// ========================================================
// NV-3: Meshlet Packaging
// ========================================================
pub mod meshlet {
    /// Formato compresso vertice Nvidium (16 byte per vertice).
    /// Conforme a `NvidiumCompactChunkVertex.java`.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
    pub struct CompactVertex {
        pub pos_xy: u32,
        pub pos_z_light: u32,
        pub color_light: u32,
        pub uv: u32,
    }

    impl CompactVertex {
        // PORT: NvidiumCompactChunkVertex#getEncoder()
        pub fn new(x: f32, y: f32, z: f32, u: f32, v: f32, light: i32, color_rgba: u32) -> Self {
            let enc_x = ((8.0 + x) * (65536.0 / 32.0)) as u32;
            let enc_y = ((8.0 + y) * (65536.0 / 32.0)) as u32;
            let enc_z = ((8.0 + z) * (65536.0 / 32.0)) as u32;

            let u_light = light as u32;
            let sky = ((u_light >> 16) & 0xFF).clamp(8, 248);
            let block = (u_light & 0xFF).clamp(8, 248);
            let compact_light = block | (sky << 8);

            let round_u = (u * 32768.0 + 0.5).floor() as u32 & 0xFFFF;
            let round_v = (v * 32768.0 + 0.5).floor() as u32 & 0xFFFF;

            Self {
                pos_xy: (enc_x & 0xFFFF) | ((enc_y & 0xFFFF) << 16),
                pos_z_light: (enc_z & 0xFFFF) | ((compact_light & 0xFF) << 24),
                color_light: (color_rgba & 0x00FF_FFFF) | (((compact_light >> 8) & 0xFF) << 24),
                uv: round_u | (round_v << 16),
            }
        }
    }

    /// Cluster meshlet contenente un blocco locale di vertici ed indici.
    #[derive(Debug, Clone, Default)]
    pub struct ChunkMeshlet {
        pub vertices: Vec<CompactVertex>,
        pub indices: Vec<u32>,
    }

    impl ChunkMeshlet {
        // PORT: ChunkMeshlet#packVertices(ChunkData)
        pub fn pack_quad(
            &mut self,
            quad_positions: [[f32; 3]; 4],
            quad_uvs: [[f32; 2]; 4],
            light: i32,
            color: u32,
        ) {
            let base_idx = self.vertices.len() as u32;
            for i in 0..4 {
                self.vertices.push(CompactVertex::new(
                    quad_positions[i][0],
                    quad_positions[i][1],
                    quad_positions[i][2],
                    quad_uvs[i][0],
                    quad_uvs[i][1],
                    light,
                    color,
                ));
            }
            // Due triangoli per quad: 0, 1, 2 e 2, 3, 0
            self.indices.extend_from_slice(&[
                base_idx,
                base_idx + 1,
                base_idx + 2,
                base_idx + 2,
                base_idx + 3,
                base_idx,
            ]);
        }
    }
}

// ========================================================
// NV-4: Multi-Draw Indirect (MDI)
// ========================================================
pub mod indirect {
    use bytemuck::{Pod, Zeroable};

    /// Struttura standard wgpu per comando `DrawIndexedIndirect` (20 byte).
    #[repr(C)]
    #[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
    pub struct DrawIndexedIndirectCommand {
        pub index_count: u32,
        pub instance_count: u32,
        pub first_index: u32,
        pub base_vertex: i32,
        pub first_instance: u32,
    }

    impl DrawIndexedIndirectCommand {
        pub fn new(index_count: u32, first_index: u32, base_vertex: i32) -> Self {
            Self {
                index_count,
                instance_count: 1,
                first_index,
                base_vertex,
                first_instance: 0,
            }
        }
    }

    /// Buffer contenente i comandi indiretti per GPU.
    #[derive(Debug, Default)]
    pub struct IndirectDrawBuffer {
        pub commands: Vec<DrawIndexedIndirectCommand>,
    }

    impl IndirectDrawBuffer {
        // PORT: IndirectDrawBuffer#emitCommands(List<Meshlet>)
        pub fn emit_draw_command(&mut self, index_count: u32, first_index: u32, base_vertex: i32) {
            self.commands.push(DrawIndexedIndirectCommand::new(
                index_count,
                first_index,
                base_vertex,
            ));
        }

        pub fn clear(&mut self) {
            self.commands.clear();
        }

        pub fn byte_len(&self) -> usize {
            self.commands.len() * std::mem::size_of::<DrawIndexedIndirectCommand>()
        }
    }
}

// ========================================================
// NV-5: Integrazione RenderPass wgpu
// ========================================================
pub mod pipeline {
    use super::*;

    pub struct NvidiumHook {
        pub indirect_commands: std::sync::Mutex<indirect::IndirectDrawBuffer>,
    }

    impl Default for NvidiumHook {
        fn default() -> Self {
            Self {
                indirect_commands: std::sync::Mutex::new(indirect::IndirectDrawBuffer::default()),
            }
        }
    }

    impl GPUCullingHook for NvidiumHook {
        // PORT: RenderPipeline#prepareCulling(CommandEncoder)
        fn prepare_culling(
            &self,
            _encoder: &mut wgpu::CommandEncoder,
            _ctx: &RenderPassContext<'_>,
        ) {
            let mut cmds = self.indirect_commands.lock().unwrap();
            cmds.clear();
        }

        // PORT: RenderPipeline#getIndirectBuffer()
        fn indirect_buffer(&self) -> Option<&wgpu::Buffer> {
            None
        }
    }
}
