use horizon_dh::anvil_importer::MmapAnvilRegion;
use std::path::PathBuf;

#[test]
fn test_debug_real_chunks() {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    let region_path = PathBuf::from(&appdata)
        .join("horizon")
        .join("instances")
        .join("modded")
        .join("saves")
        .join("modded")
        .join("dimensions")
        .join("minecraft")
        .join("overworld")
        .join("region")
        .join("r.-1.-1.mca");

    println!("CHECKING PATH: {:?}", region_path);
    assert!(
        region_path.exists(),
        "Region file must exist at {:?}",
        region_path
    );

    let dir = PathBuf::from(&appdata)
        .join("horizon")
        .join("instances")
        .join("modded")
        .join("saves")
        .join("modded")
        .join("dimensions")
        .join("minecraft")
        .join("overworld");
    let save = horizon_client::save::SaveDir::new(dir);
    let has = save.lod_saved(-32, -32);
    println!("SaveDir.lod_saved(-32, -32) = {}", has);
    let has_nonexistent = save.lod_saved(9999, 9999);
    println!("SaveDir.lod_saved(9999, 9999) = {}", has_nonexistent);
}
