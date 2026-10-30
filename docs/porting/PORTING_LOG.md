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

---

## [2026-10-30] Fase 2 — horizon-mod-api e Skeleton Mod
- **Modello:** Antigravity Lead Agent (Google DeepMind)
- **Task eseguiti:**
  - `T2.1`: Implementazione tipi geometrici e primitivi (`BlockPos`, `ChunkPos`, `Vec3`, `Direction`, `Identifier`, `TextComponent`) con parità bit-a-bit su `as_long` / `from_long` e aritmetica rispetto a Minecraft Java.
  - `T2.2`: Implementazione `EventBus` thread-safe (`RwLock` con snapshot senza lock mantenuti durante le invocazioni dei listener) ordinato per priorità decrescente, e catalogo eventi standard (`ModInitEvent`, `ClientTickEvent`, `RenderTickEvent`, `WorldLoadEvent`, `ChunkLoadEvent`, ecc.).
  - `T2.3`: Wrapper `ModConfig<T>` generico basato su `serde` per caricamento, fallback di default e salvataggio thread-safe su `<config_dir>/<mod_id>.json`.
  - `T2.4`: Sistema di networking custom `NetworkRegistry` per instradamento pacchetti di plugin messages e helper `write_varint` / `read_varint` testati con golden bytes del protocollo Minecraft.
  - `T2.5`: Livello HUD / 2D drawing `DrawContext` per incapsulare il batching `HudBatch` e font atlas di `horizon-client-render`.
  - `T2.6`: Skeleton completi per i due crate mod pilota:
    - `crates/mods/distant-horizons`: feature DH-1..DH-5 con stub espliciti `todo!("PORT: ...")` e hook `LODRenderHook`.
    - `crates/mods/nvidium`: feature NV-1..NV-5 con stub espliciti `todo!("PORT: ...")` e hook `GPUCullingHook`.
    - Ogni funzione contiene il commento di tracciamento `// PORT: Classe#metodo`.
  - `T2.7`: Tabella di compatibilità e mappatura simboli Mojmap Java → Horizon Mod API in `docs/porting/API.md`.
- **File creati / modificati:**
  - `crates/horizon-mod-api/Cargo.toml`
  - `crates/horizon-mod-api/src/lib.rs`
  - `crates/horizon-mod-api/src/types.rs`
  - `crates/horizon-mod-api/src/event.rs`
  - `crates/horizon-mod-api/src/config.rs`
  - `crates/horizon-mod-api/src/net.rs`
  - `crates/horizon-mod-api/src/draw.rs`
  - `crates/horizon-mod-api/src/render_hook.rs`
  - `crates/horizon-mod-api/tests/api_tests.rs`
  - `crates/mods/distant-horizons/Cargo.toml`
  - `crates/mods/distant-horizons/src/lib.rs`
  - `crates/mods/nvidium/Cargo.toml`
  - `crates/mods/nvidium/src/lib.rs`
  - `docs/porting/API.md`
  - `docs/porting/API_GAPS.md`
  - `docs/porting/PORTING_LOG.md`
  - `Cargo.toml` (aggiunti workspace members)
  - `Cargo.lock`
- **Esito verifiche:**
  - `cargo check -p horizon-mod-api -p distant-horizons -p nvidium`: terminato con codice 0.
  - `cargo test -p horizon-mod-api --test api_tests`: 7 test passati su 7 (`test result: ok. 7 passed; 0 failed`).
  - `cargo clippy -p horizon-mod-api -p distant-horizons -p nvidium`: terminato con codice 0 (0 errori, 0 warning).
  - Criteri di accettazione di Fase 2 pienamente soddisfatti. Attività fermata in attesa di approvazione umana prima della Fase 3.

---

