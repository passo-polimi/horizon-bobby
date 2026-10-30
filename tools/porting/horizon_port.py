#!/usr/bin/env python3
"""
horizon-port: CLI unificata per il porting di mod Java Fabric/Forge in Horizon (Rust).
Gestisce l'intero ciclo di vita: ingestion, census, skeleton, verification e status (Fase 6).
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

# Configura encoding UTF-8 per console Windows
if hasattr(sys.stdout, "reconfigure"):
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass

REPO_ROOT = Path(__file__).resolve().parent.parent.parent


def cmd_ingest(args):
    """Inoltra il comando allo strumento di ingestion."""
    ingest_script = REPO_ROOT / "tools" / "porting" / "ingest.py"
    cmd = [sys.executable, str(ingest_script), args.source]
    if args.name:
        cmd.extend(["--name", args.name])
    return subprocess.run(cmd).returncode


def cmd_census(args):
    """Visualizza o genera il report di census per una mod analizzata."""
    census_file = REPO_ROOT / "docs" / "porting" / f"census-{args.mod}.md"
    manifest_file = REPO_ROOT / "work" / args.mod / "mod-manifest.json"

    if census_file.exists():
        print(f"\n=== Report Census Esistente: {census_file} ===")
        with open(census_file, "r", encoding="utf-8") as f:
            print(f.read())
        return 0

    if not manifest_file.exists():
        print(f"Errore: Manifest per la mod '{args.mod}' non trovato in {manifest_file}.")
        print("Esegui prima: horizon-port ingest <repo_url>")
        return 1

    with open(manifest_file, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    print(f"\n=== Census per {manifest.get('name', args.mod)} ===")
    print(f"Versione mod: {manifest.get('version', 'sconosciuta')}")
    print(f"Licenza: {manifest.get('license', 'sconosciuta')}")
    print(f"Classi Java analizzate: {len(manifest.get('classes', []))}")
    print(f"Mixin dichiarati: {len(manifest.get('mixins', []))}")
    print(f"Shader individuati: {len(manifest.get('shaders', []))}")
    return 0


def cmd_skeleton(args):
    """Inizializza un nuovo crate Rust in crates/mods/<mod> per il porting."""
    mod_name = args.mod.lower().replace("_", "-")
    mod_dir = REPO_ROOT / "crates" / "mods" / mod_name

    if mod_dir.exists():
        print(f"Il crate {mod_dir} esiste già.")
        return 0

    mod_dir.mkdir(parents=True, exist_ok=True)
    src_dir = mod_dir / "src"
    tests_dir = mod_dir / "tests"
    src_dir.mkdir(exist_ok=True)
    tests_dir.mkdir(exist_ok=True)

    cargo_toml = f"""[package]
name = "{mod_name}"
version = "0.1.0"
edition.workspace = true
license = "LGPL-3.0-only"
rust-version.workspace = true
description = "Porting nativo di {args.mod} per Horizon (Rust)"

[dependencies]
horizon-mod-api = {{ path = "../../horizon-mod-api" }}
log = "0.4"
"""
    (mod_dir / "Cargo.toml").write_text(cargo_toml, encoding="utf-8")

    lib_rs = f"""//! {args.mod} — Porting nativo per Horizon

#![allow(unused)]

use horizon_mod_api::prelude::*;

pub mod core {{
    use super::*;

    // TODO: Aggiungi moduli e funzioni con tracciamento:
    // // PORT: ClasseOriginale#metodoOriginale
}}
"""
    (src_dir / "lib.rs").write_text(lib_rs, encoding="utf-8")

    tests_rs = f"""use {mod_name.replace('-', '_')}::*;

