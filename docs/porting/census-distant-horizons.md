# Census e Punteggio di Portabilità: Distant Horizons

*Generato in Fase 1 (Task T1.4) · Data: 2026-10-30*  
*Basato sull'analisi statica di `work/distant-horizons/mod-manifest.json`*

---

## 1. Dati Statistici Generali di Distant Horizons

- **Mod ID:** `distanthorizons`
- **Licenza:** `LGPL-3.0`
- **File Java analizzati:** 366
- **Mixins totali:** 101 (Inject: 54, Redirect: 28, Accessor: 12, ModifyVariable: 7)
- **Fabric Callbacks rilevati:** `ClientTickEvents.START_CLIENT_TICK`, `ClientTickEvents.END_CLIENT_TICK`, `WorldRenderEvents.AFTER_TRANSLUCENT`
- **Concorrenza / Threading:** Elevata (`CompletableFuture`, `ExecutorService`, `Thread`, `ThreadPoolExecutor`, blocchi `synchronized`)
- **Reflection / Unsafe:** Presente (`Class.forName`, `MethodHandles`, `Unsafe`, `setAccessible`)

---

## 2. Valutazione delle Feature da Portare (DH-1 .. DH-5)

| Feature | Titolo | Punteggio | Stima Sforzo | Rischio Principale | Mitigazione |
|---|---|:---:|---|---|---|
| **DH-1** | Modello Dati Columnar RLE | 🟢 **VERDE** | 1-2 giorni | Complessità negli indici verticali Y (-64..320) | Uso di array piatti compatti con primitive Rust tipizzate (`u16`/`u32`). |
| **DH-2** | Persistenza su Disco (.hlod) | 🟢 **VERDE** | 1-2 giorni | Corruzione file LZ4 o determinismo I/O | Crate `lz4_flex` e struct serializzabili con `bincode`/`serde`. |
| **DH-3** | Greedy Mesher 2.5D & Skirt | 🟡 **GIALLO** | 3-4 giorni | Cucitura delle giunzioni tra livelli di dettaglio adiacenti | Algoritmo 2D greedy collaudato con skirt verticali lungo i bordi esterni. |
| **DH-4** | Frustum Culling Geometrico | 🟢 **VERDE** | 1 giorno | Precisione a grandi coordinate | Uso di coordinate relative alla camera (`dvec3` origin) e glam plane culling. |
| **DH-5** | Shader WGSL Far-Terrain | 🟡 **GIALLO** | 2-3 giorni | Assenza di hook per render pass custom in Horizon (GAP-001) | Introduzione del pass LOD all'interno della catena del renderer o via estensione wgpu. |

### Feature Escluse dalla Prima Versione (Valutate 🔴 ROSSO)
- **GUI Options a schede custom:** 🔴 **ROSSO**. DH include una massiccia interfaccia Java Swing/custom con centinaia di slider. *Decisione:* configurazione iniziale gestita tramite file JSON/RON (`options.json`), rimandando la GUI complessa.
- **Supporto Shaderpack Iris/OptiFine a runtime:** 🔴 **ROSSO**. Troppo dipendente dalle pipeline GLSL dinamiche di Iris. *Decisione:* uso esclusivo della pipeline nativa WGSL di Horizon.

---

## 3. Sintesi e Raccomandazione per la Fase 2

Il core algoritmico di Distant Horizons (DH-1..DH-4) è **altamente portabile** e non dipende strettamente dai 101 mixin di compatibilità di Java (la maggior parte dei quali serve solo per iniettarsi nel motore grafico legacy di Minecraft Java).  
In Horizon, il rendering è già pulito e centralizzato su `wgpu`: il porting del core richiederà circa **8-12 giorni-agente** concentrati sui moduli dati e meshing.
