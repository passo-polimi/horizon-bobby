use horizon_mod_api::prelude::*;
use std::{
    io::Cursor,
    sync::atomic::{AtomicI32, Ordering},
};

#[test]
fn test_block_pos_packing_and_relative() {
    let pos = BlockPos::new(12345, -64, -67890);
    let packed = pos.as_long();
    let unpacked = BlockPos::from_long(packed);

    assert_eq!(
        pos, unpacked,
        "Roundtrip di as_long / from_long deve preservare le coordinate"
    );

    let north = pos.relative(Direction::North);
    assert_eq!(north, BlockPos::new(12345, -64, -67891));

    let up = pos.relative(Direction::Up);
    assert_eq!(up, BlockPos::new(12345, -63, -67890));
}

#[test]
fn test_chunk_pos_conversion() {
    let pos = BlockPos::new(35, 100, -20);
    let chunk = ChunkPos::from_block_pos(pos);
    assert_eq!(chunk, ChunkPos::new(2, -2));

    let packed = chunk.as_long();
    let unpacked = ChunkPos::from_long(packed);
    assert_eq!(chunk, unpacked);
}

#[test]
fn test_direction_opposites() {
    for d in Direction::ALL {
        assert_eq!(d.opposite().opposite(), d);
    }
    assert_eq!(Direction::North.opposite(), Direction::South);
    assert_eq!(Direction::Up.opposite(), Direction::Down);
    assert_eq!(Direction::West.opposite(), Direction::East);
}

#[test]
fn test_identifier_parsing() {
    let id1 = Identifier::parse("custom:my_texture").unwrap();
    assert_eq!(id1.namespace, "custom");
    assert_eq!(id1.path, "my_texture");

    let id2 = Identifier::parse("stone").unwrap();
    assert_eq!(id2.namespace, "minecraft");
    assert_eq!(id2.path, "stone");
}

#[test]
fn test_event_bus_order_and_no_lock() {
    let bus = EventBus::new();
    let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

    let o1 = order.clone();
    bus.register(20, move |_: &ClientTickEvent| {
        o1.lock().unwrap().push(2);
    });

    let o2 = order.clone();
    bus.register(10, move |_: &ClientTickEvent| {
        o2.lock().unwrap().push(1);
    });

    bus.post(&ClientTickEvent {
        phase: Phase::Start,
        tick_count: 42,
    });

    assert_eq!(
        *order.lock().unwrap(),
        vec![1, 2],
        "L'ordine di dispatch deve rispettare le priorità"
    );
}

#[test]
fn test_varint_golden_bytes() {
    let test_cases: &[(i32, &[u8])] = &[
        (0, &[0x00]),
        (1, &[0x01]),
        (2, &[0x02]),
        (127, &[0x7F]),
        (128, &[0x80, 0x01]),
        (255, &[0xFF, 0x01]),
        (25565, &[0xDD, 0xC7, 0x01]),
        (2147483647, &[0xFF, 0xFF, 0xFF, 0xFF, 0x07]),
        (-1, &[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]),
    ];

    for &(val, expected_bytes) in test_cases {
        let mut buf = Vec::new();
        let bytes_written = write_varint(&mut buf, val).unwrap();
        assert_eq!(bytes_written, expected_bytes.len());
        assert_eq!(
            &buf[..],
            expected_bytes,
            "VarInt per {val} non corrisponde ai golden bytes di Minecraft"
        );

        let mut cursor = Cursor::new(&buf);
        let read_back = read_varint(&mut cursor).unwrap();
        assert_eq!(read_back, val);
    }
}

#[test]
fn test_network_registry_dispatch() {
    let net = NetworkRegistry::new();
    let counter = std::sync::Arc::new(AtomicI32::new(0));

    let c = counter.clone();
    net.register_receiver("test:channel", move |data| {
        c.fetch_add(data.len() as i32, Ordering::SeqCst);
    });

    net.dispatch("test:channel", &[1, 2, 3, 4, 5]);
    assert_eq!(counter.load(Ordering::SeqCst), 5);
}