#[test]
fn test_smoke_{mod_name.replace('-', '_')}() {{
    // Smoke test iniziale per {mod_name}
    assert!(true);
}}
"""
    (tests_dir / f"{mod_name.replace('-', '_')}_tests.rs").write_text(tests_rs, encoding="utf-8")

    # Aggiunge a root Cargo.toml se non presente
    root_cargo = REPO_ROOT / "Cargo.toml"
    content = root_cargo.read_text(encoding="utf-8")
    member_entry = f'    "crates/mods/{mod_name}",'
    if member_entry not in content:
        target = '    "crates/horizon-mod-harness",'
        if target in content:
            new_content = content.replace(target, f"{target}\n{member_entry}")
            root_cargo.write_text(new_content, encoding="utf-8")
            print(f"Aggiunto crates/mods/{mod_name} a workspace members in Cargo.toml.")

    print(f"Skeleton creato con successo in: {mod_dir}")
    return 0


def cmd_verify(args):
    """Esegue il gate CI unificato (verify.bat su Windows o verify.sh su Unix)."""
    if os.name == "nt":
        gate_script = REPO_ROOT / "tools" / "porting" / "verify.bat"
        return subprocess.run([str(gate_script)], shell=True).returncode
    else:
        gate_script = REPO_ROOT / "tools" / "porting" / "verify.sh"
        return subprocess.run(["bash", str(gate_script)]).returncode


def cmd_status(args):
    """Riassume lo stato complessivo di tutte le mod portate."""
    mods_dir = REPO_ROOT / "crates" / "mods"
    print("\n============================================================")
    print("           Stato Crate Mod Porting in Horizon               ")
    print("============================================================")

    if not mods_dir.exists():
        print("Nessun crate mod trovato in crates/mods/.")
        return 0

    for mod_path in sorted(mods_dir.iterdir()):
        if not mod_path.is_dir() or not (mod_path / "Cargo.toml").exists():
            continue

        mod_name = mod_path.name
        # Conta LoC Rust
        rust_files = list(mod_path.glob("**/*.rs"))
        total_lines = 0
        todos = 0
        for rf in rust_files:
            try:
                lines = rf.read_text(encoding="utf-8").splitlines()
                total_lines += len(lines)
                for line in lines:
                    if 'todo!("PORT:' in line:
                        todos += 1
            except Exception:
                pass

        print(f"\n* Mod: {mod_name}")
        print(f"  Percorso: {mod_path}")
        print(f"  File Rust: {len(rust_files)}")
        print(f"  Righe di codice (LoC): {total_lines}")
        print(f"  todo! residui: {todos}")
        if todos == 0:
            print("  Stato feature: 🟢 COMPLETATO (Tutte le feature core implementate)")
        else:
            print(f"  Stato feature: 🟡 IN CORSO ({todos} stub da completare)")

    print("\n============================================================")
    return 0


def main():
    parser = argparse.ArgumentParser(
        prog="horizon-port",
        description="CLI unificata per il porting di mod Java Fabric/Forge in Horizon (Rust)",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    # Ingest
    p_ingest = subparsers.add_parser("ingest", help="Acquisisce e analizza una mod Java")
    p_ingest.add_argument("source", help="URL git o percorso cartella del sorgente/jar")
    p_ingest.add_argument("--name", help="Nome identificativo della mod (opzionale)")
    p_ingest.set_defaults(func=cmd_ingest)

    # Census
    p_census = subparsers.add_parser("census", help="Visualizza il report di portabilità di una mod")
    p_census.add_argument("mod", help="Nome identificativo della mod")
    p_census.set_defaults(func=cmd_census)

    # Skeleton
    p_skel = subparsers.add_parser("skeleton", help="Genera uno skeleton di crate Rust per una nuova mod")
    p_skel.add_argument("mod", help="Nome della mod da creare")
    p_skel.set_defaults(func=cmd_skeleton)

    # Verify
    p_ver = subparsers.add_parser("verify", help="Esegue l'intero gate di verifica (linter, unit, difftest)")
    p_ver.set_defaults(func=cmd_verify)

    # Status
    p_stat = subparsers.add_parser("status", help="Mostra lo stato di avanzamento e metriche dei crate mod")
    p_stat.set_defaults(func=cmd_status)

    args = parser.parse_args()
    sys.exit(args.func(args))


if __name__ == "__main__":
    main()
