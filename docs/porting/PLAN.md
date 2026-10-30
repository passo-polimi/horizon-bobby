# Piano di implementazione: porting di mod Java in Horizon (Rust)

*Versione 1 · calibrato su un porting di poche mod alla volta · per gli agenti di Antigravity e per chi li supervisiona*

## 1. Obiettivo e principi

L’obiettivo è costruire l’infrastruttura minima per portare un piccolo numero di mod Java client-side (Fabric per prima) in crate Rust nativi per Horizon, in modo ripetibile e verificabile. Non esiste un compilatore Java→Rust magico: l’infrastruttura è a strati (analisi statica, API di compatibilità, generatori mirati, test) e sulla logica di calcolo resta necessario l’intervento di un agente o di una persona.

Principi non negoziabili:

- **API-first.** Il valore principale è una `horizon-mod-api` ben progettata, non i generatori. I generatori si costruiscono solo quando il loro costo si ripaga (vedi §2).
- **Fail loudly.** Tutto ciò che esce dal sottoinsieme supportato produce `todo!("PORT: …")` e una riga nel report di porting. Mai approssimazioni silenziose.
- **Nessuna invenzione.** Se l’API di Horizon non offre qualcosa, l’agente lo annota in `docs/porting/API_GAPS.md`; non lo inventa e non lo simula.
- **Namespace unico.** Mojmap + Parchment per tutti i simboli Minecraft; ogni jar viene rimappato prima di qualunque analisi.
- **Gate licenze.** Una mod entra nella pipeline solo se la sua licenza lo consente (Fase 0).
- **Ogni fase termina con un gate:** criteri di accettazione verificati con comandi, report scritto e approvazione umana prima di proseguire.

## 2. Percorso snello o completo

Con due o tre mod il costo di un generatore (progettarlo, testarlo, mantenerlo) supera quasi sempre il costo di far scrivere il codice a un agente con buoni test. Regola di break-even: costruisci un generatore solo se **(a)** almeno tre mod ne beneficiano, oppure **(b)** copre più del 30% del codice di una mod ed è puramente meccanico.

Percorso consigliato ora: Fasi 0 → 1 → 2 → 3 → 4, poi go/no-go sulla Fase 5. Durata indicativa 8-12 settimane con 1-2 persone più agenti; sono stime grezze, ricalibrale dopo la Fase 0.

Nel concreto, con due mod: niente data transpiler (struct e DTO si scrivono con l’agente, supportati da test), niente hook generator (gli eventi si collegano a mano nel crate della mod), shader pipeline solo come script con `naga` se una delle due mod ha shader.

## 3. Fase 0 — Audit e decisioni (\~1 settimana)

**T0.1 — Audit di Horizon.** Mappa i sottosistemi: pipeline di rendering wgpu e come si registrano pass e shader, loop di tick ed eventi, networking e protocollo, UI/HUD, registry di blocchi e item, config, input e keybind, organizzazione dei crate. Output: `docs/porting/horizon-audit.md`, con per ogni area lo stato (esiste / parziale / manca) e i punti di estensione. *Accettazione:* ogni affermazione cita un percorso di file reale; nessuna area lasciata a «presumibilmente».

**T0.2 — Decisioni architetturali** in `docs/porting/DECISIONS.md` (formato ADR, con alternative scartate): versione di Minecraft target; loader di partenza (Fabric); mappings (Mojmap + Parchment); modello di estensione (crate in-tree, con tipi semplici e serializzabili ai confini dell’API per poter passare in futuro a WIT/WASM); toolchain (edizione e MSRV di Rust, Java 21 e Gradle per gli strumenti lato JVM). *Accettazione:* ogni decisione ha motivazione e alternative; approvata da un umano.

**T0.3 — Policy licenze.** `docs/porting/LICENSE_POLICY.md` e classificazione di ogni mod pilota: portabile, richiede permesso dell’autore, oppure bloccata (All Rights Reserved). Un porting è un’opera derivata: con licenze GPL il crate risultante eredita la licenza, quindi va verificata la compatibilità con quella di Horizon. Non è consulenza legale. *Accettazione:* la licenza è letta dal repository o dal jar, non dedotta; se non è portabile, l’agente si ferma e chiede.

**T0.4 — Pilota e baseline.** Due mod client-side, piccole, possibilmente una HUD/QoL e una con config e rete (o shader). Se le mod sono già scelte, il task si riduce a validarle (licenza, dimensione, fattibilità). Per ciascuna, elenco concordato delle feature da portare. Metriche: ore umane, % di righe scritte dall’agente, numero di `todo!`, numero di gap API, test passati. *Accettazione:* feature list e baseline firmate prima di iniziare la Fase 4.

## 4. Fase 1 — Ingestion e analisi statica (\~1-2 settimane)

**T1.1 — Acquisizione.** `tools/porting/ingest`: dato un jar o un repository produce `work/<mod>/{remapped.jar, decompiled/, meta/}` con tiny-remapper (intermediary → Mojmap) e Vineflower. Se il sorgente originale è disponibile si preferisce quello. Versioni dei tool pinnate, un solo comando. *Accettazione:* stesso input, stesso hash di output; funziona sulle due mod pilota.

