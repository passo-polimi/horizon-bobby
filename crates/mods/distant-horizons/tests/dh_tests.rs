use distant_horizons::{
    culling::LodFrustum,
    mesher::build_greedy_mesh_from_columns,
    rle_data::{ColumnarData, HeightSpan},
    shader::{DistantHorizonsHook, FAR_TERRAIN_WGSL},
    storage::{load_hlod, save_hlod},
};
use horizon_mod_api::prelude::*;

#[test]
fn test_columnar_data_and_downsample() {
    let mut data = ColumnarData::new(4);
    for z in 0..4 {
        for x in 0..4 {
            data.set_column(
                x,
                z,
                vec![HeightSpan {
                    min_y: 60,
                    max_y: 70,
                    block_id: 1,
                    light: 15,
                }],
            );
        }
    }

    assert_eq!(data.size, 4);
    assert_eq!(data.get_column(0, 0).len(), 1);

    // Downsample 4x4 -> 2x2
    let downsampled = data.downsample();
    assert_eq!(downsampled.size, 2);
    let col = downsampled.get_column(0, 0);
    assert_eq!(col.len(), 1);
    assert_eq!(col[0].min_y, 60);
    assert_eq!(col[0].max_y, 70);
    assert_eq!(col[0].light, 15);
}

#[test]
fn test_hlod_storage_save_load_roundtrip() {
    let pos = ChunkPos::new(12, -34);
    let payload = b"Test binary columnar data payload for distant horizons .hlod format!";

    let encoded = save_hlod(pos, payload).unwrap();
    assert!(encoded.starts_with(b"DHLOD1"));

    let (loaded_pos, loaded_payload) = load_hlod(&encoded).unwrap();
    assert_eq!(loaded_pos, pos);
    assert_eq!(loaded_payload, payload);

    // Test corruzione checksum
    let mut corrupted = encoded.clone();
    let last = corrupted.len() - 1;
    corrupted[last] ^= 0xFF;
    assert!(
        load_hlod(&corrupted).is_err(),
        "Un checksum corrotto deve generare errore"
    );
}

#[test]
fn test_greedy_mesher_quad_merging_and_skirt() {
    let mut data = ColumnarData::new(4);
    // Tutte le 16 colonne hanno la stessa altezza e blocco
    for z in 0..4 {
        for x in 0..4 {
            data.set_column(
                x,
                z,
                vec![HeightSpan {
                    min_y: 0,
                    max_y: 64,
                    block_id: 2,
                    light: 15,
                }],
            );
        }
    }

    // Senza skirt: le 16 celle complanari devono unirsi in 1 solo quad (4x4)
    let quads = build_greedy_mesh_from_columns(&data, false);
    assert_eq!(
        quads.len(),
        1,
        "Il mesher greedy deve fondere 16 colonne complanari in un unico quad"
    );
    assert_eq!(quads[0].width, 4);
    assert_eq!(quads[0].depth, 4);
    assert_eq!(quads[0].y, 64);
    assert!(!quads[0].is_skirt);

    // Con skirt per confinante a livello inferiore
    let quads_with_skirt = build_greedy_mesh_from_columns(&data, true);
    assert!(
        quads_with_skirt.iter().any(|q| q.is_skirt),
        "Deve essere presente almeno uno skirt verticale per colmare il dislivello"
    );
}

#[test]
fn test_lod_frustum_visibility() {
    let camera_near = Vec3::new(0.0, 100.0, 0.0);
    let camera_far = Vec3::new(10000.0, 100.0, 10000.0);

    // Box centrata su (0, 0, 0)
    let is_near_visible = LodFrustum::is_box_visible(
        [-100.0, -64.0, -100.0],
        [100.0, 320.0, 100.0],
        camera_near,
        4096.0,
    );
    assert!(is_near_visible);

    let is_far_visible = LodFrustum::is_box_visible(
        [-100.0, -64.0, -100.0],
        [100.0, 320.0, 100.0],
        camera_far,
        4096.0,
    );
    assert!(!is_far_visible);
}

#[test]
fn test_far_terrain_wgsl_syntax() {
    assert!(FAR_TERRAIN_WGSL.contains("fn vs_main"));
    assert!(FAR_TERRAIN_WGSL.contains("fn fs_main"));
    assert!(FAR_TERRAIN_WGSL.contains("BAYER_4X4"));

    let _hook = DistantHorizonsHook;
}
