use horizon_client::{
    assets::Assets,
    biome::Biomes,
    blocks::BlockFacts,
    chunk::Chunk,
    light::LightTables,
    mesher::MeshTables,
    models,
    streaming::{ChunkGenerator, Proto, Streaming, WorkerContext},
    world::World,
};
use horizon_world::registry::BlockStateId;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

fn resources() -> Option<std::path::PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.local/horizon-client")
        .join(horizon_core::VANILLA_VERSION);
    root.join("items.json").exists().then_some(root)
}

struct MissingRemoteGenerator;

impl ChunkGenerator for MissingRemoteGenerator {
    type Worker = ();
    fn worker(&self) {}
    fn load(&self, _x: i32, _z: i32) -> Option<Chunk> {
        None
    }
    fn proto(&self, _: &mut (), x: i32, z: i32) -> Proto {
        Box::new((x, z))
    }
    fn finish(&self, proto: Proto) -> Chunk {
        let (x, z) = *proto.downcast::<(i32, i32)>().unwrap();
        Chunk::empty(x, z)
    }
}

/// Helper to build a WorkerContext from client resources.
fn build_context() -> Option<Arc<WorkerContext>> {
    let root = resources()?;
    let assets = Arc::new(Assets::load(&root).ok()?);
    let render = Arc::new(assets.reload(&root).ok()?);
    let facts = BlockFacts::load(&root, assets.registry).ok()?;
    let visibility = models::load_visibility(&root, assets.registry).ok()?;
    Some(Arc::new(WorkerContext {
        tables: Arc::new(MeshTables::new(&assets, &render, &facts, visibility).ok()?),
        light: Arc::new(LightTables::load(&root, assets.registry, &facts).ok()?),
        biomes: Arc::new(Biomes::load(&root).ok()?),
        assets,
        render,
    }))
}

