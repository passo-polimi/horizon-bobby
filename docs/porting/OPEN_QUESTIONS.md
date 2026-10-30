# Open Questions — Horizon Mod Porting

*Registro di dubbi e decisioni non bloccanti da chiarire con il supervisore umano.*

---

## Q-001: Strategia Definitiva su Voxy (All Rights Reserved)
- **Contesto:** Voxy ha una licenza esplicita `All rights reserved. Do not redistribute.`. Questo impedisce legalmente l'ingestione e la decompilazione del suo JAR/sorgente Java nel pipeline automatico `tools/porting/ingest`.
- **Domanda per l'utente:** Si approva la strategia Clean-Room, ovvero costruire il crate `crates/mods/voxy` come implementazione autonoma e pulita delle specifiche algoritmiche (SVO 3D + Hi-Z wgpu), senza mai decompilare o copiare classi originali dal repository di MCRcortex?

---

## Q-002: Separazione di Distant Horizons per Requisiti LGPL
- **Contesto:** Distant Horizons è concesso sotto LGPL-3.0. Horizon è interamente sotto licenza permissiva MIT.
- **Domanda per l'utente:** Si conferma che il crate `crates/mods/distant-horizons` manterrà la propria licenza LGPL-3.0 isolata, documentando chiaramente nel README di Horizon che tale componente è concesso sotto LGPL-3.0?
