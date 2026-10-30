# Open Questions — Horizon Mod Porting

*Registro di dubbi e decisioni architetturali del progetto.*

---

## Q-001: Selezione della Mod Pilota GPU-Driven (RISOLTO)
- **Stato:** **RISOLTO**
- **Decisione:** Voxy (All Rights Reserved) è stata sostituita con **Nvidium** (`https://github.com/MCRcortex/nvidium`), creata dallo stesso autore (MCRcortex) ma rilasciata sotto licenza libera **GNU LGPL-3.0-only**. Questo consente l'ingestione e il porting a norma di licenza della tecnologia GPU-driven / Hi-Z / indirect drawing.

---

## Q-002: Separazione Licenze Copyleft (LGPL-3.0) in Horizon
- **Stato:** **CONFERMATO**
- **Decisione:** Entrambi i crate portati (`crates/mods/distant-horizons` e `crates/mods/nvidium`) conterranno nel rispettivo `Cargo.toml` la dicitura `license = "LGPL-3.0-only"`, mantenendo il core di Horizon sotto licenza MIT.
