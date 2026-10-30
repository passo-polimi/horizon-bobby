# Validazione Mod Pilota e Baseline Metriche

*Generato in Fase 0 (Task T0.4) · Aggiornato: 2026-10-30*  
*Configurazione adottata: Opzione A (Distant Horizons + Nvidium)*

---

## 1. Validazione delle Mod Scelte

Le due mod pilota approvate per il porting sono:
1. **Distant Horizons**
2. **Nvidium**

Entrambe le mod provengono dall'ecosistema Fabric di Minecraft e affrontano il rendering a lunghissimo raggio con due paradigmi complementari:
- **Distant Horizons:** Approccio CPU/LOD a colonne 2.5D con Quadtree e greedy meshing.
- **Nvidium:** Approccio GPU-driven rendering con culling hardware (Frustum + Hi-Z occlusion culling) e draw call indirette.

### Tabella di Validazione

| Criterio | Distant Horizons | Nvidium |
|---|---|---|
| **Repository Ufficiale** | `gitlab.com/distant-horizons-team/distant-horizons` | `github.com/MCRcortex/nvidium` |
| **Licenza Verificata** | LGPL-3.0 / GPL-3.0 | LGPL-3.0-only |
| **Classificazione Licenza** | 🟢 **PORTABILE (Copyleft LGPL)** | 🟢 **PORTABILE (Copyleft LGPL)** |
| **Dimensione Codice Java** | Molto grande (~80k - 120k LoC) | Media (~15k - 25k LoC) |
| **Complessità Architetturale** | Alta (Mesher CPU, multithreading, persistenza binaria) | Alta (GPU Compute Culling, Hi-Z, Multi-Draw Indirect) |
| **Rischio di Scope Creep** | **Critico** (citato nel §12 del Piano; mitigato da subset mirato) | **Medio** (dipendenza da estensioni wgpu per indirect draw) |
| **Idoneità Porting** | **100% Ammissibile** | **100% Ammissibile** |

---

## 2. Feature List Concordata per il Porting

Per garantire la massima efficacia evitando dispersioni su codebase estese, definiamo il perimetro funzionale esatto (feature list) per la Fase 4:

### Distant Horizons — Feature List (Core 2.5D LOD Engine)
- [ ] **DH-1: Modello Dati Columnar RLE.** Rappresentazione a colonne verticali con compressione run-length per altezze, blockstate e luce nei livelli di dettaglio $L_0$ (1:1), $L_1$ (2:2), $L_2$ (4:4) e $L_3$ (8:8).
- [ ] **DH-2: Persistenza su Disco.** Salvataggio e deserializzazione deterministica dei chunk compressi con LZ4 nel formato `.hlod`.
- [ ] **DH-3: Greedy Meshing 2.5D con Skirt Culling.** Algoritmo di meshing che unisce celle complanari ed erige "skirt" verticali per sigillare le discontinuità tra livelli di dettaglio adiacenti.
- [ ] **DH-4: Frustum Culling Geometrico.** Culling su frustum di vista applicato alle sezioni LOD.
- [ ] **DH-5: Shader WGSL Far-Terrain.** Pipeline di rendering wgpu con dithering Bayer $4\times 4$ per mascherare il confine tra chunk vanilla completi e chunk LOD.

### Nvidium — Feature List (GPU-Driven Rendering Core)
- [ ] **NV-1: Bounding Box & Frustum Culling GPU.** Calcolo delle bounding box delle sezioni di chunk e test di intersezione sul frustum di vista.
- [ ] **NV-2: Hierarchical Z-Buffer (Hi-Z) Occlusion Culling.** Costruzione della piramide di profondità (mipmap della texture di depth con Reverse-Z) e culling dei chunk occlusi.
- [ ] **NV-3: Meshlet Packaging.** Suddivisione e impacchettamento delle sezioni di chunk in cluster compatti di vertici per l'elaborazione su GPU.
- [ ] **NV-4: Multi-Draw Indirect (MDI).** Generazione dei comandi `DrawIndexedIndirect` su buffer GPU per disegnare l'intero mondo visibile in un numero minimo di chiamate.
- [ ] **NV-5: Integrazione RenderPass wgpu.** Sostituzione del rendering vanilla monolitico con il pass indiretto accelerato su wgpu.

---

## 3. Baseline di Metriche per la Valutazione

Come richiesto dal §3 (T0.4) e §8 (Fase 5 Go/No-Go), registriamo la baseline di metriche per valutare l'efficacia del processo di porting:

| Metrica | Obiettivo Distant Horizons | Obiettivo Nvidium | Metodo di Misura |
|---|---|---|---|
| **Righe di codice Rust stimate** | ~2.500 - 3.500 LoC | ~2.000 - 3.000 LoC | `tokei` o `cloc` sui crate in `crates/mods/` |
| **Percentuale scritta da agente AI** | $\ge 90\%$ | $\ge 90\%$ | Tracciamento commit e prompt |
| **Numero di `todo!("PORT: ...")` residui** | $\le 10$ (giustificati in report) | $\le 10$ (giustificati in report) | `grep -rn "todo!(\"PORT:"` |
| **Gap API di Horizon aperti** | Registrati in `API_GAPS.md` | Registrati in `API_GAPS.md` | Conteggio voci in `docs/porting/API_GAPS.md` |
| **Copertura di test (Unit / Difftest)** | 100% test verdi sui moduli core | 100% test verdi sui moduli core | `cargo test` in `verify.sh` |
| **Impatto prestazionale** | Frame time $\le 2$ ms per LOD pass | $\ge 120$ FPS a 64+ chunk render distance | GPU timer (`Renderer.gpu_times`) |
