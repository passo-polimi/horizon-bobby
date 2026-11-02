//! Risoluzione Conflitti Live (Network Delta Override).
//!
//! Quando il client Horizon riceve dal server pacchetti autoritativi
//! (`ClientboundLevelChunkWithLightPacket` o `BlockUpdatePacket`):
//! 1. Sostituisce immediatamente in memoria il Fake-Chunk di Livello 1 (Bobby) con il chunk reale.
//! 2. Salva su disco sia la cache completa del chunk sia la piramide Quadtree LOD (Livello 2),
//!    mantenendo il mondo client e i LOD sempre sincronizzati con le modifiche del server.

use crate::anvil_importer::{BlockStateNbt, ParsedChunk, UnpackedSection};
use crate::bobby_cache::BobbyCache;
use crate::lod_baker::LodBaker;
use std::io;
use std::sync::Arc;

/// Risolutore di delta di rete per la sincronizzazione autoritativa della cache.
pub struct DeltaResolver {
    bobby: Arc<BobbyCache>,
    lod_baker: Arc<LodBaker>,
}

impl DeltaResolver {
    /// Crea un nuovo resolver collegato alla cache Bobby e al LodBaker della sessione.
    pub fn new(bobby: Arc<BobbyCache>, lod_baker: Arc<LodBaker>) -> Self {
        Self { bobby, lod_baker }
    }

    /// Ritorna un riferimento alla cache Bobby.
    pub fn bobby(&self) -> &Arc<BobbyCache> {
        &self.bobby
    }

    /// Ritorna un riferimento al LodBaker.
    pub fn lod_baker(&self) -> &Arc<LodBaker> {
        &self.lod_baker
    }

    /// Gestisce l'arrivo di un chunk completo inviato dal server.
    ///
    /// Sovrascrive il fake-chunk sia nella cache in memoria che su disco,
    /// e ricalcola simultaneamente la piramide LOD 0 -> 4.
    pub fn on_network_chunk(&self, chunk: Arc<ParsedChunk>) -> io::Result<()> {
        // 1. Aggiorna Bobby Cache (in RAM e su disco)
        self.bobby.put(chunk.clone())?;

        // 2. Ricalcola piramide Quadtree LOD
        self.lod_baker.bake_chunk_pyramid(&chunk)?;

        Ok(())
    }

    /// Gestisce un singolo aggiornamento di blocco (`BlockUpdatePacket`).
    ///
    /// Coordinate globali `(x, y, z)`:
    /// - Se il chunk corrispondente è presente nella cache, ne aggiorna la sezione e la heightmap.
    /// - Salva il chunk modificato e ricalcola le colonne LOD interessate.
    pub fn on_block_update(&self, x: i32, y: i32, z: i32, block_name: &str) -> io::Result<bool> {
        let chunk_x = x.div_euclid(16);
        let chunk_z = z.div_euclid(16);

        let Some(existing) = self.bobby.get(chunk_x, chunk_z) else {
            return Ok(false);
        };

        let mut updated = (*existing).clone();
        let local_x = (x.rem_euclid(16)) as usize;
        let local_z = (z.rem_euclid(16)) as usize;
        let sec_y = (y.div_euclid(16)) as i8;
        let local_y = (y.rem_euclid(16)) as usize;

        // Trova o crea la sezione
        let sec_idx = updated.sections.iter().position(|s| s.y == sec_y);

        let sec = match sec_idx {
            Some(idx) => &mut updated.sections[idx],
            None => {
                // Se la sezione non esiste, la aggiungiamo
                updated.sections.push(UnpackedSection {
                    y: sec_y,
                    palette: vec![BlockStateNbt {
                        name: "minecraft:air".to_string(),
                        properties: None,
                    }],
                    blocks: [0; 4096],
                    biome_palette: vec!["minecraft:plains".to_string()],
                    biomes: [0; 64],
                    block_light: None,
                    sky_light: None,
                });
                updated.sections.sort_by_key(|s| s.y);
                let new_pos = updated.sections.iter().position(|s| s.y == sec_y).unwrap();
                &mut updated.sections[new_pos]
            }
        };

        // Inserisce il blocco nella palette della sezione se necessario
        let palette_pos = match sec.palette.iter().position(|b| b.name == block_name) {
            Some(p) => p as u16,
            None => {
                sec.palette.push(BlockStateNbt {
                    name: block_name.to_string(),
                    properties: None,
                });
                (sec.palette.len() - 1) as u16
            }
        };

        let block_idx = (local_y << 8) | (local_z << 4) | local_x;
        sec.blocks[block_idx] = palette_pos;

        // Aggiorna heightmap superficie se applicabile
        if let Some(surface) = &mut updated.heightmap_surface {
            let col = local_z * 16 + local_x;
            if block_name != "minecraft:air" && y as i16 >= surface[col] {
                surface[col] = (y + 1) as i16;
            } else if block_name == "minecraft:air" && y as i16 == surface[col] - 1 {
                // Ricalcola altezza scendendo verso il basso
                let mut found = -64i16;
                'scan: for s in updated.sections.iter().rev() {
                    for sy in (0..16).rev() {
                        let cur_y = (s.y as i16 * 16) + sy as i16;
                        if cur_y >= y as i16 {
                            continue;
                        }
                        let b_idx = (sy << 8) | (local_z << 4) | local_x;
                        let p = s.blocks[b_idx];
                        if let Some(entry) = s.palette.get(p as usize) {
                            if entry.name != "minecraft:air" {
                                found = cur_y + 1;
                                break 'scan;
                            }
                        }
                    }
                }
                surface[col] = found;
            }
        }

