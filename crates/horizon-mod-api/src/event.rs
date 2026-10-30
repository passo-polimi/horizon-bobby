//! Event bus e lifecycle di Horizon per mod (Task T2.2).
//!
//! Contratto di sicurezza: nessun lock viene mantenuto attivo durante l'esecuzione
//! delle callback registrate, prevenendo qualsiasi deadlock con i thread del gioco.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// Trait marcatore per tutti gli eventi del sistema Horizon.
pub trait HorizonEvent: Any + Send + Sync + 'static {}

/// Trait implementato dai listener di eventi.
pub trait HorizonListener<E: HorizonEvent>: Send + Sync + 'static {
    fn on_event(&self, event: &E);
}

impl<E: HorizonEvent, F: Fn(&E) + Send + Sync + 'static> HorizonListener<E> for F {
    fn on_event(&self, event: &E) {
        (self)(event);
    }
}

/// Fase di un tick (iniziale o finale).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Start,
    End,
}

// ==========================================
// Eventi standard di ciclo di vita
// ==========================================

/// Evento scatenato all'inizializzazione del client moddato.
#[derive(Clone, Debug)]
pub struct ModInitEvent;
impl HorizonEvent for ModInitEvent {}

/// Evento scatenato ad ogni tick del client (20 TPS).
///
/// Corrisponde a `ClientTickEvents.START_CLIENT_TICK` e `END_CLIENT_TICK`.
#[derive(Clone, Debug)]
pub struct ClientTickEvent {
    pub phase: Phase,
    pub tick_count: u64,
}
impl HorizonEvent for ClientTickEvent {}

/// Evento scatenato prima di renderizzare il frame.
#[derive(Clone, Debug)]
pub struct RenderTickEvent {
    pub partial_ticks: f32,
    pub delta_time_secs: f32,
}
impl HorizonEvent for RenderTickEvent {}

/// Evento scatenato al caricamento di un mondo / connessione server.
#[derive(Clone, Debug)]
pub struct WorldLoadEvent {
    pub world_name: String,
    pub is_remote: bool,
}
impl HorizonEvent for WorldLoadEvent {}

/// Evento scatenato allo scaricamento del mondo.
#[derive(Clone, Debug)]
pub struct WorldUnloadEvent;
impl HorizonEvent for WorldUnloadEvent {}

/// Evento scatenato quando un chunk entra nel raggio di visione.
#[derive(Clone, Debug)]
pub struct ChunkLoadEvent {
    pub x: i32,
    pub z: i32,
}
impl HorizonEvent for ChunkLoadEvent {}

/// Evento scatenato quando un chunk esce dal raggio di visione.
#[derive(Clone, Debug)]
pub struct ChunkUnloadEvent {
    pub x: i32,
    pub z: i32,
}
impl HorizonEvent for ChunkUnloadEvent {}

// ==========================================
// Bus di eventi Thread-Safe senza lock in callback
// ==========================================

type ListenerCallback = Arc<dyn Fn(&dyn Any) + Send + Sync>;

#[derive(Default)]
pub struct EventBus {
    listeners: Mutex<HashMap<TypeId, Vec<(i32, ListenerCallback)>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            listeners: Mutex::new(HashMap::new()),
        }
    }

    /// Registra un listener con priorità (numeri più bassi vengono eseguiti per primi).
    pub fn register<E: HorizonEvent, L: HorizonListener<E>>(&self, priority: i32, listener: L) {
        let listener = Arc::new(listener);
        let callback: ListenerCallback = Arc::new(move |any_event| {
            if let Some(event) = any_event.downcast_ref::<E>() {
                listener.on_event(event);
            }
        });

        let mut lock = self.listeners.lock().unwrap();
        let list = lock.entry(TypeId::of::<E>()).or_default();
        list.push((priority, callback));
        list.sort_by_key(|(p, _)| *p);
    }

    /// Registra una closure con priorità standard (0).
    pub fn listen<E: HorizonEvent, F: Fn(&E) + Send + Sync + 'static>(&self, callback: F) {
        self.register(0, callback);
    }

    /// Distribuisce l'evento a tutti i listener registrati.
    ///
    /// GARANZIA: Il lock interno viene rilasciato prima di invocare le callback!
    pub fn post<E: HorizonEvent>(&self, event: &E) {
        let callbacks = {
            let lock = self.listeners.lock().unwrap();
            match lock.get(&TypeId::of::<E>()) {
                Some(list) => list.iter().map(|(_, cb)| cb.clone()).collect::<Vec<_>>(),
                None => return,
            }
        };

        for cb in callbacks {
            cb(event);
        }
    }
}