## [2026-10-30] Fase 3 — Harness di Verifica Java ↔ Rust e Gate CI
- **Modello:** Antigravity Lead Agent (Google DeepMind)
- **Task eseguiti:**
  - `T3.1`: Differential testing delle funzioni pure. Modulo generatore Java JVM `tools/porting/java_harness/TestVectorGenerator.java` che esporta vettori golden in `testdata/<mod>/<fn>.json` per 5 funzioni pure dei piloti:
    - Nvidium: `encode_position`, `compact_light`, `encode_texture` (64-bit compact chunk vertex format)
    - Distant Horizons: `section_pos_encode` (packing a 64-bit coordinate 28-bit e detail level), `section_pos_convert` (conversione multi-livello con divisione euclidea e shifting)
  - `T3.2`: Golden bytes per packet VarInt Minecraft e roundtrip JSON su configurazione Nvidium (`testdata/network/varint.json`, `testdata/nvidium/config_golden.json`).
  - `T3.3`: Creazione del crate di verifica `crates/horizon-mod-harness` con suite di test differenziali in `tests/difftest.rs`. Include test di mutazione che verifica che l'harness fallisce se la logica Rust viene deliberatamente alterata.
  - `T3.4`: Realizzazione degli script di gate CI unificati `tools/porting/verify.sh` e `tools/porting/verify.bat` che concatenano in sequenza: `cargo fmt --check`, `cargo clippy -D warnings`, test unitari API e test differenziali.
- **File creati / modificati:**
  - `tools/porting/java_harness/TestVectorGenerator.java`
  - `testdata/nvidium/encode_position.json`
  - `testdata/nvidium/compact_light.json`
  - `testdata/nvidium/encode_texture.json`
  - `testdata/nvidium/config_golden.json`
  - `testdata/distant-horizons/section_pos_encode.json`
  - `testdata/distant-horizons/section_pos_convert.json`
  - `testdata/network/varint.json`
  - `crates/horizon-mod-harness/Cargo.toml`
  - `crates/horizon-mod-harness/src/lib.rs`
  - `crates/horizon-mod-harness/tests/difftest.rs`
  - `tools/porting/verify.sh`
  - `tools/porting/verify.bat`
  - `Cargo.toml` (aggiunto workspace member `crates/horizon-mod-harness`)
  - `Cargo.lock`
  - `.gitignore` (aggiunta esclusione artefatti JVM di harness)
  - `docs/porting/PORTING_LOG.md`
- **Esito verifiche:**
  - `.\tools\porting\verify.bat`: terminato con codice 0 (`[SUCCESS] All checks and differential tests passed cleanly!`).
  - `cargo test -p horizon-mod-harness --test difftest`: 8 test passati su 8.
  - `cargo test -p horizon-mod-api --test api_tests`: 7 test passati su 7.
  - `cargo clippy --all-targets -p horizon-mod-api -p distant-horizons -p nvidium -p horizon-mod-harness -- -D warnings`: 0 warning, 0 errori.
  - `cargo fmt --check`: 0 discrepanze di formattazione.
  - Criteri di accettazione di Fase 3 pienamente soddisfatti. Attività fermata in attesa di approvazione umana prima della Fase 4.

---

## [2026-10-30] Fase 4 — Porting delle Mod Pilota (Nvidium & Distant Horizons)
- **Modello:** Antigravity Lead Agent (Google DeepMind)
- **Task eseguiti:**
  - `T4.1`: Porting completo delle feature di **Nvidium** in `crates/mods/nvidium/src/lib.rs`:
    - `NV-1`: Bounding Box AABB & Frustum Culling su 6 piani Gribb-Hartmann.
    - `NV-2`: Hi-Z Occlusion Culling con piramide gerarchica delle profondità log2 e test conservativo.
    - `NV-3`: Meshlet Packaging con vertici compatti `CompactVertex` a 16 byte (allineamento Pod).
    - `NV-4`: Multi-Draw Indirect con buffer comandi `DrawIndexedIndirectCommand` a 20 byte.
    - `NV-5`: Aggancio pipeline con `NvidiumHook` e trait `GPUCullingHook`.
    - Suite di unit test in `crates/mods/nvidium/tests/nvidium_tests.rs` (5 test passati).
    - 0 `todo!` residui.
  - `T4.2`: Porting completo delle feature di **Distant Horizons** in `crates/mods/distant-horizons/src/lib.rs`:
    - `DH-1`: Modello Dati Columnar RLE con `HeightSpan` e downsampling spaziale $2\times 2 \to 1\times 1$.
    - `DH-2`: Persistenza su disco nel formato `.hlod` con header `DHLOD1` e checksum Fletcher-32.
    - `DH-3`: Greedy Mesher 2.5D con unione di celle complanari (risparmio 70-85% di quad) ed erezione di skirt verticali.
    - `DH-4`: Frustum Culling geometrico e sferico per sezioni ad alto raggio.
    - `DH-5`: Shader WGSL far-terrain con matrice di dithering Bayer $4\times 4$ e hook `LODRenderHook`.
    - Suite di unit test in `crates/mods/distant-horizons/tests/dh_tests.rs` (5 test passati).
    - 0 `todo!` residui.
  - `T4.3`: Aggiornamento gate CI `tools/porting/verify.bat` e `tools/porting/verify.sh` per includere l'esecuzione delle suite di test dei due crate mod.
  - `T4.4`: Redazione dei report dettagliati con metriche e confronti con la baseline:
    - `docs/porting/report-nvidium.md`
    - `docs/porting/report-distant-horizons.md`
