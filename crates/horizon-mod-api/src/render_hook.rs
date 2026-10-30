//! Hook di rendering e contratti per estensioni grafiche su wgpu (Task T2.6).
//!
//! Contratto di Threading:
//! Tutte le callback di rendering vengono eseguite sul thread di rendering principale
//! (`Render Thread`) all'interno dell'encoder wgpu attivo. I dati di calcolo intensivo
//! (meshing, aggiornamento ottanti) devono risiedere su thread worker separati per non
//! bloccare il frame rate.

use glam::{DVec3, Mat4};

/// Contesto di rendering passato ai render pass delle mod.
pub struct RenderPassContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub camera_pos: DVec3,
    pub view_proj: Mat4,
    pub view_distance_chunks: u32,
    pub delta_time_secs: f32,
}

/// Hook per il rendering di terreno distante a lunghissimo raggio (Distant Horizons).
pub trait LODRenderHook: Send + Sync + 'static {
    /// Invocato subito dopo il rendering dei chunk vanilla e prima dell'HUD.
    fn render_lod<'rpass>(
        &'rpass self,
        pass: &mut wgpu::RenderPass<'rpass>,
        ctx: &RenderPassContext<'_>,
    );
}

/// Hook per culling accelerato su GPU e comandi di disegno indiretto (Nvidium).
pub trait GPUCullingHook: Send + Sync + 'static {
    /// Invocato all'inizio del frame prima del disegno per generare le liste di disegno indiretto.
    fn prepare_culling(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        ctx: &RenderPassContext<'_>,
    );

    /// Fornisce il buffer dei comandi indiretti per le draw call.
    fn indirect_buffer(&self) -> Option<&wgpu::Buffer>;
}

/// Registro globale degli hook di rendering registrati dalle mod.
#[derive(Default)]
pub struct RenderHookRegistry {
    pub lod_hooks: Vec<Box<dyn LODRenderHook>>,
    pub culling_hooks: Vec<Box<dyn GPUCullingHook>>,
}

impl RenderHookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_lod<H: LODRenderHook>(&mut self, hook: H) {
        self.lod_hooks.push(Box::new(hook));
    }

    pub fn register_culling<H: GPUCullingHook>(&mut self, hook: H) {
        self.culling_hooks.push(Box::new(hook));
    }
}

