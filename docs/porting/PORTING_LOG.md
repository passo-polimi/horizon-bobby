# Porting Log

*Registro cronologico delle attività di porting, ingressi, modelli e file toccati (Regola 8).*

---

## [2026-10-30] Fase 0 — Audit, Architettura e Policy Licenze
- **Modello:** Antigravity Lead Agent (Google DeepMind)
- **Task eseguiti:**
  - `T0.1`: Audit completo dei sottosistemi di Horizon con percorsi reali verificati.
  - `T0.2`: Redazione delle decisioni architetturali (ADR-001 .. ADR-005).
  - `T0.3`: Redazione della policy sulle licenze e classificazione delle mod pilota:
    - Distant Horizons: LGPL-3.0 (🟢 Portabile).
    - Voxy: All Rights Reserved (🔴 Bloccata per porting diretto). Sostituita con **Nvidium** (LGPL-3.0, 🟢 Portabile, dello stesso autore).
  - `T0.4`: Validazione delle due mod pilota (Distant Horizons + Nvidium), feature list mirata e definizione baseline di metriche.
- **Hash degli input:**
  - `Piano di implementazione porting mod Java in Horizon (Rust).md`: `SHA256: 941D6B6A123491CA59AA0FD60BC7928A1DC47B781F43173A9F5EDA13D2B8BAD9`
  - Base commit Horizon: `4f3457a`
- **File creati / modificati:**
  - `docs/porting/PLAN.md`
  - `docs/porting/horizon-audit.md`
  - `docs/porting/DECISIONS.md`
  - `docs/porting/LICENSE_POLICY.md`
  - `docs/porting/pilot-baseline.md`
  - `docs/porting/API_GAPS.md`
  - `docs/porting/OPEN_QUESTIONS.md`
  - `docs/porting/PORTING_LOG.md`
- **Esito:** Tutti i criteri di accettazione della Fase 0 sono soddisfatti e verificati con Opzione A approvata dall'utente. Nessun codice di porting scritto in questa fase.


## [2026-10-30] Fase 1 — Ingestion e Analisi Statica
- **Modello:** Antigravity Lead Agent (Google DeepMind)
- **Task eseguiti:**
  - `T1.1`: Costruzione di `tools/porting/ingest.py` (con wrapper `.bat` e bash) e acquisizione dei repository ufficiali delle due mod pilota:
    - Nvidium: `commit f2028b2ba9a5dc95d69e73a9eaeadcb447367d7d`
    - Distant Horizons: `commit bbf5730983ac5845abc2749df829f6a8213716a0`
  - `T1.2`: Estrazione metadati da `fabric.mod.json`, `*mixins.json`, access wideners e asset in `work/<mod>/meta/`.
  - `T1.3`: Analisi statica completa e generazione dei manifest:
    - Nvidium: 67 classi Java, 28 mixin, 19 shader GLSL, 0 callback Fabric.
      - Hash manifest SHA-256: `8bf12fbc1143c0ed998c0f38c2ac4677d6038d6f432c135386023d9fe261fefb`
    - Distant Horizons: 366 classi Java, 101 mixin, 0 shader, 6 callback Fabric.
      - Hash manifest SHA-256: `f6ddc7c0d3c71db53b07abf47120ace7cbbdc1b621d1f4d58e9eccf62e3b3f4b`
    - Entrambi i manifest validano con successo contro `tools/porting/manifest_schema.json`.
  - `T1.4`: Punteggio di portabilità (Verde/Giallo/Rosso) per feature, stima dello sforzo e mitigazione rischi in:
    - `docs/porting/census-distant-horizons.md`
    - `docs/porting/census-nvidium.md`
- **File creati / modificati:**
  - `tools/porting/ingest.py`
  - `tools/porting/ingest.bat`
  - `tools/porting/ingest`
  - `tools/porting/manifest_schema.json`
  - `docs/porting/census-distant-horizons.md`
  - `docs/porting/census-nvidium.md`
  - `docs/porting/PORTING_LOG.md`
  - `.gitignore`
- **Esito:** Fase 1 completata con successo, criteri di accettazione verificati. Attività fermata in attesa di approvazione umana prima della Fase 2.
