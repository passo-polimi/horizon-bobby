# Architectural Decision Records (ADR) — Horizon Mod Porting

*Generato in Fase 0 (Task T0.2) · Data: 2026-10-30*  
*Stato: In attesa di approvazione umana*

---

## ADR-001: Versione di Minecraft Target (26.3)

### Contesto
Horizon è sviluppato fin dall'inizio come implementazione pulita in Rust di Minecraft: Java Edition 26.3 (`crates/horizon-core/src/version.rs` definisce `VERSION_ID = "26.3"`, `PROTOCOL_VERSION = 777`, `DATA_VERSION = 5023`). Le mod Java originali possono essere state compilate per versioni precedenti (es. 1.21.1 o 1.20.x).

### Decisione
Il target unico per tutti i porting e per `horizon-mod-api` è **Minecraft 26.3**.  
Le mod Java analizzate verranno rimappate e aggiornate concettualmente alla semantica 26.3 (stati dei blocchi, formati dei pacchetti e DataVersion 5023).

### Alternative Scartate
- **Supportare versioni multiple (1.20.1 / 1.21.1):** Scartata perché l'onere di mantenere livelli multipli di astrazione per packet protocol e registry blocchi romperebbe la semplicità del porting nativo. Come stabilito in `docs/version-ports.md`, Horizon supporta una sola versione di Minecraft per release.

---

## ADR-002: Loader di Partenza (Fabric)

### Contesto
Le mod Java client-side sono prevalentemente distribuite per Fabric, NeoForge e Forge. Dobbiamo stabilire quale ecosistema analizzare per primo nel pipeline di ingestion.

### Decisione
Il loader di riferimento primario per l'ingestion e per la modellazione degli eventi in `horizon-mod-api` è **Fabric**.

### Motivazione
1. Fabric adotta un'architettura minimalista con **Fabric API** modulare ed eventi puliti (`ClientTickEvents`, `HudRenderCallback`).
2. I mixin Fabric puntano a classi Java con nomi deoffuscati trasparenti.
3. Consente di mappare facilmente le callback a un trait Rust `HorizonListener<E>`.

### Alternative Scartate
- **NeoForge / LexForge:** Scartate per la prima fase per via dell'elevato accoppiamento con l'EventBus proprietario di Forge, bus di mod separati e annotazioni `@SubscribeEvent` con priorità e cancellabilità complesse.

---

## ADR-003: Namespace e Mappings (Mojmap + Parchment)

### Contesto
Le mod Fabric compilate contengono simboli offuscati o mappati a `intermediary` (es. `net.minecraft.class_310`). Per l'analisi statica e la corrispondenza con Horizon serve un namespace universale.

### Decisione
Tutti i JAR e sorgenti di input verranno rimappati a **Mojang Official Mappings (Mojmap) + Parchment** (per i nomi dei parametri dei metodi) prima di qualunque analisi con ASM o Spoon.

### Motivazione
1. Horizon adotta la nomenclatura Mojang vanilla in tutti i propri sorgenti e commenti di parità (`// LevelChunk.isTicking`, `ClientboundLevelChunkWithLightPacket`).
2. Il tracciamento tramite commento `// PORT: Classe#metodo` (§10 del piano) risulterà direttamente confrontabile con la documentazione e i sorgenti deoffuscati.

### Alternative Scartate
- **Yarn:** Scartata perché i nomi dei metodi e campi differiscono da quelli ufficiali usati nei port interni di Horizon.
- **Intermediary grezzo:** Scartata perché illeggibile per lo sviluppo e per i modelli di AI.

---

## ADR-004: Modello di Estensione e ABI (Crate In-Tree con Tipi Semplici)

### Contesto
Le mod Rust possono essere caricate come librerie dinamiche (.dll/.so a runtime via `libloading`), come componenti WebAssembly (Wasmtime), oppure compilate come crate in-tree nel workspace Cargo di Horizon.

### Decisione
Adottare un'architettura a **Crate In-Tree** posizionati in `crates/mods/<mod>`, dipendenti da `crates/horizon-mod-api`.  
I tipi scambiati ai confini dell'API devono essere **semplici, possibilmente POD o serializzabili con Serde**, senza esporre puntatori raw o layout interni di Horizon, in modo da consentire in futuro una transizione indolore verso componenti WIT/WASM.

### Motivazione
1. Rust non possiede un'ABI stabile tra diverse versioni del compilatore: le DLL dinamiche native causerebbero crash frequenti o richiederebbero una complessa ABI C `extern "C"`.
2. I crate in-tree beneficiano di ottimizzazioni LTO, inlining e controlli di tipo completi a tempo di compilazione (`cargo check`, `cargo test`).
3. L'ecosistema dei mixin nativi di Horizon (`docs/native-mods.md`) opera già compilando bundle nativi in-tree.

### Alternative Scartate
- **Dynamic Linking a runtime (.dll/.so):** Scartata per instabilità ABI e difficoltà di debugging su Windows/Linux.
- **WebAssembly puro (Fase 0-4):** Scartata per le mod con logica grafica avanzata (come LOD), a causa dei limiti di accesso diretto a buffer GPU wgpu e overhead prestazionale.

---

## ADR-005: Toolchain e Requisiti di Sistema

### Contesto
Definire le versioni pinnate degli strumenti di build per garantire la riproducibilità sia lato Rust sia lato Java/JVM.

### Decisione
1. **Rust:**
   - Edizione: `2024` (coerente con il workspace di Horizon).
   - MSRV: `1.85` (come dichiarato in `Cargo.toml`).
   - Compilatore verificato: `rustc 1.99.1` (già installato e operativo).
2. **JVM (per strumenti di ingestion e test differenziali):**
   - Java 21 LTS (per compatibilità con Minecraft 26.3 e Gradle 8.x).
   - Gradle 8.x con tiny-remapper e Vineflower per deoffuscazione e decompilazione.

### Alternative Scartate
- **Java 17:** Scartata perché Minecraft 26.3 adotta Java 21/25 (`horizon_core::version::JAVA_VERSION = 25`).

---

## ADR-006: Decisione Go / No-Go sui Generatori Automatici di Codice (Fase 5)
- **Data:** 2026-10-30
- **Stato:** Approvato (NO-GO)

### Contesto
Il piano al §8 prevede una valutazione al termine della Fase 4 per stabilire se costruire transpiler/generatori automatici (Fase 5A Dati, 5B Hook, 5C Shader) in base alla regola di break-even (§2):
$$\text{Costo sviluppo generatore} < \text{Ore risparmiate} \times \text{Numero mod da portare}$$

### Decisione
**NO-GO sui generatori automatici.**  
L'approccio assistito a strati (Analisi Statica $\to$ `horizon-mod-api` $\to$ Harness Differenziale) ha dimostrato di:
1. Portare le mod pilota (Nvidium e Distant Horizons) in tempi brevissimi (~1 giorno-agente per l'infrastruttura completa).
2. Raggiungere **0 `todo!` residui** senza generare il debito tecnico o la fragilità di un transpiler AST.
3. Produrre codice Rust nativo idiomatico, performante e allineato alle API wgpu di Horizon anziché tradurre letteralmente le complessità legacy OpenGL/Java.

### Conseguenze
Si passa direttamente alla **Fase 6: Scala**, consolidando la CLI unificata `horizon-port`, le guide operative e la procedura di bump.

