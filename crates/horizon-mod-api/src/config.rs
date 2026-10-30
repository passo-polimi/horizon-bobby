//! Persistenza e gestione configurazioni per mod con Serde (Task T2.3).

use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::RwLock,
};

/// Trait implementato dalle struct di configurazione delle mod.
pub trait Config: Serialize + DeserializeOwned + Default + Clone + Send + Sync + 'static {}
impl<T: Serialize + DeserializeOwned + Default + Clone + Send + Sync + 'static> Config for T {}

/// Wrapper thread-safe per gestire caricamento, modifica e salvataggio della configurazione su disco.
pub struct ModConfig<T: Config> {
    mod_id: String,
    path: PathBuf,
    data: RwLock<T>,
}

impl<T: Config> ModConfig<T> {
    /// Inizializza la configurazione per la mod, caricando da `<config_dir>/<mod_id>.json`.
    pub fn load(config_dir: &Path, mod_id: &str) -> Self {
        let path = config_dir.join(format!("{mod_id}.json"));
        let data = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                    log::warn!("Errore nel parsing config per {mod_id}: {e}. Uso defaults.");
                    T::default()
                }),
                Err(_) => T::default(),
            }
        } else {
            let def = T::default();
            if let Ok(json_str) = serde_json::to_string_pretty(&def) {
                let _ = fs::create_dir_all(config_dir);
                let _ = fs::write(&path, json_str);
            }
            def
        };

        Self {
            mod_id: mod_id.to_string(),
            path,
            data: RwLock::new(data),
        }
    }

    /// Restituisce l'identificativo della mod.
    pub fn mod_id(&self) -> &str {
        &self.mod_id
    }

    /// Restituisce il percorso del file di configurazione.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Legge una copia dei dati correnti di configurazione.
    pub fn get(&self) -> T {
        self.data.read().unwrap().clone()
    }

    /// Modifica la configurazione ed aggiorna il file su disco.
    pub fn mutate<F: FnOnce(&mut T)>(&self, f: F) -> Result<(), String> {
        let mut lock = self.data.write().unwrap();
        f(&mut lock);

        let json_str = serde_json::to_string_pretty(&*lock).map_err(|e| e.to_string())?;
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&self.path, json_str).map_err(|e| e.to_string())?;
        Ok(())
    }
}

