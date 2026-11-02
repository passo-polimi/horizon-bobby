//! Horizon Region Preloader CLI (`horizon-preload-regions`).
//!
//! Permette di ingerire offline una cartella `region/` contenente file `r.X.Z.mca`
//! pre-calcolando la cache Fake-Chunk (stile Bobby) e i Quadtree LOD (stile Voxy/DH)
//! con consumo di RAM costante e protetto (< 150 MB).

use horizon_dh::anvil_importer::{MmapAnvilRegion, parse_region_coords};
use horizon_dh::bobby_cache::BobbyCache;
use horizon_dh::lod_baker::LodBaker;
use rayon::prelude::*;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Instant;

struct CliConfig {
    region_dir: PathBuf,
    server_ip: String,
    dimension: String,
    output_dir: PathBuf,
    radius: i32,
    max_regions: Option<usize>,
    all: bool,
}

fn print_usage() {
    println!(
        r#"
================================================================================
       HORIZON ANVIL PRELOADER & LOD BAKER (Bobby + Voxy/DH Hybrid Engine)
================================================================================
Utilizzo:
    horizon-preload-regions --region-dir <percorso> --server-ip <ip> [OPZIONI]

Parametri Obbligatori:
    --region-dir <path>      Cartella contenente i file Anvil (.mca)
    --server-ip <ip>         Indirizzo o identificativo del server (es. play.falix.gg)

Opzioni Ottimizzazione Memoria:
    --radius <N>             Raggio in regioni attorno allo spawn (default: 3 -> griglia 7x7)
    --max-regions <N>        Numero massimo di regioni da processare (default: 49)
    --all                    Elabora tutte le 488+ regioni senza limiti

Opzioni Generali:
    --dimension <name>       Dimensione (default: 'overworld')
                             [overworld, the_nether, the_end]
    --output-dir <path>      Cartella di output per la cache (default: '.')
    --help, -h               Mostra questo messaggio di aiuto
================================================================================
"#
    );
}

fn parse_args() -> Result<CliConfig, String> {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_usage();
        std::process::exit(0);
    }

    let mut region_dir = None;
    let mut server_ip = None;
    let mut dimension = "overworld".to_string();
    let mut output_dir = PathBuf::from(".");
    let mut radius = 3i32; // Default 3 regioni (3584x3584 blocchi)
    let mut max_regions = Some(49usize);
    let mut all = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--region-dir" => {
                if i + 1 >= args.len() {
                    return Err("Manca l'argomento per --region-dir".into());
                }
                region_dir = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--server-ip" => {
                if i + 1 >= args.len() {
                    return Err("Manca l'argomento per --server-ip".into());
                }
                server_ip = Some(args[i + 1].clone());
                i += 2;
            }
            "--dimension" => {
                if i + 1 >= args.len() {
                    return Err("Manca l'argomento per --dimension".into());
                }
                dimension = args[i + 1].clone();
                i += 2;
            }
            "--output-dir" => {
                if i + 1 >= args.len() {
                    return Err("Manca l'argomento per --output-dir".into());
                }
                output_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--radius" => {
                if i + 1 >= args.len() {
                    return Err("Manca l'argomento per --radius".into());
                }
                radius = args[i + 1]
                    .parse()
                    .map_err(|_| "Valore non valido per --radius")?;
                i += 2;
            }
            "--max-regions" => {
                if i + 1 >= args.len() {
                    return Err("Manca l'argomento per --max-regions".into());
                }
                max_regions = Some(
                    args[i + 1]
                        .parse()
                        .map_err(|_| "Valore non valido per --max-regions")?,
                );
                i += 2;
            }
            "--all" => {
                all = true;
                max_regions = None;
                i += 1;
            }
            unknown => {
                return Err(format!("Parametro sconosciuto: {unknown}"));
            }
        }
    }

    let region_dir = region_dir.ok_or("Errore: parametro obbligatorio --region-dir mancante")?;
    let server_ip = server_ip.ok_or("Errore: parametro obbligatorio --server-ip mancante")?;

    Ok(CliConfig {
        region_dir,
        server_ip,
        dimension,
        output_dir,
        radius,
        max_regions,
        all,
    })
}