**T1.2 — Metadati.** Parser per `fabric.mod.json` (entrypoint, dipendenze, mixin), `*.mixins.json`, access widener, licenza, elenco asset.

**T1.3 — Analisi statica → `mod-manifest.json`.** Lato JVM (ASM o Spoon, con il jar di Minecraft sul classpath per la risoluzione dei tipi). Il manifest contiene: entrypoint, callback Fabric API usati, Mixin classificati per tipo (Inject HEAD/RETURN/altro, Redirect, ModifyVariable, Overwrite, Accessor, Invoker, local capture), classi e metodi Minecraft referenziati con conteggio, uso di reflection e thread, shader, asset, packet custom, dipendenze da altre mod o librerie. *Accettazione:* il manifest valida contro un JSON Schema; ogni Mixin ha una classe; verifica manuale a campione su una mod.

**T1.4 — Punteggio di portabilità.** Verde / giallo / rosso per mod e per feature, con motivazione, in `docs/porting/census-<mod>.md`. *Accettazione:* per ogni feature da portare ci sono stima di sforzo e rischio principale.

**T1.5 — Census di corpus (opzionale).** Solo se prevedi più di cinque mod: analisi di 50-100 mod per ordinare per frequenza le API da costruire.

## 5. Fase 2 — `horizon-mod-api` v0 (\~3-4 settimane)

È guidata dal manifest delle mod pilota, non da un’idea astratta di «tutto Minecraft». Può procedere in parallelo alla Fase 1 una volta chiuso il T0.1.

**T2.1 — Tipi base:** `BlockPos`, `Vec3`, `ChunkPos`, `Identifier`, testo/`Component`, `Direction`, con la semantica Java documentata (overflow, convenzioni sulle coordinate). *Accettazione:* unit test e proptest.

**T2.2 — Event bus e lifecycle:** init, client tick, world load/unload, shutdown; trait `HorizonListener<E>`. *Accettazione:* ordine di dispatch documentato e testato; nessun lock tenuto durante le callback.

**T2.3 — Registry, comandi, keybind, config** (serde, persistenza su disco).

**T2.4 — Networking custom:** registrazione di canali e payload, encode/decode con parità di formato rispetto a Java (VarInt, stringhe UTF, NBT dove serve). *Accettazione:* golden bytes prodotti da Java e letti da Rust.

**T2.5 — Draw context per HUD e Screen:** testo, rettangoli, texture, matrix stack minimo.

**T2.6 — Hook di rendering e mondo** richiesti dai pilota, solo quelli presenti nel manifest. Per ognuno: evento, dati esposti, contratto di threading.

**T2.7 — Documentazione:** `docs/porting/API.md` con la tabella simbolo Java (Mojmap) → API Horizon per ogni elemento esposto. È la base dell’eventuale schema (RON) e della `hooks_map` se più avanti servirà generare i binding.

*Accettazione di fase:* le due mod pilota compilano contro l’API con corpi `todo!`; `cargo check`, `clippy` e `test` sono verdi; ogni lacuna sta in `API_GAPS.md`.

## 6. Fase 3 — Harness di verifica (\~1-2 settimane)

Viene prima del porting vero e proprio: senza test l’agente non ha un modo oggettivo per sapere se ha sbagliato.

**T3.1 — Differential testing delle funzioni pure.** Un modulo JVM (Gradle/JUnit) chiama funzioni statiche o pure scelte della mod con input generati (seed fisso; casi limite: MIN/MAX, zero, negativi, NaN, stringhe non ASCII) e salva i vettori in `testdata/<mod>/<fn>.json`. Test Rust li rilegge e confronta: esatto per gli interi, tolleranza in ULP dichiarata per i float. *Accettazione:* ciclo verde su almeno tre funzioni; il test fallisce se si altera la funzione Rust.

**T3.2 — Golden bytes** per packet, NBT e config prodotti dal codice Java e riletti in Rust, e viceversa.

**T3.3 — Shader (solo se serve):** rendering headless wgpu su adapter software, golden image con soglia di differenza.

