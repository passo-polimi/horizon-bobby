# Census e Punteggio di Portabilità: Nvidium

*Generato in Fase 1 (Task T1.4) · Data: 2026-10-30*  
*Basato sull'analisi statica di `work/nvidium/mod-manifest.json`*

---

## 1. Dati Statistici Generali di Nvidium

- **Mod ID:** `nvidium`
- **Licenza:** `LGPL-3.0`
- **File Java analizzati:** 67
- **Mixins totali:** 28 (Inject: 12, Redirect: 9, Accessor: 4, Invoker: 3)
- **Shader GLSL rilevati:** 19 (mesh shaders `mesh.glsl`, task shaders `task.glsl`, compute shaders `region_section_sorter.comp`, fragment shaders `frag.frag`)
- **Fabric Callbacks rilevati:** 0 (si interfaccia direttamente con Sodium e con il rendering engine vanilla)
- **Concorrenza / Threading:** Bassa/Media (worker threads di meshing, offload su GPU)
- **Uso di Unsafe:** Presente (`sun.misc.Unsafe` per allocazione diretta di memory buffer off-heap, mappabile a normali buffer Rust / wgpu)

---

## 2. Valutazione delle Feature da Portare (NV-1 .. NV-5)

| Feature | Titolo | Punteggio | Stima Sforzo | Rischio Principale | Mitigazione |
|---|---|:---:|---|---|---|
| **NV-1** | Bounding Box & Frustum Culling | 🟢 **VERDE** | 1-2 giorni | Overhead CPU su decine di migliaia di sezioni | Implementazione SIMD in Rust su coordinate camera-relative o compute shader. |
| **NV-2** | Hi-Z Occlusion Culling | 🟡 **GIALLO** | 3-4 giorni | Gestione mipmap depth buffer con Reverse-Z su wgpu | Pipeline compute dedicata in WGSL con texture di profondità downsampled. |
| **NV-3** | Meshlet Packaging | 🟢 **VERDE** | 2-3 giorni | Formato dati vertici compresso e allineamento memoria | Struct `#[repr(C)]` con vertici compatti a 32/64 bit (`bytemuck`). |
| **NV-4** | Multi-Draw Indirect (MDI) | 🟢 **VERDE** | 2 giorni | Supporto hardware delle chiamate indirette | `wgpu::RenderPass::multi_draw_indexed_indirect` (supportato su Vulkan, Metal e DX12). |
| **NV-5** | Integrazione RenderPass wgpu | 🟡 **GIALLO** | 2-3 giorni | Sostituzione/aggancio alla pipeline terrain vanilla di Horizon | Hook in `crates/horizon-client-render/src/gfx.rs` o modalità alternativa di rendering. |

### Componenti Dipendenti da Sodium (Valutati 🔴 ROSSO)
- **Sodium Mixins interni:** 🔴 **ROSSO**. I mixin su `me.jellysquid.mods.sodium.client.*` sono scartati in quanto Horizon non usa Sodium né il suo formato interno di meshing, ma dispone già del proprio mesher nativo in `crates/horizon-client-render/src/mesher.rs`.

---

## 3. Sintesi e Raccomandazione per la Fase 2

Nvidium è un progetto estremamente compatto e mirato (solo 67 file Java contro gli oltre 360 di Distant Horizons).  
Il 90% della complessità di Nvidium risiede nella corretta orchestrazione dei buffer GPU e nell'occlusion culling Hi-Z. Poiché Horizon opera già nativamente su `wgpu 30` (con supporto diretto a storage buffer e chiamate indirette), l'architettura di Nvidium è **architetturalmente affine a Horizon** ed eviterà il collo d'impiego su CPU.  
Sforzo stimato: circa **9-14 giorni-agente**.
