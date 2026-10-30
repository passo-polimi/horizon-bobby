# Audit architetturale di Horizon per il Mod Porting

*Generato in Fase 0 (Task T0.1) · Data: 2026-10-30*  
*Tutti i percorsi e le righe citati sono stati verificati direttamente sul repository.*

---

## 1. Organizzazione dei Crate nel Workspace

Il repository radice definisce un workspace Cargo in [`Cargo.toml`](../../Cargo.toml#L1-L49):
- **Edition & MSRV:** Rust 2024, `rust-version = "1.85"` ([`Cargo.toml`](../../Cargo.toml#L51-L55)), con crate specifici compilati fino a `rust-version = "1.90"` (es. `horizon-client-render`).
- **Crate core:**
  - `crates/horizon-core`: primitive di simulazione, matematica, costanti di versione Minecraft ([`crates/horizon-core/src/version.rs`](../../crates/horizon-core/src/version.rs)).
  - `crates/horizon-protocol`: codec di rete, framer VarInt, crittografia e pacchetti tipizzati ([`crates/horizon-protocol/src/lib.rs`](../../crates/horizon-protocol/src/lib.rs)).
  - `crates/horizon-world`: chunks, blocchi, chunk generator, collisioni e registry ([`crates/horizon-world/src/lib.rs`](../../crates/horizon-world/src/lib.rs)).
  - `crates/horizon-client-render`: pipeline wgpu, meshing, luci, camera, HUD, sky, LOD nativo ([`crates/horizon-client-render/src/lib.rs`](../../crates/horizon-client-render/src/lib.rs)).
  - `crates/horizon-client`: loop dell'applicazione, integrazione winit, multiplayer client, hook nativi ([`crates/horizon-client/src/game.rs`](../../crates/horizon-client/src/game.rs)).
  - `crates/horizon-shader-api` e `crates/horizon-shaders`: parser shaderpack GLSL e traduttore Naga WGSL.
  - `crates/horizon-mods` e `crates/horizon-mod-sdk`: runtime WebAssembly Wasmtime 48 LTS basato su component model WIT.
  - `crates/horizon-port-pack`: strumenti batch per modpack ATM10 e conversioni dati ([`crates/horizon-port-pack/src/lib.rs`](../../crates/horizon-port-pack/src/lib.rs)).

---

## 2. Pipeline di Rendering wgpu e Registrazione Pass/Shader

- **Stato:** **Parziale** (Rendering completo e performante, ma monolitico: registrazione dinamica di pass/shader mancante).
- **File verificati:**
  - [`crates/horizon-client-render/Cargo.toml`](../../crates/horizon-client-render/Cargo.toml#L43): `wgpu = "30.0.1"`, `naga = "30.0.1"`, `bytemuck = "1.24"`.
  - [`crates/horizon-client-render/src/gfx.rs`](../../crates/horizon-client-render/src/gfx.rs#L368-L470): struct `Renderer` e inizializzazione pipeline (`wgpu::Device`, `wgpu::Queue`, `wgpu::Surface`).
- **Pass di rendering esistenti:**
  I pass sono codificati direttamente all'interno di `Renderer::render`:
  1. *Sky pass* ([`gfx.rs:4736`](../../crates/horizon-client-render/src/gfx.rs#L4736)): `"Horizon sky"`.
  2. *World pass* ([`gfx.rs:4790`](../../crates/horizon-client-render/src/gfx.rs#L4790)): `"Horizon world"` (chunk meshati con `TERRAIN_ATTRIBUTES`, entità, particelle, weather, crumbling/breaking).
  3. *LOD far-terrain pass* ([`crates/horizon-client-render/src/lod.rs`](../../crates/horizon-client-render/src/lod.rs#L1-L60) e [`gfx.rs`](../../crates/horizon-client-render/src/gfx.rs)): rendering columnar far-terrain.
  4. *First-person hand* ([`gfx.rs:5244`](../../crates/horizon-client-render/src/gfx.rs#L5244)).
  5. *Screen effects* ([`gfx.rs:5295`](../../crates/horizon-client-render/src/gfx.rs#L5295)).
  6. *3D crosshair* ([`gfx.rs:5324`](../../crates/horizon-client-render/src/gfx.rs#L5324)).
  7. *HUD pass* ([`crates/horizon-client-render/src/hud.rs:1333`](../../crates/horizon-client-render/src/hud.rs#L1333)): `HudRenderer::render`.
- **Punti di estensione per modding:**
  - *Attualmente disponibili:* Nessun'interfaccia a plugin per inserire RenderPass wgpu custom o BindGroup arbitrari. Mod con pipeline custom (Distant Horizons / Voxy) richiedono o modifiche dirette a `Renderer` o l'introduzione di un trait `RenderHook` / `PassCallback` in `horizon-client-render`.
  - *Shaders custom:* `crates/horizon-shaders` traduce shader pack GLSL OptiFine/Iris in moduli WGSL, ma non espone API per caricare shader di compute pass o pass grafici dedicati delle mod.

---

## 3. Loop di Tick ed Eventi

- **Stato:** **Esiste** (Loop centrale robusto su winit; dispatch eventi per mod parziale).
- **File verificati:**
  - [`crates/horizon-client/src/game.rs`](../../crates/horizon-client/src/game.rs#L13061-L14120): `impl ApplicationHandler for Game`.
  - [`crates/horizon-client/src/game.rs:5287`](../../crates/horizon-client/src/game.rs#L5287): `fn tick(&mut self) -> Result<()>`.
- **Funzionamento:**
  - Event loop guidato da `winit::application::ApplicationHandler`:
    - `resumed`: creazione della finestra e del renderer.
    - `window_event`: input da tastiera, mouse, resize, focus, chiusura.
    - `about_to_wait`: tick della simulazione (20 TPS fixed timestep), sincronizzazione animazioni, polling del client remoto (`poll_remote`), streaming chunk e richiesta redraw.
- **Punti di estensione per modding:**
  - In `crates/horizon-client/src/game/native_hooks.rs` sono presenti punti vuoti a chiamata diretta (es. `native_zoom`, `native_panel_event`), ma manca un `EventBus` tipizzato (es. `ClientTickEvent`, `WorldLoadEvent`, `RenderTickEvent`) con registri di callback per mod.

---

## 4. Networking e Protocollo

- **Stato:** **Esiste** per il protocollo vanilla; **Parziale** per payload personalizzati delle mod.
- **File verificati:**
  - [`crates/horizon-protocol/src/packets/play.rs`](../../crates/horizon-protocol/src/packets/play.rs): pacchetti clientbound e serverbound di gioco.
  - [`crates/horizon-protocol/src/packets/configuration.rs`](../../crates/horizon-protocol/src/packets/configuration.rs): pacchetti di negoziazione e custom payload.
  - [`crates/horizon-client/src/net.rs`](../../crates/horizon-client/src/net.rs): smistamento pacchetti di rete in arrivo.
  - [`crates/horizon-client/src/game/remote.rs`](../../crates/horizon-client/src/game/remote.rs): gestione sessione multiplayer client.
- **Plugin Messages (CustomPayload):**
  - Il tipo `CustomPayload { channel: String, data: Vec<u8> }` è definito in `horizon-protocol`.
  - In `crates/horizon-client/src/net.rs` vengono gestiti unicamente i canali `"minecraft:brand"` e `"horizon:quiet"`.
- **Punti di estensione per modding:**
  - È necessario esporre in `horizon-mod-api` un registro per canali di rete custom:
    `register_clientbound_channel(channel: &str, handler: Box<dyn Fn(&[u8])>)` e `send_custom_payload(channel: &str, data: &[u8])`.

---

## 5. UI e HUD

- **Stato:** **Esiste** (Sistema di disegno HUD performante in batch; estendibilità mod parziale).
- **File verificati:**
  - [`crates/horizon-client-render/src/hud.rs`](../../crates/horizon-client-render/src/hud.rs#L650): `HudBatch` (vertici 2D `HudVertex`, colori `rgba`, text patching e blend modes).
  - [`crates/horizon-client-render/src/hud.rs:1333`](../../crates/horizon-client-render/src/hud.rs#L1333): `HudRenderer`.
  - [`crates/horizon-client/src/game/native_hooks.rs`](../../crates/horizon-client/src/game/native_hooks.rs#L9-L13): API per minimap, panel item e ricette (`native_panel_items`, `native_panel_recipes`, `native_panel_event`).
- **Funzionamento:**
  - Tutte le chiamate di disegno 2D accumulano vertici in `HudBatch` durante il frame (rettangoli, sprite da atlas, glifi di font e tooltip).
- **Punti di estensione per modding:**
  - Fornire in `horizon-mod-api` un wrapper sicuro tipo `DrawContext` che incapsuli `HudBatch` per consentire alle mod di disegnare testo, rettangoli e sprite senza toccare direttamente i raw pointer o gli indici interni.

---

## 6. Registry di Blocchi, Item, Biomi ed Entità

- **Stato:** **Parziale** (Registry vanilla completi e immutabili; iniezione dinamica a runtime mancante).
- **File verificati:**
  - [`crates/horizon-world/src/registry.rs`](../../crates/horizon-world/src/registry.rs#L17): `BlockRegistry` (indicizzato tramite hash `state_key` e `BlockStateId(pub u32)`).
  - [`crates/horizon-items/src/lib.rs`](../../crates/horizon-items/src/lib.rs): definizioni e registry item.
  - [`crates/horizon-core/src/biome.rs`](../../crates/horizon-core/src/biome.rs): registry biomi.
- **Funzionamento:**
  - I dati di blocchi e item sono caricati e bloccati al bootstrap da `horizon-resources.json` e tag NBT generati per Minecraft 26.3 (`DATA_VERSION = 5023`).
- **Punti di estensione per modding:**
  - Per mod che aggiungono nuovi blocchi o item (Tier content), serve il pipeline di preprocessing `horizon-port-pack` o un registry dinamico a runtime.
  - Per mod puramente client-side (come HUD, rendering o QoL), la sola lettura tramite `BlockStateId` e `ItemState` è sufficiente.

---

## 7. Config e Persistenza

- **Stato:** **Esiste** per le opzioni native del client; **Parziale** per le mod.
- **File verificati:**
  - [`crates/horizon-client-render/src/options_file.rs`](../../crates/horizon-client-render/src/options_file.rs): parsing e scrittura di `options.txt`.
  - [`crates/horizon-client/src/game/native_hooks.rs:6`](../../crates/horizon-client/src/game/native_hooks.rs#L6): cartella impostazioni mod (`mod_settings_path(id)`).
- **Punti di estensione per modding:**
  - In `horizon-mod-api` serve un helper basato su `serde` (`serde_json` o RON) per salvare/caricare automaticamente la configurazione di ciascuna mod in `<instance>/config/<mod_id>.json`.

---

## 8. Input e Keybinds

- **Stato:** **Esiste** (Keybind vanilla mappati staticamente su winit `KeyCode`).
- **File verificati:**
  - [`crates/horizon-client-render/src/keybinds.rs`](../../crates/horizon-client-render/src/keybinds.rs): struct `KeyBinds` e mappa statica dei tasti vanilla.
  - [`crates/horizon-client/src/game/interaction_input.rs`](../../crates/horizon-client/src/game/interaction_input.rs): gestione click e azioni di gioco.
  - [`crates/horizon-client/src/game/native_hooks.rs:213`](../../crates/horizon-client/src/game/native_hooks.rs#L213): `pending_keys: Vec<KeyCode>`.
- **Punti di estensione per modding:**
  - Manca un registro dinamico per registrare tasti rapidi custom delle mod (es. zoom key, toggle HUD) visibili nel menu Opzioni > Controlli.

---

## 9. Infrastruttura Mod Preesistente in Horizon

Horizon supporta nativamente due percorsi di modding:
1. **WebAssembly Components (Sandboxed):**
   - [`crates/horizon-mods`](../../crates/horizon-mods/) con Wasmtime 48 LTS e contratti WIT in [`crates/horizon-mod-sdk/wit/horizon.wit`](../../crates/horizon-mod-sdk/wit/horizon.wit).
   - Sicuro ed isolato, ma limitato nelle prestazioni e impossibilitato ad accedere a shader WGSL nativi o buffer di memoria diretta GPU.
2. **Native Mods (In-tree mixins / hooks):**
   - Documentato in [`docs/native-mods.md`](../../docs/native-mods.md) e [`crates/horizon-client/src/game/native_hooks.rs`](../../crates/horizon-client/src/game/native_hooks.rs).
   - `NATIVE_HOOKS_API = 2`.
   - Mod compilate nativamente con il client tramite bundle `horizon-mixins`.

---

## 10. Tabella di Sintesi per Sottosistema

| Sottosistema | Stato | Percorsi Chiave Verificati | Gap & Punti di Estensione per `horizon-mod-api` |
|---|---|---|---|
| **Pipeline wgpu / Pass** | Parziale | `horizon-client-render/src/gfx.rs` | Nessun hook per render pass/pipeline custom; pass monolitici in `Renderer`. |
| **Shader System** | Esiste | `horizon-shaders/`, `horizon-shader-api/` | Traduce GLSL in WGSL per pack OptiFine/Iris; manca caricamento di shader WGSL sciolti per mod. |
| **Event / Tick Loop** | Esiste | `horizon-client/src/game.rs` | Timestep a 20 TPS fisso presente; serve un EventBus tipizzato con listener (`HorizonListener<E>`). |
| **Networking Custom** | Parziale | `horizon-protocol/src/packets/`, `horizon-client/src/net.rs` | Pacchetti `CustomPayload` presenti ma instradati solo per brand/quiet; manca router per canali mod. |
| **UI & 2D HUD** | Esiste | `horizon-client-render/src/hud.rs` | `HudBatch` e rendering 2D completi; serve interfaccia sicura `DrawContext` per mod. |
| **Registry Blocchi/Item** | Parziale | `horizon-world/src/registry.rs`, `horizon-items/` | Registri immutabili e baked (`DATA_VERSION 5023`); sufficiente in sola lettura per mod client. |
| **Config & Persistenza** | Parziale | `options_file.rs`, `native_hooks.rs` | File vanilla presenti; serve astrazione serde standard per le opzioni delle mod. |
| **Input / Keybinds** | Parziale | `keybinds.rs`, `native_hooks.rs` | Tasti vanilla presenti; manca registrazione dinamica di nuovi keybind in UI. |
| **Native Mod Hooks** | Esiste | `horizon-client/src/game/native_hooks.rs` | API v2 con hook minimap e pannelli; espandibile per i nuovi hook del porting. |

