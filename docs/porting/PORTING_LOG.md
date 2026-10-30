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
