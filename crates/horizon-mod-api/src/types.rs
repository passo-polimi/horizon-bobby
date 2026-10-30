//! Tipi fondamentali di Minecraft con semantica Java documentata (Task T2.1).
//! I metodi mantengono la corrispondenza con le classi vanilla deoffuscate con Mojmap.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Rappresenta una coordinata discreta nel mondo voxel (Minecraft `BlockPos`).
///
/// Semantica Java (Mojmap: `net.minecraft.core.BlockPos`):
/// - Coordinate a 32-bit `x`, `y`, `z` con aritmetica wrapping per evitare panic su overflow.
/// - Serializzazione in long a 64 bit identica all'algoritmo di Minecraft vanilla:
///   `((x & 0x3FFFFFF) << 38) | ((z & 0x3FFFFFF) << 12) | (y & 0xFFF)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const ZERO: Self = Self { x: 0, y: 0, z: 0 };

    // PORT: BlockPos#<init>(int, int, int)
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    // PORT: BlockPos#offset(int, int, int)
    #[inline]
    pub const fn offset(self, dx: i32, dy: i32, dz: i32) -> Self {
        Self {
            x: self.x.wrapping_add(dx),
            y: self.y.wrapping_add(dy),
            z: self.z.wrapping_add(dz),
        }
    }

    // PORT: BlockPos#relative(Direction)
    #[inline]
    pub fn relative(self, direction: Direction) -> Self {
        let [dx, dy, dz] = direction.normal();
        self.offset(dx, dy, dz)
    }

    // PORT: BlockPos#asLong()
    #[inline]
    pub const fn as_long(self) -> i64 {
        let x = (self.x as i64) & 0x3FFFFFF;
        let y = (self.y as i64) & 0xFFF;
        let z = (self.z as i64) & 0x3FFFFFF;
        (x << 38) | (z << 12) | y
    }

    // PORT: BlockPos#of(long)
    #[inline]
    pub const fn from_long(packed: i64) -> Self {
        let x = packed >> 38;
        let y = (packed << 52) >> 52;
        let z = (packed << 26) >> 38;
        Self {
            x: x as i32,
            y: y as i32,
            z: z as i32,
        }
    }

    // PORT: BlockPos#distManhattan(Vec3i)
    #[inline]
    pub fn dist_manhattan(self, other: Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()
    }
}

/// Coordinata di un chunk 16x16 nel piano orizzontale (Minecraft `ChunkPos`).
///
/// Semantica Java (Mojmap: `net.minecraft.world.level.ChunkPos`):
/// - Coordinate `x` e `z`.
/// - Impaccamento in u64/i64 via `((x as u64) << 32) | (z as u64 & 0xFFFFFFFF)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    // PORT: ChunkPos#<init>(int, int)
    #[inline]
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    // PORT: ChunkPos#from(BlockPos)
    #[inline]
    pub const fn from_block_pos(pos: BlockPos) -> Self {
        Self {
            x: pos.x >> 4,
            z: pos.z >> 4,
        }
    }

    // PORT: ChunkPos#asLong()
    #[inline]
    pub const fn as_long(self) -> i64 {
        ((self.x as i64) << 32) | ((self.z as i64) & 0xFFFFFFFF)
    }

    // PORT: ChunkPos#<init>(long)
    #[inline]
    pub const fn from_long(packed: i64) -> Self {
        Self {
            x: (packed >> 32) as i32,
            z: packed as i32,
        }
    }

    // PORT: ChunkPos#getMinBlockX()
    #[inline]
    pub const fn min_block_x(self) -> i32 {
        self.x << 4
    }

    // PORT: ChunkPos#getMinBlockZ()
    #[inline]
    pub const fn min_block_z(self) -> i32 {
        self.z << 4
    }

    // PORT: ChunkPos#chebyshevDistance(ChunkPos)
    #[inline]
    pub fn distance_chebyshev(self, other: Self) -> i32 {
        (self.x - other.x).abs().max((self.z - other.z).abs())
    }
}

/// Vettore continuo tridimensionale (Minecraft `Vec3`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    // PORT: Vec3#<init>(double, double, double)
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    // PORT: Vec3#add(double, double, double)
    #[inline]
    pub fn add(self, dx: f64, dy: f64, dz: f64) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            z: self.z + dz,
        }
    }

    // PORT: Vec3#distanceTo(Vec3)
    #[inline]
    pub fn distance_to(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Direzione cardinale nello spazio a 6 facce (Minecraft `Direction`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Direction {
    pub const ALL: [Direction; 6] = [
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ];

    // PORT: Direction#getOpposite()
    #[inline]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::North => Self::South,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
        }
    }

    // PORT: Direction#getNormal()
    #[inline]
    pub const fn normal(self) -> [i32; 3] {
        match self {
            Self::Down => [0, -1, 0],
            Self::Up => [0, 1, 0],
            Self::North => [0, 0, -1],
            Self::South => [0, 0, 1],
            Self::West => [-1, 0, 0],
            Self::East => [1, 0, 0],
        }
    }
}

/// Identificatore con namespace (Minecraft `ResourceLocation` / `Identifier`).
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Identifier {
    pub namespace: String,
    pub path: String,
}

impl Identifier {
    // PORT: Identifier#<init>(String, String)
    pub fn new(namespace: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            path: path.into(),
        }
    }

    // PORT: Identifier#parse(String)
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.split_once(':') {
            Some((ns, p)) => Ok(Self::new(ns, p)),
            None => Ok(Self::new("minecraft", s)),
        }
    }

    // PORT: Identifier#of(String)
    pub fn of(s: &str) -> Self {
        Self::parse(s).unwrap_or_else(|_| Self::new("minecraft", "empty"))
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

impl fmt::Debug for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Identifier(\"{}:{}\")", self.namespace, self.path)
    }
}

/// Rappresentazione di testo formattato (Minecraft `Component`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TextComponent {
    pub text: String,
    pub color_rgb: Option<u32>,
    pub bold: bool,
    pub italic: bool,
}

impl TextComponent {
    // PORT: Component#literal(String)
    pub fn literal(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color_rgb: None,
            bold: false,
            italic: false,
        }
    }
}