**T3.4 — `tools/porting/verify.sh`:** `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, difftest. È il gate unico che ogni agente deve far passare prima di dichiarare un task finito.

## 7. Fase 4 — Porting delle mod pilota (\~2-4 settimane)

Per ogni mod, un ciclo:

1. **Skeleton:** crate `crates/mods/<mod>` con un modulo per feature, firme Rust e `todo!` derivati dal manifest.
2. **Task agente per feature**, con questo contratto: input = sorgente Java rimappato del metodo, firma Rust, estratto di `API.md`, vettori di test; output = solo il corpo della funzione più i test. Un task, una feature piccola, una PR.
3. **Gate:** `verify.sh` verde, nessun `todo!` non giustificato.
4. **Revisione umana** della PR.

Ordine: prima gli eventi Fabric API (più semplici da mappare), poi i Mixin `@Inject` semplici. I Mixin complessi (`@Redirect`, `@ModifyVariable`, `@Overwrite`, local capture) non si aggirano con workaround: diventano una richiesta di nuovo hook in `API_GAPS.md`.

Al termine, `docs/porting/report-<mod>.md`: cosa è portato e cosa no, `todo!` residui, lacune API, metriche rispetto alla baseline.

*Accettazione di fase:* le due mod girano in Horizon con le feature concordate nel T0.4; smoke test manuale documentato (log o screenshot); metriche registrate.

## 8. Fase 5 — Go/no-go sui generatori (condizionale)

Confronta le metriche della Fase 4 con la baseline. Se il porting assistito è già rapido e i `todo!` sono pochi, ferma qui. Costruisci un generatore solo se supera la regola di break-even del §2.

- **5A · Data transpiler.** Java → IR JSON → Rust con `quote` o template; solo record, enum, DTO, config, funzioni statiche pure. Tabella dei tipi, nullability → `Option`, aritmetica `wrapping_*`, snapshot test con `insta`. *Soglia:* molte classi dati ripetitive in più mod.
- **5B · Hook generator.** Tier 1: eventi Fabric API/Forge. Tier 2: Mixin `@Inject` semplici. Tier 3: tutto il resto, segnalato come «manuale, serve nuovo hook». Sorgente di verità: schema dell’API (RON) da cui si generano sia i trait Rust sia le tabelle Java → Horizon.
- **5C · Shader pipeline.** Risoluzione di `#moj_import`, normalizzazione di `#version` e degli uniform sciolti in UBO, poi `naga` → WGSL e validazione. Nomi vanilla (`ProjMat`, `ModelViewMat`), non `u_*`. *Soglia:* almeno due mod con shader.
- **5D · Asset e data.** Conversione di lang, modelli, blockstate, ricette e tag.

*Accettazione per ogni generatore:* riproduce in modo funzionalmente equivalente il porting fatto a mano nella Fase 4; output deterministico (stesso input, stessi byte); snapshot test; ciò che non supporta genera `todo!` e una riga di report.

## 9. Fase 6 — Scala (continuo)

Comando unico `horizon-port <mod>` con report, CI, supporto NeoForge/Forge, procedura di bump della versione di Minecraft (aggiornamento mappings), valutazione del component model WASM per mod esterne. TeaVM resta un’alternativa da valutare solo per logica fortemente algoritmica, tenendo presente che porta con sé un runtime.

## 10. Convenzioni Java → Rust per gli agenti

- `null` → `Option<T>`; niente `unwrap` su valori che in Java potevano essere null, se non motivato.
- Overflow di `int`/`long`: usare `wrapping_*`. `>>>` → cast a `u32`/`u64` e shift. `Math.floorMod` → `rem_euclid`. `Math.round` (arrotonda per eccesso a metà) non coincide con `f32::round`.
- `String` Java è UTF-16: `length`, `charAt` e `substring` lavorano su unità UTF-16; usare `encode_utf16()` quando la semantica conta.
- L’ordine di iterazione di `HashMap` è deterministico ma non garantito: se il risultato dipende dall’ordine, usare `IndexMap` o `BTreeMap` e documentarlo.
- Ereditarietà → trait più composizione, oppure enum. Evitare `Rc<RefCell<…>>` se non necessario; per i thread `Arc<RwLock<…>>` e canali, mai lock tenuti durante callback di render.
- Eccezioni → `Result` con enum di errore, non panic.
- Nessun `unsafe` senza ADR.
- Ogni funzione portata ha un commento `// PORT: Classe#metodo` per la tracciabilità.

## 11. Struttura del repository

```
crates/horizon-mod-api/
crates/mods/<mod>/
tools/porting/{ingest,census,difftest,verify.sh}
tools/porting/{codegen-data,codegen-hooks,shader-xpile}   # solo se la Fase 5 li approva
testdata/<mod>/
docs/porting/{PLAN,DECISIONS,LICENSE_POLICY,API,API_GAPS,OPEN_QUESTIONS,PORTING_LOG}.md
```

Se il layout reale di Horizon è diverso, l’agente propone un adattamento nel T0.2 invece di imporre questo.

## 12. Rischi principali

- **API di Horizon incompleta.** È dove andrà la maggior parte del tempo. Mitigazione: audit T0.1 e registro delle lacune.
- **Mixin profondi.** Non sono eventi: richiedono che Horizon esponga davvero quel punto. Mitigazione: classificazione nel manifest e priorità agli eventi Fabric API.
- **Licenze.** Un porting automatico di una mod All Rights Reserved è una violazione. Mitigazione: gate T0.3.
- **Cambio di versione di Minecraft** a metà percorso: rompe mappings e API. Mitigazione: versione fissata in T0.2.
- **Scope creep.** Mod con pipeline di rendering propria (come Distant Horizons) vanno rimandate finché l’API non è matura.
- **Riproducibilità degli agenti.** Registra in `PORTING_LOG.md` modello, prompt e hash degli input di ogni task.
