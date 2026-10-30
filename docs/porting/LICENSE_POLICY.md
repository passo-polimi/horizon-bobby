# Policy sulle Licenze per il Porting di Mod Java in Horizon

*Generato in Fase 0 (Task T0.3) · Data: 2026-10-30*  
*Attenzione: questo documento costituisce un'analisi tecnica di conformità e non un parere legale formale.*

---

## 1. Principi Generali di Proprietà Intellettuale e Opere Derivate

1. **Il Porting è un'Opera Derivata:**  
   La traduzione di un programma da un linguaggio a un altro (Java → Rust), sia essa manuale, semi-automatica o generata da modelli di intelligenza artificiale, conserva la struttura concettuale, l'astrazione e la sequenza logica del codice originale. Ai sensi della normativa sul diritto d'autore (copyright), il codice risultante costituisce un'**opera derivata** (*derivative work*).
2. **Vincoli di Licenza dell'Originale:**  
   Il crate Rust derivato deve rispettare integralmente le condizioni della licenza del codice Java di origine. Non è lecito relicenziare arbitrariamente il codice sotto una licenza diversa (es. MIT) se la licenza di partenza impone clausole di copyleft (GPL/LGPL) o vieta la derivazione.
3. **Compatibilità con Horizon (MIT):**  
   Horizon è concesso sotto licenza permissiva **MIT** ([`Cargo.toml`](../../Cargo.toml#L53)).  
   - Crate con licenze permissive (MIT, Apache-2.0, BSD): compatibilità piena, possono essere inclusi in-tree.
   - Crate con licenze copyleft deboli (LGPL-3.0, MPL-2.0): ammissibili come crate separati con propria dichiarazione di licenza `license = "LGPL-3.0"`, lasciando il resto di Horizon sotto MIT.
   - Crate con licenze All Rights Reserved: **categoricamente esclusi** dalla pipeline di ingestione e porting diretto.

---

## 2. Criteri di Classificazione delle Mod

Ogni mod candidata al porting viene categorizzata in uno di tre livelli di idoneità:

| Livello | Descrizione | Azione nel Pipeline |
|---|---|---|
| 🟢 **PORTABILE (Permissiva / LGPL / GPL)** | Codice rilasciato con licenza Open Source (MIT, BSD, Apache 2.0, LGPL v3, GPL v3). | Procedere con ingestione. Il crate eredita la licenza copyleft se applicabile. |
| 🟡 **RICHIEDE PERMESSO** | Licenze personalizzate (es. "non-commercial only", clausole anti-AI, o fork senza licenza chiara). | L'agente si ferma e richiede prova di consenso scritto dell'autore prima dell'ingestione. |
| 🔴 **BLOCCATA (All Rights Reserved)** | Codice protetto da copyright totale o divieto esplicito di redistribuzione/modifica. | **Ingestione vietata.** Nessuna riga di codice Java può essere deoffuscata o portata. |

---

## 3. Classificazione delle Mod Pilota Scelte

### Mod 1: Distant Horizons
- **Repository di riferimento:** `https://gitlab.com/distant-horizons-team/distant-horizons` (GitLab ufficiale)
- **Licenza letta dal repository:** **GNU Lesser General Public License v3.0 (LGPL-3.0)** / **GPL-3.0-or-later**.
- **Classificazione:** 🟢 **PORTABILE (CON VINCOLO COPYLEFT LGPL-3.0)**.
- **Implicazioni legali ed architetturali per Horizon:**
  1. Il crate risultante (`crates/mods/distant-horizons`) dovrà obbligatoriamente indicare `license = "LGPL-3.0-only"` nel proprio `Cargo.toml`.
  2. Il codice sorgente del crate portato deve essere reso disponibile pubblicamente.
  3. Il linking con Horizon (MIT) è compatibile in quanto l'LGPL consente il collegamento con opere sotto altre licenze, a patto che il componente LGPL rimanga modificabile e ricompilabile.

---

### Mod 2: Voxy
- **Repository di riferimento:** `https://github.com/MCRcortex/voxy`
- **Licenza letta da `LICENSE.md` nel repository:**
  ```text
  Copyright 2025 MCRcortex. All rights reserved. Do not redistribute.
  
  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND...
  ```
- **Classificazione:** 🔴 **BLOCCATA (All Rights Reserved / Proprietary Source-Available)**.
- **Implicazioni legali ed architetturali per Horizon:**
  1. **Divieto di Porting Diretto:** È severamente vietato inserire il file JAR o il codice sorgente Java di Voxy nella pipeline di ingestione (`tools/porting/ingest`), deoffuscarlo o decompilarlo per tradurne le classi in Rust.
  2. **Eccezione Clean-Room Implementation:**  
     La normativa sul copyright tutela la specifica forma espressiva del codice sorgente, non i concetti matematici e algoritmici generali (come l'uso di un Sparse Voxel Octree a 32-bit, l'occlusion culling tramite Hierarchical Z-Buffer o le chiamate Vulkan/wgpu Multi-Draw Indirect).  
     Qualora si intenda realizzare un motore voxel simile a Voxy, esso deve essere sviluppato come **opera autonoma e pulita (Clean-Room)**, scrivendo codice Rust originale senza copiare file o metodi proprietari di MCRcortex.
