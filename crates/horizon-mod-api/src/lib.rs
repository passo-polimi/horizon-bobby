//! # Horizon Mod API
//!
//! Livello di compatibilità ed integrazione in Rust nativo per il porting di mod
//! Minecraft Java (Fabric) in Horizon.
//!
//! Offre tipi standard di Minecraft, event bus thread-safe, gestione configurazioni
//! serde, routing di pacchetti custom di rete e hook di rendering wgpu.

pub mod config;
pub mod draw;
pub mod event;
pub mod net;
pub mod render_hook;
pub mod types;

pub use wgpu;

pub mod prelude {
    pub use crate::config::{Config, ModConfig};
    pub use crate::draw::{Color, DrawContext};
    pub use crate::event::{
        ChunkLoadEvent, ChunkUnloadEvent, ClientTickEvent, EventBus, HorizonEvent, HorizonListener,
        ModInitEvent, Phase, RenderTickEvent, WorldLoadEvent, WorldUnloadEvent,
    };
    pub use crate::net::{NetworkRegistry, read_varint, write_varint};
    pub use crate::render_hook::{
        GPUCullingHook, LODRenderHook, RenderHookRegistry, RenderPassContext,
    };
    pub use crate::types::{BlockPos, ChunkPos, Direction, Identifier, TextComponent, Vec3};
    pub use wgpu;
}

pub use prelude::*;
