//! Hot-Folder Automatica per l'ingestione all'avvio di Horizon.
//!
//! Controlla la presenza di cartelle `horizon_import/<server_ip>/<dimension>/region/`
//! (o `~/.minecraft/horizon_import/...`), rileva i file `.mca` nuovi o modificati
//! tramite mtime/dimensione, e avvia l'elaborazione multi-thread in background
//! popolando sia la cache Fake-Chunk (Bobby) sia i LOD Quadtree prima del login al server.

use crate::anvil_importer::MmapAnvilRegion;
use crate::bobby_cache::BobbyCache;
use crate::lod_baker::LodBaker;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// Statistiche dell'ingestione Hot-Folder.
#[derive(Debug, Clone, Default)]
pub struct HotFolderStats {
    pub regions_found: usize,
    pub regions_processed: usize,
    pub chunks_imported: usize,
    pub bytes_written: u64,
    pub elapsed: Duration,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ImportState {
    files: HashMap<String, u64>, // file_name -> mtime_secs
}

/// Restituisce il percorso canonico della hot-folder di importazione per un dato server e dimensione.
pub fn default_hot_folder_dir(server_ip: &str, dimension: &str) -> PathBuf {
    // 1. Controlla cartella .minecraft standard
    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = PathBuf::from(appdata)
            .join(".minecraft")
            .join("horizon_import")
            .join(server_ip)
            .join(dimension)
            .join("region");
        if p.exists() {
            return p;
        }
    }

    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        let p = PathBuf::from(home)
            .join(".minecraft")
            .join("horizon_import")
            .join(server_ip)
            .join(dimension)
            .join("region");
        if p.exists() {
            return p;
        }
    }

    // 2. Fallback locale al workspace/cartella corrente
    PathBuf::from("horizon_import")
        .join(server_ip)
        .join(dimension)
        .join("region")
}

/// Scansiona una cartella `region/` ed esegue l'ingestione automatica dei file Anvil nuovi o aggiornati.
pub fn scan_and_import(
    region_dir: impl AsRef<Path>,
    bobby: &BobbyCache,
    lod_baker: &LodBaker,
) -> io::Result<HotFolderStats> {
    let region_dir = region_dir.as_ref();
    let start_time = Instant::now();

    if !region_dir.exists() {
        return Ok(HotFolderStats::default());
    }

    let state_file = region_dir.join(".import_state.json");
    let mut state: ImportState = if state_file.exists() {
        fs::read(&state_file)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    } else {
        ImportState::default()
    };

    // Raccoglie tutti i file .mca
    let mut mca_files = Vec::new();
    for entry in fs::read_dir(region_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("mca") {
            let file_name = path.file_name().unwrap().to_string_lossy().to_string();
            let metadata = fs::metadata(&path)?;
            let mtime = metadata
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            mca_files.push((path, file_name, mtime));
        }
    }

    let regions_found = mca_files.len();
    let mut to_process = Vec::new();

    for (path, file_name, mtime) in mca_files {
        let need_process = match state.files.get(&file_name) {
            Some(&prev_mtime) => prev_mtime != mtime,
            None => true,
        };
        if need_process {
            to_process.push((path, file_name, mtime));
        }
    }

    let mut total_chunks = 0;
    let regions_processed = to_process.len();

    // Processa i file .mca
    for (path, file_name, mtime) in to_process {
        if let Ok(region) = MmapAnvilRegion::open(&path) {
            let chunks = region.parse_all_chunks_parallel();
            let count = chunks.len();

            // 1. Salva nella cache Bobby
            let _ = bobby.bulk_put(&chunks);

            // 2. Esegui il baking Quadtree dei LOD in parallelo con Rayon
            chunks.par_iter().for_each(|chunk| {
                let _ = lod_baker.bake_chunk_pyramid(chunk);
            });

            total_chunks += count;
            state.files.insert(file_name, mtime);
        }
    }

    // Salva lo stato aggiornato
    if let Ok(serialized) = serde_json::to_vec_pretty(&state) {
        let _ = fs::write(state_file, serialized);
    }

    let elapsed = start_time.elapsed();
    let approx_bytes = (total_chunks as u64) * 8192; // stima byte scritti tra fake-chunk e lod

    Ok(HotFolderStats {
        regions_found,
        regions_processed,
        chunks_imported: total_chunks,
        bytes_written: approx_bytes,
        elapsed,
    })
}

/// Avvia l'ingestione Hot-Folder su un thread in background prima del login.
pub fn spawn_background_importer(
    region_dir: PathBuf,
    bobby: BobbyCache,
    lod_baker: LodBaker,
    on_complete: impl FnOnce(HotFolderStats) + Send + 'static,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("Horizon-HotFolder-Importer".into())
        .spawn(
            move || match scan_and_import(&region_dir, &bobby, &lod_baker) {
                Ok(stats) => {
                    log::info!(
                        "Horizon HotFolder: importati {} chunk da {} regioni in {:?}",
                        stats.chunks_imported,
                        stats.regions_processed,
                        stats.elapsed
                    );
                    on_complete(stats);
                }
                Err(e) => {
                    log::warn!("Horizon HotFolder: errore durante l'importazione: {e}");
                }
            },
        )
        .expect("Failed to spawn HotFolder thread")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_hot_folder_detection_and_idempotency() {
        let dir = tempdir().unwrap();
        let region_dir = dir.path().join("region");
        fs::create_dir_all(&region_dir).unwrap();

        let bobby = BobbyCache::new(dir.path(), "srv1", "overworld").unwrap();
        let lod_baker = LodBaker::new(dir.path(), "srv1", "overworld").unwrap();

        // Cartella vuota
        let stats = scan_and_import(&region_dir, &bobby, &lod_baker).unwrap();
        assert_eq!(stats.regions_found, 0);
        assert_eq!(stats.regions_processed, 0);

        // Aggiungi un file .mca fittizio
        let fake_mca = region_dir.join("r.0.0.mca");
        fs::write(&fake_mca, vec![0u8; 8192]).unwrap(); // header vuoto

        let stats2 = scan_and_import(&region_dir, &bobby, &lod_baker).unwrap();
        assert_eq!(stats2.regions_found, 1);
        assert_eq!(stats2.regions_processed, 1);

        // Seconda scansione: il file non è cambiato, deve essere saltato!
        let stats3 = scan_and_import(&region_dir, &bobby, &lod_baker).unwrap();
        assert_eq!(stats3.regions_found, 1);
        assert_eq!(stats3.regions_processed, 0); // saltato grazie allo state tracker
    }
}
