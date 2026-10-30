//! Distant Horizons — Porting nativo per Horizon
//!
//! Implementa la feature list minima concordata in T0.4 (DH-1 .. DH-5).
//! Tutti i corpi dei metodi in attesa di implementazione in Fase 4 usano `todo!("PORT: ...")`.

#![allow(unused)]

use horizon_mod_api::prelude::*;

// ========================================================
// DH-1: Modello Dati Columnar RLE
// ========================================================
pub mod rle_data {
    use super::*;

    // PORT: ColumnarData#downsample(int, int, int)
    pub fn downsample_chunk(pos: ChunkPos, level: u8) -> Vec<u8> {
        todo!("PORT: ColumnarData#downsample - generazione span verticali RLE")
    }
}

// ========================================================
// DH-2: Persistenza su Disco (.hlod)
// ========================================================
pub mod storage {
    use super::*;

    // PORT: LodPersistence#saveChunk(ChunkPos, byte[])
    pub fn save_hlod(pos: ChunkPos, data: &[u8]) -> Result<(), String> {
        todo!("PORT: LodPersistence#saveChunk - compressione LZ4 su .hlod")
    }

    // PORT: LodPersistence#loadChunk(ChunkPos)
    pub fn load_hlod(pos: ChunkPos) -> Result<Option<Vec<u8>>, String> {
        todo!("PORT: LodPersistence#loadChunk - decompressione LZ4 da .hlod")
    }
}

// ========================================================
// DH-3: Greedy Mesher 2.5D con Skirt Culling
// ========================================================
pub mod mesher {
    use super::*;

    // PORT: LodMesher#buildMesh(ColumnarData)
    pub fn build_greedy_mesh(pos: ChunkPos, level: u8) -> Vec<u8> {
        todo!("PORT: LodMesher#buildMesh - meshing greedy 2.5D e skirt verticali")
    }
}

// ========================================================
// DH-4: Frustum Culling Geometrico
// ========================================================
pub mod culling {
    use super::*;

    // PORT: LodFrustum#isBoxVisible(double, double, double, double, double, double)
    pub fn is_lod_visible(pos: ChunkPos, level: u8, camera: Vec3) -> bool {
        todo!("PORT: LodFrustum#isBoxVisible - culling geometrico bounding box")
    }
}

// ========================================================
// DH-5: Shader WGSL Far-Terrain
// ========================================================
pub mod shader {
    use super::*;

    pub struct DistantHorizonsHook;

    impl LODRenderHook for DistantHorizonsHook {
        // PORT: LodRenderer#renderLODs(RenderPass)
        fn render_lod<'rpass>(
            &'rpass self,
            pass: &mut wgpu::RenderPass<'rpass>,
            ctx: &RenderPassContext<'_>,
        ) {
            todo!("PORT: LodRenderer#renderLODs - dispatch far terrain pass WGSL")
        }
    }
}