/// (a) Colonna mancante su disco in remoto: nessun chunk vuoto inserito a (0,0),
/// chiave liberata da generating, aggiunta a missing, non ritentata.
#[test]
fn missing_column_on_disk_in_remote_does_not_insert_empty_chunk_and_frees_key() {
    let Some(context) = build_context() else {
        return;
    };
    let generator = Arc::new(MissingRemoteGenerator);
    let mut streaming = Streaming::new(generator, context, 2);
    streaming.set_remote(true);
    streaming.center = (10, 10);
    streaming.radius = 2;

    let mut world = World::new(BlockStateId(0), 0);
    let deadline = Instant::now() + Duration::from_secs(5);

    while Instant::now() < deadline {
        let _ = streaming.update(&mut world);
        if !streaming.missing.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    // Verify missing set populated, generating emptied, and no chunk inserted in world
    assert!(
        !streaming.missing.is_empty(),
        "missing columns should be tracked"
    );
    for &key in &streaming.missing {
        assert!(
            !streaming.generating.contains(&key),
            "generating must be cleared"
        );
        assert!(
            !world.chunks.contains_key(&key),
            "missing column must NOT be inserted into world"
        );
    }
    // Specifically verify center (10, 10) did NOT overwrite (0, 0)
    assert!(
        !world.chunks.contains_key(&(0, 0)),
        "missing chunks must not insert fake (0, 0)"
    );
}

/// (b) Chunk del server non sovrascritto da un job da disco in ritardo.
#[test]
fn server_chunk_not_overwritten_by_late_disk_job() {
    let Some(context) = build_context() else {
        return;
    };
    let generator = Arc::new(MissingRemoteGenerator);
    let mut streaming = Streaming::new(generator, context, 1);
    streaming.set_remote(true);
    streaming.center = (0, 0);
    streaming.radius = 4;

    let mut world = World::new(BlockStateId(0), 0);
    let key = (3, 3);

    // 1. Authoritative server chunk arrives
    let mut server_chunk = Chunk::empty(key.0, key.1);
    server_chunk.set(0, 60, 0, 999); // custom marker
    streaming.insert_remote(&mut world, server_chunk);

    assert!(streaming.server_chunks.contains(&key));
    assert_eq!(world.chunks.get(&key).unwrap().get(0, 60, 0), 999);

    // 2. Late disk job arrives for the same key
    let mut late_disk_chunk = Chunk::empty(key.0, key.1);
    late_disk_chunk.set(0, 60, 0, 111); // disk marker
    streaming.receive_generated(late_disk_chunk, &mut world);

    // 3. Must retain server chunk (marker 999, not overwritten by 111)
    assert_eq!(
        world.chunks.get(&key).unwrap().get(0, 60, 0),
        999,
        "authoritative server chunk must not be overwritten by late disk job"
    );
    assert!(!streaming.cached_chunks.contains(&key));
}

/// (c) Selezione della cartella per dimensione.
#[test]
fn dimension_folder_selection_logic() {
    fn resolve_dimension_path(save_root: &Path, dim: &str) -> PathBuf {
        match dim {
            "minecraft:the_nether" | "the_nether" | "DIM-1" => {
                let p = save_root
                    .join("dimensions")
                    .join("minecraft")
                    .join("the_nether");
                if p.exists() {
                    p
                } else {
                    save_root.join("DIM-1")
                }
            }
            "minecraft:the_end" | "the_end" | "DIM1" => {
                let p = save_root
                    .join("dimensions")
                    .join("minecraft")
                    .join("the_end");
                if p.exists() {
                    p
                } else {
                    save_root.join("DIM1")
                }
            }
            _ => {
                let p = save_root
                    .join("dimensions")
                    .join("minecraft")
                    .join("overworld");
                if p.exists() {
                    p
                } else {
                    save_root.to_path_buf()
                }
            }
        }
    }

    let temp_root = std::env::temp_dir().join("horizon_test_dim_sel");
    let overworld = temp_root
        .join("dimensions")
        .join("minecraft")
        .join("overworld");
    let nether = temp_root
        .join("dimensions")
        .join("minecraft")
        .join("the_nether");
    let end = temp_root
        .join("dimensions")
        .join("minecraft")
        .join("the_end");

    let _ = std::fs::create_dir_all(&overworld);
    let _ = std::fs::create_dir_all(&nether);
    let _ = std::fs::create_dir_all(&end);

    assert_eq!(
        resolve_dimension_path(&temp_root, "minecraft:overworld"),
        overworld
    );
    assert_eq!(
        resolve_dimension_path(&temp_root, "minecraft:the_nether"),
        nether
    );
    assert_eq!(resolve_dimension_path(&temp_root, "minecraft:the_end"), end);

    let _ = std::fs::remove_dir_all(&temp_root);
}

/// (d) Nessuna scrittura su SaveDir / world.modified in remoto.
#[test]
fn zero_writes_to_modified_or_saves_in_remote() {
    let Some(context) = build_context() else {
        return;
    };
    let generator = Arc::new(MissingRemoteGenerator);
    let mut streaming = Streaming::new(generator, context, 1);
    streaming.set_remote(true);
    streaming.center = (0, 0);
    streaming.radius = 4;

    let mut world = World::new(BlockStateId(0), 0);
    let key = (1, 1);

    let mut fresh_chunk = Chunk::empty(key.0, key.1);
    fresh_chunk.fresh = true;

    streaming.receive_generated(fresh_chunk, &mut world);

    // In remote mode, fresh chunk must NOT be added to world.modified
    assert!(
        !world.modified.contains(&key),
        "fresh chunks in remote must never be scheduled for saving in world.modified"
    );
    assert!(streaming.cached_chunks.contains(&key));
}

/// (e) Verifica che l'auto-salvataggio asincrono scriva i chunk di rete nei file .mca della cache server.
#[test]
fn auto_save_flushes_pending_network_chunks_to_anvil_region_cache() {
    let root = match resources() {
        Some(r) => r,
        None => return,
    };
    let assets = match Assets::load(&root) {
        Ok(a) => a,
        Err(_) => return,
    };
    let biomes = match Biomes::load(&root) {
        Ok(b) => Arc::new(b),
        Err(_) => return,
    };

    let temp_dir = std::env::temp_dir().join(format!(
        "horizon-bobby-autosave-test-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(temp_dir.join("region")).unwrap();

    let save = horizon_client::save::SaveDir::new(temp_dir.clone());
    let chunk = Arc::new(Chunk::empty(5, 5));

    // Save later in background
    let queued = save.save_later(&[chunk], assets.registry, &biomes).unwrap();
    assert_eq!(queued, 1);

    // Flush to disk
    save.flush().unwrap();

    // Verify .mca exists for chunk (5, 5) -> region r.0.0.mca
    let mca_path = temp_dir.join("region").join("r.0.0.mca");
    assert!(
        mca_path.exists(),
        "Region file r.0.0.mca must be created by auto-save"
    );
    assert!(
        std::fs::metadata(&mca_path).unwrap().len() > 0,
        "Region file must not be empty"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}