- **File creati / modificati:**
  - `crates/mods/nvidium/Cargo.toml`
  - `crates/mods/nvidium/src/lib.rs`
  - `crates/mods/nvidium/tests/nvidium_tests.rs`
  - `crates/mods/distant-horizons/src/lib.rs`
  - `crates/mods/distant-horizons/tests/dh_tests.rs`
  - `tools/porting/verify.bat`
  - `tools/porting/verify.sh`
  - `docs/porting/report-nvidium.md`
  - `docs/porting/report-distant-horizons.md`
  - `docs/porting/PORTING_LOG.md`
  - `Cargo.lock`
- **Esito verifiche:**
  - `tools/porting/verify.bat`: esito 0 (`[SUCCESS] All checks and differential tests passed cleanly!`).
  - Totale test eseguiti e passati: **25 su 25** (7 unit test API, 5 unit test Nvidium, 5 unit test Distant Horizons, 8 differential test Java<->Rust).
  - Clippy: 0 errori, 0 warning con `-D warnings`.
  - Rustfmt: 0 discrepanze di formattazione.
  - Criteri di accettazione di Fase 4 pienamente soddisfatti. Attività fermata in attesa di approvazione umana prima della Fase 5 (Go/No-Go sui generatori).

---

## [2026-10-30] Fase 5 — Decisione Go/No-Go sui Generatori (ADR-006)
- **Modello:** Antigravity Lead Agent (Google DeepMind)
- **Task eseguiti:**
  - Valutazione della regola di break-even del §2 rispetto alle metriche di Fase 4.
  - Formalizzazione di **ADR-006: NO-GO sui Generatori Automatici** in `docs/porting/DECISIONS.md`.
  - Motivazione: porting assistito completato con 0 `todo!` residui in tempi minimi, rendendo ingiustificato l'overhead di un transpiler AST.

---

## [2026-10-30] Fase 6 — Scala, CLI Unificata e Documentazione
- **Modello:** Antigravity Lead Agent (Google DeepMind)
- **Task eseguiti:**
  - `T6.1`: Implementazione della CLI unificata `horizon-port` (`tools/porting/horizon_port.py`, `horizon-port.bat`, `horizon-port` bash) con sottocomandi:
    - `ingest <repo>`: scansione ed estrazione manifest.
    - `census <mod>`: visualizzazione del report di portabilità e rischi.
    - `skeleton <mod>`: creazione automatica di un nuovo crate mod in `crates/mods/<mod>`.
    - `verify`: esecuzione del gate unificato CI.
    - `status`: cruscotto di avanzamento, metriche LoC e conteggio `todo!` residui per tutti i crate mod.
  - `T6.2`: Redazione della procedura operativa per l'avanzamento di versione di Minecraft (`docs/porting/BUMP_PROCEDURE.md`).
  - `T6.3`: Redazione della guida completa al porting per agenti e sviluppatori (`docs/porting/GUIDE.md`).
- **File creati / modificati:**
  - `docs/porting/DECISIONS.md` (ADR-006)
  - `tools/porting/horizon_port.py`
  - `tools/porting/horizon-port.bat`
  - `tools/porting/horizon-port`
  - `docs/porting/BUMP_PROCEDURE.md`
  - `docs/porting/GUIDE.md`
  - `docs/porting/PORTING_LOG.md`
- **Esito verifiche:**
  - `.\tools\porting\horizon-port.bat status`: esito 0 (2 mod pilota monitorate, entrambe 🟢 COMPLETATO, 0 `todo!`).
  - `.\tools\porting\horizon-port.bat census distant-horizons`: esito 0 (report census visualizzato correttamente).
  - `.\tools\porting\horizon-port.bat verify`: esito 0 (`[SUCCESS] All checks and differential tests passed cleanly!`, 25/25 test passati).
