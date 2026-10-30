# Validazione Mod Pilota e Baseline Metriche

*Generato in Fase 0 (Task T0.4) · Data: 2026-10-30*

---

## 1. Validazione delle Mod Scelte

L'utente ha selezionato come mod pilota:
1. **Distant Horizons**
2. **Voxy**

Entrambe le mod operano nell'ambito del rendering Level-of-Detail (LOD) a lunghissimo raggio per Minecraft.

### Tabella di Validazione

| Criterio | Distant Horizons | Voxy |
|---|---|---|
| **Repository Ufficiale** | `gitlab.com/distant-horizons-team/distant-horizons` | `github.com/MCRcortex/voxy` |
| **Licenza Verificata** | LGPL-3.0 / GPL-3.0 | All Rights Reserved (Non redistribuibile) |
| **Classificazione Licenza** | 🟢 **PORTABILE (Copyleft LGPL)** | 🔴 **BLOCCATA per porting diretto** (Ammissibile solo Clean-Room) |
| **Dimensione Codice Java** | Molto grande (~80k - 120k LoC) | Media (~30k LoC) |
| **Complessità Architetturale** | Alta (Mesher CPU, multithreading, persistenza binaria) | Molto alta (GPU Compute Hi-Z, SVO 3D, Multi-Draw Indirect) |
| **Rischio di Scope Creep** | **Critico** (esplicitamente citato come rischio nel §12 del Piano) | **Critico** (vincolo legale + dipendenza da estensioni GPU wgpu) |
| **Strategia Adottata** | Porting modulare di un **core subset** ben delimitato | **Clean-Room Re-engineering** delle specifiche matematico-algoritmiche |

---

## 2. Feature List Concordata per il Porting

Per evitare il fallimento del progetto dovuto a scope creep su codebase mastodontiche, definiamo il perimetro funzionale esatto (feature list) da portare e verificare:

### Distant Horizons — Feature List Minima (Core LOD Engine)
- [ ] **DH-1: Modello Dati Columnar RLE.** Rappresentazione a colonne verticali con compressione run-length per altezze, blockstate e luce nei livelli di dettaglio $L_0$ (1:1), $L_1$ (2:2), $L_2$ (4:4) e $L_3$ (8:8).
- [ ] **DH-2: Persistenza su Disco.** Salvataggio e deserializzazione deterministica dei chunk compressi con LZ4 nel formato `.hlod`.
- [ ] **DH-3: Greedy Meshing 2.5D con Skirt Culling.** Algoritmo di meshing che unisce celle complanari ed erige "skirt" verticali per sigillare le discontinuità tra livelli di dettaglio adiacenti.
- [ ] **DH-4: Frustum Culling Geometrico.** Culling su frustum di vista applicato alle sezioni LOD.
- [ ] **DH-5: Shader WGSL Far-Terrain.** Pipeline di rendering wgpu con dithering Bayer $4\times 4$ per mascherare il confine tra chunk vanilla completi e chunk LOD.

### Voxy — Feature List Minima (Clean-Room Voxel Engine)
- [ ] **VX-1: Voxel Packing 32-bit & Brick Structure.** Definizione di `PackedVoxel` (32 bit: 16 bit block ID, 4 bit sky light, 4 bit block light, flag) aggregati in cubi brick da $8^3$ voxel.
- [ ] **VX-2: Sparse Voxel Octree (SVO).** Struttura gerarchica a ottante con mipmapping tridimensionale 3D per chunk e macro-aree.
- [ ] **VX-3: 3D Ambient Occlusion.** Calcolo deterministico dell'AO sui 26 vicini a livello di voxel.
- [ ] **VX-4: Hierarchical Z-Buffer (Hi-Z) Culling.** Algoritmo di occlusion culling su piramide di profondità GPU (con supporto Reverse-Z).
- [ ] **VX-5: Multi-Draw Indirect (MDI).** Generazione buffer indiretto per il disegno efficiente in un'unica dispatch call GPU delle facce visibili compresse a 64 bit (`PackedVoxelFace`).

---

## 3. Baseline di Metriche per la Valutazione

Come richiesto dal §3 (T0.4) e §8 (Fase 5 Go/No-Go), registriamo la baseline di metriche per valutare l'efficacia del processo di porting e l'eventuale introduzione di generatori automatici:

| Metrica | Obiettivo Distant Horizons | Obiettivo Voxy | Metodo di Misura |
|---|---|---|---|
| **Righe di codice Rust stimate** | ~2.500 - 3.500 LoC | ~2.000 - 3.000 LoC | `tokei` o `cloc` sui crate `crates/mods/` |
| **Percentuale scritta da agente AI** | $\ge 90\%$ | $\ge 90\%$ | Tracciamento commit e prompt |
| **Numero di `todo!("PORT: ...")` residui** | $\le 10$ (giustificati in report) | $\le 10$ (giustificati in report) | `grep -rn "todo!(\"PORT:"` |
| **Gap API di Horizon aperti** | Registrati in `API_GAPS.md` | Registrati in `API_GAPS.md` | Conteggio voci in `docs/porting/API_GAPS.md` |
| **Copertura di test (Unit / Difftest)** | 100% test verdi sui moduli core | 100% test verdi sui moduli core | `cargo test` in `verify.sh` |
| **Impatto prestazionale** | Frame time $\le 2$ ms per LOD pass | Frame time $\le 2$ ms per LOD pass | GPU timer (`Renderer.gpu_times`) |
