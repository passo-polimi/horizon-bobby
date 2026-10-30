# Tabella dei Simboli di Compatibilità: Java (Mojmap) → Horizon Mod API

*Generato in Fase 2 (Task T2.7) · Data: 2026-10-30*  
*Base per la corrispondenza dei simboli, generazione di binding e parità differenziale.*

---

## 1. Tipi Primitivi e Geometrici

| Simbolo Java (Mojmap) | Equivalente Horizon Mod API | File Sorgente Rust | Note di Semantica & Differenze |
|---|---|---|---|
| `net.minecraft.core.BlockPos` | `horizon_mod_api::types::BlockPos` | [`crates/horizon-mod-api/src/types.rs`](../../crates/horizon-mod-api/src/types.rs) | Coordinate $x, y, z$ con aritmetica `wrapping_*`. Impaccamento `as_long` identico a 64-bit. |
| `net.minecraft.world.level.ChunkPos` | `horizon_mod_api::types::ChunkPos` | [`crates/horizon-mod-api/src/types.rs`](../../crates/horizon-mod-api/src/types.rs) | Coordinate $x, z$ di chunk ($16\times 16$). Impaccamento a 64-bit identico a Java. |
| `net.minecraft.world.phys.Vec3` | `horizon_mod_api::types::Vec3` | [`crates/horizon-mod-api/src/types.rs`](../../crates/horizon-mod-api/src/types.rs) | Floating point a doppia precisione (`f64`). Metodi `add`, `distance_to`. |
| `net.minecraft.core.Direction` | `horizon_mod_api::types::Direction` | [`crates/horizon-mod-api/src/types.rs`](../../crates/horizon-mod-api/src/types.rs) | Enum delle 6 direzioni (`Down`, `Up`, `North`, `South`, `West`, `East`) con `opposite()` e `normal()`. |
| `net.minecraft.resources.ResourceLocation` | `horizon_mod_api::types::Identifier` | [`crates/horizon-mod-api/src/types.rs`](../../crates/horizon-mod-api/src/types.rs) | Stringhe `namespace:path`, default su `minecraft`. Display e Serialize conformi. |
| `net.minecraft.network.chat.Component` | `horizon_mod_api::types::TextComponent` | [`crates/horizon-mod-api/src/types.rs`](../../crates/horizon-mod-api/src/types.rs) | Testo con formattazione e colori RGBA. |

---

## 2. Eventi e Ciclo di Vita (Lifecycle)

| Callback Fabric API / Mixin Java | Evento Horizon Mod API | Dispatch Point in Horizon | Note |
|---|---|---|---|
| `net.fabricmc.api.ModInitializer#onInitialize` | `horizon_mod_api::event::ModInitEvent` | Inizializzazione client (`game.rs`) | Eseguito una sola volta all'avvio. |
| `ClientTickEvents.START_CLIENT_TICK` | `ClientTickEvent { phase: Phase::Start }` | Inizio di `Game::tick()` ([`game.rs:5287`](../../crates/horizon-client/src/game.rs#L5287)) | 20 TPS fixed timestep. |
| `ClientTickEvents.END_CLIENT_TICK` | `ClientTickEvent { phase: Phase::End }` | Fine di `Game::tick()` | Nessun lock tenuto durante le callback. |
| `WorldRenderEvents.START` | `horizon_mod_api::event::RenderTickEvent` | `Renderer::render()` | Fornisce `partial_ticks` e `delta_time_secs`. |
| `ClientPlayConnectionEvents.JOIN` | `horizon_mod_api::event::WorldLoadEvent` | `Game::remote_login()` / integrated start | Connessione mondo single/multiplayer. |
| `ClientPlayConnectionEvents.DISCONNECT` | `horizon_mod_api::event::WorldUnloadEvent` | Chiusura del mondo | Cleanup delle risorse mod. |
| `ClientChunkEvents.CHUNK_LOAD` | `horizon_mod_api::event::ChunkLoadEvent` | `Game::remote_chunk()` | Chunk ricevuto dal server / generato. |
| `ClientChunkEvents.CHUNK_UNLOAD` | `horizon_mod_api::event::ChunkUnloadEvent` | `Game::remote.forget()` | Chunk rimosso dallo streaming. |

---

## 3. Disegno 2D, HUD e Interfaccia

| Metodo Java (Mojmap) | Metodo Horizon Mod API | Target di Disegno | Note |
|---|---|---|---|
| `GuiGraphics#fill(int, int, int, int, int)` | `DrawContext::fill(x1, y1, x2, y2, color)` | `HudBatch` vertex buffer | Batching rettangoli pieni con blend standard. |
| `GuiGraphics#drawString(Font, String, int, int, int)` | `DrawContext::draw_text(text, x, y, color)` | `HudBatch` text atlas | Rendering font integrato con upload glifi. |
| `GuiGraphics#pose().translate(x, y, z)` | `DrawContext::translate(dx, dy)` | Offset 2D interno | Emulazione matrix stack per coordinate relative. |
| `HudRenderCallback#onHudRender` | `DrawContext` in render callback | Pass HUD di Horizon ([`hud.rs:1333`](../../crates/horizon-client-render/src/hud.rs#L1333)) | Nessun lock su GPU durante la costruzione del batch. |

---

## 4. Networking e Canali Custom (Plugin Messages)

| Concetto Java | Equivalente Horizon Mod API | File Sorgente Rust |
|---|---|---|
| `CustomPacketPayload` | `NetworkRegistry::register_receiver(channel, cb)` | [`crates/horizon-mod-api/src/net.rs`](../../crates/horizon-mod-api/src/net.rs) |
| `FriendlyByteBuf#writeVarInt(int)` | `horizon_mod_api::net::write_varint(w, val)` | [`crates/horizon-mod-api/src/net.rs`](../../crates/horizon-mod-api/src/net.rs) |
| `FriendlyByteBuf#readVarInt()` | `horizon_mod_api::net::read_varint(r)` | [`crates/horizon-mod-api/src/net.rs`](../../crates/horizon-mod-api/src/net.rs) |
| `FriendlyByteBuf#writeUtf(String)` | `horizon_mod_api::net::write_string(w, s)` | [`crates/horizon-mod-api/src/net.rs`](../../crates/horizon-mod-api/src/net.rs) |
| `FriendlyByteBuf#readUtf()` | `horizon_mod_api::net::read_string(r)` | [`crates/horizon-mod-api/src/net.rs`](../../crates/horizon-mod-api/src/net.rs) |

---

## 5. Hook di Rendering Avanzati (LOD & GPU Culling)

| Mod / Meccanismo Java | Trait Hook Horizon Mod API | File Sorgente Rust | Note di Esecuzione |
|---|---|---|---|
| Distant Horizons Far-Terrain Pass | `horizon_mod_api::render_hook::LODRenderHook` | [`crates/horizon-mod-api/src/render_hook.rs`](../../crates/horizon-mod-api/src/render_hook.rs) | Invocato nel pass `Renderer::render()` dopo il mondo vanilla. |
| Nvidium Hi-Z Occlusion & Indirect | `horizon_mod_api::render_hook::GPUCullingHook` | [`crates/horizon-mod-api/src/render_hook.rs`](../../crates/horizon-mod-api/src/render_hook.rs) | Invocato prima delle draw call per generare comandi MDI. |

