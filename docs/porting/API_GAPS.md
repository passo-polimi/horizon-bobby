# API Gaps — Horizon Mod Porting

*Registro delle funzionalità o hook assenti nelle API attuali di Horizon.*  
*Regola: se l'API di Horizon non offre qualcosa, lo si annota qui; non si inventa e non si simula.*

---

## GAP-001: Registrazione Dinamica di RenderPass e Pipeline wgpu Custom
- **Sottosistema:** Rendering (`crates/horizon-client-render/src/gfx.rs`)
- **Descrizione:** In Horizon `Renderer::render` esegue una sequenza rigida e monolitica di RenderPass wgpu. Non esiste un'interfaccia a plugin (tipo Fabric Rendering API / Iris) che permetta a una mod di iniettare un pass prima o dopo il rendering del mondo o della skybox, né di associare un BindGroup dedicato.
- **Mod interessate:** Distant Horizons, Voxy.
- **Risoluzione temporanea / proposta:** Esporre in `horizon-client-render` un hook `RenderPassCallback` o un trait per consentire l'esecuzione di pass aggiuntivi.

---

## GAP-002: Inoltro Canali di Rete Custom (Plugin Messages)
- **Sottosistema:** Networking (`crates/horizon-client/src/net.rs`, `crates/horizon-protocol`)
- **Descrizione:** Il pacchetto `CustomPayload` è supportato nel protocollo ma `net.rs` filtra ed inoltra solo `minecraft:brand` e canali quiet proprietari. Tutti gli altri canali custom registrati dalle mod vengono scartati.
- **Mod interessate:** Distant Horizons (sincronizzazione LOD multiplayer).
- **Risoluzione proposta:** Implementare in `horizon-mod-api` un registro thread-safe di ascoltatori di canali:
  `register_payload_receiver(channel: &str, callback: Arc<dyn Fn(&[u8])>)`.

---

## GAP-003: Registrazione Dinamica di Tasti Rapidi (Keybindings)
- **Sottosistema:** Input (`crates/horizon-client-render/src/keybinds.rs`)
- **Descrizione:** I tasti rapidi sono definiti in un enum/struct statico per le sole funzioni vanilla. Una mod non può registrare dinamicamente un tasto visibile nella schermata Opzioni > Controlli.
- **Mod interessate:** Tutte le mod QoL, Distant Horizons (tasti per aprire menu di configurazione LOD).
- **Risoluzione proposta:** Aggiungere un registro `ModKeybinds` con callback al press/release.