        // Salva le modifiche autoritative
        let arc_chunk = Arc::new(updated);
        self.bobby.put(arc_chunk.clone())?;
        self.lod_baker.bake_chunk_pyramid(&arc_chunk)?;

        Ok(true)
    }

    /// Converte e applica un chunk in formato nativo `horizon_client_render::chunk::Chunk`.
    pub fn on_client_render_chunk(
        &self,
        chunk: &horizon_client_render::chunk::Chunk,
    ) -> io::Result<()> {
        let mut sections = Vec::with_capacity(chunk.sections.len());
        for (i, sec_arc) in chunk.sections.iter().enumerate() {
            let sy = (i as i8) - 4; // MIN_Y = -64 -> offset -4 sezioni
            let mut blocks_u32 = [0u32; 4096];
            sec_arc.unpack(&mut blocks_u32);

            // Mappa blocchi unici in una palette
            let mut palette = Vec::new();
            let mut state_to_palette = std::collections::HashMap::new();
            let mut blocks = [0u16; 4096];

            for (idx, &st) in blocks_u32.iter().enumerate() {
                let p_idx = *state_to_palette.entry(st).or_insert_with(|| {
                    let id = palette.len() as u16;
                    palette.push(BlockStateNbt {
                        name: format!("state_{st}"),
                        properties: None,
                    });
                    id
                });
                blocks[idx] = p_idx;
            }

            sections.push(UnpackedSection {
                y: sy,
                palette,
                blocks,
                biome_palette: vec!["minecraft:plains".to_string()],
                biomes: [0; 64],
                block_light: None,
                sky_light: None,
            });
        }

        let parsed = ParsedChunk {
            x: chunk.x,
            z: chunk.z,
            min_section_y: -4,
            inhabited_time: 0,
            last_update: 0,
            sections,
            heightmap_surface: None,
            heightmap_ocean: None,
        };

        self.on_network_chunk(Arc::new(parsed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_delta_resolver_live_override() {
        let dir = tempdir().unwrap();
        let bobby = Arc::new(BobbyCache::new(dir.path(), "test_srv", "overworld").unwrap());
        let lod_baker = Arc::new(LodBaker::new(dir.path(), "test_srv", "overworld").unwrap());
        let resolver = DeltaResolver::new(bobby.clone(), lod_baker.clone());

        let initial_chunk = Arc::new(ParsedChunk {
            x: 10,
            z: 20,
            min_section_y: -4,
            inhabited_time: 1000,
            last_update: 2000,
            sections: vec![UnpackedSection {
                y: 0,
                palette: vec![BlockStateNbt {
                    name: "minecraft:stone".to_string(),
                    properties: None,
                }],
                blocks: [0; 4096],
                biome_palette: vec!["minecraft:plains".to_string()],
                biomes: [0; 64],
                block_light: None,
                sky_light: None,
            }],
            heightmap_surface: Some([10; 256]),
            heightmap_ocean: None,
        });

        // 1. Ricezione chunk iniziale (LOD 0 ha altezza 10)
        resolver.on_network_chunk(initial_chunk).unwrap();
        assert!(bobby.contains(10, 20));
        let tile = lod_baker.load_tile(0, 10, 20).unwrap();
        assert_eq!(tile.samples[0].height, 10);

        // 2. Modifica autoritativa: piazzamento blocco ad altezza y=64 (coordinata x=162, z=323 -> chunk 10, 20)
        let modified = resolver
            .on_block_update(162, 64, 323, "minecraft:gold_block")
            .unwrap();
        assert!(modified);

        // Verifica che il chunk in Bobby Cache sia stato aggiornato
        let updated_chunk = bobby.get(10, 20).unwrap();
        let col = 3 * 16 + 2; // (323 % 16 = 3, 162 % 16 = 2)
        assert_eq!(updated_chunk.heightmap_surface.as_ref().unwrap()[col], 65);

        // Verifica che anche il tile LOD sia stato aggiornato
        let updated_tile = lod_baker.load_tile(0, 10, 20).unwrap();
        assert_eq!(updated_tile.samples[col].height, 65);
    }
}