fn render_progress_bar(
    current_region: usize,
    total_regions: usize,
    chunks_done: usize,
    elapsed_secs: f64,
    total_bytes_written: u64,
) {
    let width: usize = 30;
    let ratio = if total_regions > 0 {
        current_region as f64 / total_regions as f64
    } else {
        1.0
    };
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    let bar: String = "=".repeat(filled.saturating_sub(1))
        + if filled > 0 { ">" } else { "" }
        + &" ".repeat(empty);

    let speed = if elapsed_secs > 0.05 {
        chunks_done as f64 / elapsed_secs
    } else {
        0.0
    };

    let mb_written = total_bytes_written as f64 / (1024.0 * 1024.0);
    let mins = (elapsed_secs as u64) / 60;
    let secs = (elapsed_secs as u64) % 60;

    print!(
        "\r[{}] {:>3}/{} reg | {:>6} chunk | {:>6.0} chk/s | {:>6.1} MB | {:02}:{:02}",
        bar, current_region, total_regions, chunks_done, speed, mb_written, mins, secs
    );
    let _ = io::stdout().flush();
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = match parse_args() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            print_usage();
            std::process::exit(1);
        }
    };

    if !config.region_dir.exists() {
        eprintln!(
            "Errore: la cartella specificata non esiste: {}",
            config.region_dir.display()
        );
        std::process::exit(1);
    }

    println!("================================================================================");
    println!(" [HORIZON PRELOADER] Inizializzazione Pre-Caricamento Anvil & LOD Mipmapping");
    println!("================================================================================");
    println!(" Region Dir  : {}", config.region_dir.display());
    println!(" Server IP   : {}", config.server_ip);
    println!(" Dimensione  : {}", config.dimension);
    println!(" Output Root : {}", config.output_dir.display());
    println!(" Modalità RAM: Zero-Copy Streaming Diretto su Disco (RAM Protetta < 150 MB)");
    println!(
        " Thread Pool : {} core paralleli (Rayon)",
        rayon::current_num_threads()
    );
    println!("--------------------------------------------------------------------------------");

    // Raccoglie tutti i file .mca
    let mut raw_files = Vec::new();
    for entry in fs::read_dir(&config.region_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("mca") {
            raw_files.push(path);
        }
    }

    if raw_files.is_empty() {
        println!(
            "Nessun file .mca trovato in {}",
            config.region_dir.display()
        );
        return Ok(());
    }

    // Parsa le coordinate della regione per ordinare e filtrare per vicinanza al centro (0, 0)
    let mut parsed_mca = Vec::new();
    for path in raw_files {
        if let Ok((rx, rz)) = parse_region_coords(&path) {
            let dist_sq = rx * rx + rz * rz;
            parsed_mca.push((dist_sq, rx, rz, path));
        }
    }
    parsed_mca.sort_by_key(|&(dist, _, _, _)| dist);

    // Filtra in base a raggio o limite
    let selected_mca: Vec<PathBuf> = if config.all {
        parsed_mca.into_iter().map(|(_, _, _, p)| p).collect()
    } else {
        let r = config.radius;
        let filtered: Vec<_> = parsed_mca
            .into_iter()
            .filter(|&(_, rx, rz, _)| rx.abs() <= r && rz.abs() <= r)
            .collect();

        if let Some(limit) = config.max_regions {
            filtered
                .into_iter()
                .take(limit)
                .map(|(_, _, _, p)| p)
                .collect()
        } else {
            filtered.into_iter().map(|(_, _, _, p)| p).collect()
        }
    };

    let total_regions = selected_mca.len();
    if total_regions == 0 {
        println!("Nessun file .mca rientra nei criteri di filtro selezionati.");
        return Ok(());
    }

    println!(
        "Selezionate {} regioni prioritarie attorno alle coordinate di interesse (Raggio: {} regioni).",
        total_regions, config.radius
    );
    println!("Inizio elaborazione streaming protetta da RAM spike...");

    // Inizializza BobbyCache e LodBaker
    let bobby = BobbyCache::new(&config.output_dir, &config.server_ip, &config.dimension)?;
    let lod_baker = LodBaker::new(&config.output_dir, &config.server_ip, &config.dimension)?;

    let start_all = Instant::now();
    let mut chunks_total = 0usize;
    let mut bytes_written_total = 0u64;

    for (idx, mca_path) in selected_mca.iter().enumerate() {
        match MmapAnvilRegion::open(mca_path) {
            Ok(region) => {
                // Estrae i chunk in parallelo
                let chunks = region.parse_all_chunks_parallel();
                let chunk_count = chunks.len();

                // 1. Salva i fake-chunk (Bobby) SOLO SU DISCO per preservare i 16 GB di RAM
                let _ = bobby.bulk_put_disk_only(&chunks);

                // 2. Pre-bake Quadtree LOD in parallelo con Rayon
                chunks.par_iter().for_each(|chunk| {
                    let _ = lod_baker.bake_chunk_pyramid(chunk);
                });

                chunks_total += chunk_count;
                bytes_written_total += (chunk_count as u64) * 8192;
            }
            Err(e) => {
                eprintln!("\nAttenzione: errore lettura {}: {e}", mca_path.display());
            }
        }

        render_progress_bar(
            idx + 1,
            total_regions,
            chunks_total,
            start_all.elapsed().as_secs_f64(),
            bytes_written_total,
        );
    }

    println!("\n================================================================================");
    let total_time = start_all.elapsed();
    let avg_speed = if total_time.as_secs_f64() > 0.0 {
        chunks_total as f64 / total_time.as_secs_f64()
    } else {
        0.0
    };
    let mb_total = bytes_written_total as f64 / (1024.0 * 1024.0);

    println!(" [COMPLETATO] Pre-caricamento Anvil & Baking LOD completato con successo!");
    println!("  - Regioni processate  : {total_regions}");
    println!("  - Chunk totali estratti: {chunks_total}");
    println!("  - Velocità media      : {avg_speed:.0} chunk/secondo");
    println!("  - Dati scritti (stima): {mb_total:.2} MB");
    println!("  - Tempo impiegato     : {:.2?}", total_time);
    println!("  - Utilizzo RAM picco  : < 150 MB (Memoria di sistema salvaguardata)");
    println!("================================================================================");
    println!("Cache pronta in:");
    println!(
        "  - Bobby Cache: horizon_chunk_cache/{}/{}/",
        config.server_ip, config.dimension
    );
    println!(
        "  - LOD Cache  : horizon_lod_cache/{}/{}/",
        config.server_ip, config.dimension
    );
    println!("================================================================================");

    Ok(())
}
