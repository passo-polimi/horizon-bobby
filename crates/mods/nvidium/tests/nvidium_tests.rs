use horizon_mod_api::prelude::*;
use nvidium::{
    frustum::{BoundingBox, Frustum},
    hiz::HiZPyramid,
    indirect::IndirectDrawBuffer,
    meshlet::{ChunkMeshlet, CompactVertex},
    pipeline::NvidiumHook,
};

#[test]
fn test_bounding_box_and_frustum() {
    let bbox = BoundingBox::from_section(BlockPos::new(0, 0, 0));
    assert_eq!(bbox.min, [0.0, 0.0, 0.0]);
    assert_eq!(bbox.max, [16.0, 16.0, 16.0]);
    assert_eq!(bbox.center(), [8.0, 8.0, 8.0]);
    assert_eq!(bbox.extents(), [8.0, 8.0, 8.0]);

    // Matrice identità di proiezione (frustum canonico [-1, 1])
    #[rustfmt::skip]
    let identity_matrix = [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ];
    let frustum = Frustum::from_view_projection(&identity_matrix);

    let inside_box = BoundingBox {
        min: [-0.5, -0.5, -0.5],
        max: [0.5, 0.5, 0.5],
    };
    assert!(frustum.is_box_visible(&inside_box));

    let outside_box = BoundingBox {
        min: [50.0, 50.0, 50.0],
        max: [60.0, 60.0, 60.0],
    };
    assert!(!frustum.is_box_visible(&outside_box));
}

#[test]
fn test_hiz_pyramid_downsample_and_occlusion() {
    let mut hiz = HiZPyramid::new(16, 16);
    assert_eq!(hiz.levels.len(), 5); // 16x16 -> 8x8 -> 4x4 -> 2x2 -> 1x1

    // Simula profondità vicina (0.2) in primo piano
    let depth_data = vec![0.2f32; 256];
    hiz.update_hiz_from_depth(&depth_data);

    // Un oggetto dietro a profondità 0.8 deve risultare occluso
    assert!(hiz.test_occlusion(0.8, 16.0));

    // Un oggetto davanti a profondità 0.1 NON deve risultare occluso
    assert!(!hiz.test_occlusion(0.1, 16.0));
}

#[test]
fn test_compact_vertex_layout_and_meshlet() {
    assert_eq!(
        std::mem::size_of::<CompactVertex>(),
        16,
        "La dimensione di CompactVertex deve essere esattamente 16 byte"
    );

    let mut meshlet = ChunkMeshlet::default();
    let quad_pos = [
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];
    let quad_uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    meshlet.pack_quad(quad_pos, quad_uv, 0x00FF_00FF, 0xFFFF_FFFF);
    assert_eq!(meshlet.vertices.len(), 4);
    assert_eq!(meshlet.indices.len(), 6); // 2 triangoli = 6 indici
}

#[test]
fn test_indirect_draw_buffer_emission() {
    let mut buf = IndirectDrawBuffer::default();
    buf.emit_draw_command(6, 0, 0);
    buf.emit_draw_command(12, 6, 4);

    assert_eq!(buf.commands.len(), 2);
    assert_eq!(buf.byte_len(), 40); // 2 * 20 byte = 40 byte
    assert_eq!(buf.commands[1].first_index, 6);
    assert_eq!(buf.commands[1].base_vertex, 4);

    buf.clear();
    assert_eq!(buf.commands.len(), 0);
}

#[test]
fn test_nvidium_hook_lifecycle() {
    let hook = NvidiumHook::default();
    assert!(hook.indirect_buffer().is_none());
}
