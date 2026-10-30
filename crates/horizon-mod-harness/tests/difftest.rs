use horizon_mod_harness::{
    DhSectionPosConvertVector, DhSectionPosEncodeVector, LightTestVector, NvidiumGoldenConfig,
    PositionTestVector, TextureTestVector, VarIntVector, dh_pure, nvidium_pure,
};
use std::{fs, io::Cursor, path::PathBuf};

fn get_testdata_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../../testdata")
}

#[test]
fn test_nvidium_encode_position_difftest() {
    let path = get_testdata_dir().join("nvidium/encode_position.json");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Impossibile leggere {}: {e}", path.display()));
    let vec_data: PositionTestVector = serde_json::from_str(&content).unwrap();

    assert!(
        !vec_data.cases.is_empty(),
        "I casi di test non devono essere vuoti"
    );
    for (i, case) in vec_data.cases.iter().enumerate() {
        let rust_res = nvidium_pure::encode_position(case.input.v);
        assert_eq!(
            rust_res, case.expected,
            "Discrepanza su nvidium::encode_position caso {i} (input: {}): Rust={rust_res}, Java={}",
            case.input.v, case.expected
        );
    }
}

#[test]
fn test_nvidium_compact_light_difftest() {
    let path = get_testdata_dir().join("nvidium/compact_light.json");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Impossibile leggere {}: {e}", path.display()));
    let vec_data: LightTestVector = serde_json::from_str(&content).unwrap();

    assert!(
        !vec_data.cases.is_empty(),
        "I casi di test non devono essere vuoti"
    );
    for (i, case) in vec_data.cases.iter().enumerate() {
        let rust_res = nvidium_pure::compact_light(case.input.light);
        assert_eq!(
            rust_res, case.expected,
            "Discrepanza su nvidium::compact_light caso {i} (input: {:#X}): Rust={rust_res:#X}, Java={:#X}",
            case.input.light, case.expected
        );
    }
}

#[test]
fn test_nvidium_encode_texture_difftest() {
    let path = get_testdata_dir().join("nvidium/encode_texture.json");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Impossibile leggere {}: {e}", path.display()));
    let vec_data: TextureTestVector = serde_json::from_str(&content).unwrap();

    assert!(
        !vec_data.cases.is_empty(),
        "I casi di test non devono essere vuoti"
    );
    for (i, case) in vec_data.cases.iter().enumerate() {
        let rust_res = nvidium_pure::encode_texture(case.input.u, case.input.v);
        assert_eq!(
            rust_res, case.expected,
            "Discrepanza su nvidium::encode_texture caso {i} (u: {}, v: {}): Rust={rust_res}, Java={}",
            case.input.u, case.input.v, case.expected
        );
    }
}

#[test]
fn test_dh_section_pos_encode_decode_difftest() {
    let path = get_testdata_dir().join("distant-horizons/section_pos_encode.json");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Impossibile leggere {}: {e}", path.display()));
    let vec_data: DhSectionPosEncodeVector = serde_json::from_str(&content).unwrap();

    assert!(
        !vec_data.cases.is_empty(),
        "I casi di test non devono essere vuoti"
    );
    for (i, case) in vec_data.cases.iter().enumerate() {
        let rust_packed = dh_pure::section_pos_encode(case.input.level, case.input.x, case.input.z);
        assert_eq!(
            rust_packed, case.packed,
            "Discrepanza packing DhSectionPos caso {i} ({:?}): Rust={rust_packed}, Java={}",
            case.input, case.packed
        );

        let dec_level = dh_pure::section_pos_get_level(rust_packed);
        let dec_x = dh_pure::section_pos_get_x(rust_packed);
        let dec_z = dh_pure::section_pos_get_z(rust_packed);

        assert_eq!(dec_level, case.decoded.level);
        assert_eq!(dec_x, case.decoded.x);
        assert_eq!(dec_z, case.decoded.z);
    }
}

#[test]
fn test_dh_section_pos_convert_difftest() {
    let path = get_testdata_dir().join("distant-horizons/section_pos_convert.json");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Impossibile leggere {}: {e}", path.display()));
    let vec_data: DhSectionPosConvertVector = serde_json::from_str(&content).unwrap();

    assert!(
        !vec_data.cases.is_empty(),
        "I casi di test non devono essere vuoti"
    );
    for (i, case) in vec_data.cases.iter().enumerate() {
        let initial_pos =
            dh_pure::section_pos_encode(case.input.from_level, case.input.x, case.input.z);
        let converted = dh_pure::section_pos_convert_detail_level(initial_pos, case.input.to_level);

        assert_eq!(
            converted, case.expected_packed,
            "Discrepanza conversione livello caso {i} ({:?}): Rust={converted}, Java={}",
            case.input, case.expected_packed
        );

        let dec_level = dh_pure::section_pos_get_level(converted);
        let dec_x = dh_pure::section_pos_get_x(converted);
        let dec_z = dh_pure::section_pos_get_z(converted);

        assert_eq!(dec_level, case.expected.level);
        assert_eq!(dec_x, case.expected.x);
        assert_eq!(dec_z, case.expected.z);
    }
}

#[test]
fn test_varint_golden_bytes_difftest() {
    let path = get_testdata_dir().join("network/varint.json");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Impossibile leggere {}: {e}", path.display()));
    let vec_data: VarIntVector = serde_json::from_str(&content).unwrap();

    assert!(!vec_data.cases.is_empty());
    for case in vec_data.cases {
        let mut buf = Vec::new();
        horizon_mod_api::net::write_varint(&mut buf, case.value).unwrap();
        assert_eq!(
            buf, case.bytes,
            "VarInt byte mismatch per valore {}: Rust={:?}, Java={:?}",
            case.value, buf, case.bytes
        );

        let mut cursor = Cursor::new(&buf);
        let decoded = horizon_mod_api::net::read_varint(&mut cursor).unwrap();
        assert_eq!(decoded, case.value);
    }
}

#[test]
fn test_nvidium_config_golden_roundtrip() {
    let path = get_testdata_dir().join("nvidium/config_golden.json");
    let content = fs::read_to_string(&path).unwrap();
    let config: NvidiumGoldenConfig = serde_json::from_str(&content).unwrap();

    assert_eq!(config.region_keep_distance, 32);
    assert!(config.async_occlusion_culling);
    assert!(config.translucent_sorting);
    assert_eq!(config.max_draw_distance, 64);
    assert!(config.enable_hiz_culling);

    let reserialized = serde_json::to_string_pretty(&config).unwrap();
    let re_parsed: NvidiumGoldenConfig = serde_json::from_str(&reserialized).unwrap();
    assert_eq!(config, re_parsed);
}

/// Acceptance criterion test: verifica esplicitamente che una mutazione della logica Rust
/// faccia fallire il test differenziale (garantisce che l'harness rileva effettivamente le regressioni).
#[test]
fn test_mutation_detection_fails_on_altered_rust_logic() {
    let altered_encode_position = |v: f32| -> i32 {
        // Logica intenzionalmente alterata con un offset errato di +1
        nvidium_pure::encode_position(v) + 1
    };

    let path = get_testdata_dir().join("nvidium/encode_position.json");
    let content = fs::read_to_string(&path).unwrap();
    let vec_data: PositionTestVector = serde_json::from_str(&content).unwrap();

    let mut detected_difference = false;
    for case in vec_data.cases {
        let rust_res = altered_encode_position(case.input.v);
        if rust_res != case.expected {
            detected_difference = true;
            break;
        }
    }

    assert!(
        detected_difference,
        "La mutazione nella funzione Rust deve essere rilevata dall'harness!"
    );
}
