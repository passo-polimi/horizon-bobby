#!/usr/bin/env python3
"""
Horizon Mod Porting — Ingestion & Static Analysis Tool (T1.1, T1.2, T1.3)
Analizza repository o JAR di mod Java Fabric, estrae metadati e genera mod-manifest.json validato.
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import sys
from pathlib import Path


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def find_files(root: Path, pattern: str):
    return list(root.glob(f"**/{pattern}"))


def parse_json_lenient(text: str):
    """Rimuove commenti // e /* */, variabili Gradle non quotate e trailing commas per parsing JSON tollerante."""
    text = re.sub(r"//.*", "", text)
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.DOTALL)
    # Sostituisce token non quotati tipo $authors con stringhe "$authors"
    text = re.sub(r':\s*\$([a-zA-Z0-9_]+)', r': "$\1"', text)
    text = re.sub(r",\s*([\]}])", r"\1", text)
    return json.loads(text)


def analyze_mod(source_dir: Path, mod_id: str, out_dir: Path):
    meta_dir = out_dir / "meta"
    meta_dir.mkdir(parents=True, exist_ok=True)

    print(f"[*] Ingestion per mod: {mod_id}")
    print(f"[*] Sorgente: {source_dir}")

    # 1. Trova fabric.mod.json
    fabric_mod_files = find_files(source_dir, "fabric.mod.json")
    if not fabric_mod_files:
        raise FileNotFoundError("Nessun fabric.mod.json trovato nel repository.")

    # Seleziona il fabric.mod.json principale (preferibilmente sotto fabric/ o src/main)
    fabric_mod_path = fabric_mod_files[0]
    for f in fabric_mod_files:
        if "fabric" in str(f).lower() or "main" in str(f).lower():
            fabric_mod_path = f
            break

    shutil.copy2(fabric_mod_path, meta_dir / "fabric.mod.json")
    print(f"[*] fabric.mod.json trovato: {fabric_mod_path}")

    with open(fabric_mod_path, "r", encoding="utf-8", errors="ignore") as f:
        fabric_data = parse_json_lenient(f.read())

    real_id = fabric_data.get("id", mod_id)
    name = fabric_data.get("name", real_id)
    version = fabric_data.get("version", "1.0.0")
    license_str = fabric_data.get("license", "Unknown")
    depends = fabric_data.get("depends", {})
    entrypoints = fabric_data.get("entrypoints", {})

    # Normalizza entrypoints
    norm_entrypoints = {}
    if isinstance(entrypoints, dict):
        for k, v in entrypoints.items():
            if isinstance(v, list):
                norm_entrypoints[k] = [
                    x if isinstance(x, str) else x.get("value", "") for x in v
                ]
            elif isinstance(v, str):
                norm_entrypoints[k] = [v]
    elif isinstance(entrypoints, list):
        norm_entrypoints["main"] = entrypoints

    # 2. Mixins config
    mixin_files = find_files(source_dir, "*mixins.json")
    mixins_meta = []
    for mf in mixin_files:
        shutil.copy2(mf, meta_dir / mf.name)
        try:
            with open(mf, "r", encoding="utf-8", errors="ignore") as f:
                mixins_meta.append((mf, parse_json_lenient(f.read())))
        except Exception as e:
            print(f"[!] Warning nel parsing di {mf.name}: {e}")

    # 3. Access wideners
    aw_files = find_files(source_dir, "*.accesswidener")
    for aw in aw_files:
        shutil.copy2(aw, meta_dir / aw.name)

    # 4. Scansione Java per Mixin e API
    java_files = find_files(source_dir, "*.java")
    print(f"[*] Analisi di {len(java_files)} file Java...")

    mixins_found = []
    fabric_callbacks = set()
    mc_refs = {}
    threads = set()
    reflections = set()
    network_channels = set()

    re_mixin = re.compile(r"@Mixin\s*\(\s*(?:value\s*=\s*)?([^)]+)\)")
    re_inject = re.compile(r"@Inject\s*\([^)]*method\s*=\s*\"([^\"]+)\"[^)]*\)")
    re_at = re.compile(r"@At\s*\(\s*(?:value\s*=\s*)?\"([^\"]+)\"")
    re_local_capture = re.compile(r"locals\s*=\s*LocalCapture")
    re_redirect = re.compile(r"@Redirect\s*\([^)]*method\s*=\s*\"([^\"]+)\"")
    re_modify = re.compile(r"@ModifyVariable\s*\([^)]*method\s*=\s*\"([^\"]+)\"")
    re_overwrite = re.compile(r"@Overwrite")
    re_accessor = re.compile(r"@Accessor")
    re_invoker = re.compile(r"@Invoker")

    re_mc_class = re.compile(r"\bnet\.minecraft\.([a-zA-Z0-9_$.]+)")
    re_callback = re.compile(r"\b(ClientTickEvents|HudRenderCallback|ClientPlayConnectionEvents|WorldRenderEvents|ScreenEvents|AttackBlockCallback|UseBlockCallback|ServerTickEvents)\.[a-zA-Z0-9_]+")
    re_thread = re.compile(r"\b(Thread|ExecutorService|ThreadPoolExecutor|ForkJoinPool|CompletableFuture|synchronized)\b")
    re_reflection = re.compile(r"\b(Class\.forName|getMethod|getDeclaredMethod|getField|getDeclaredField|setAccessible|MethodHandles|Unsafe)\b")
    re_channel = re.compile(r"(?:new\s+Identifier|Identifier\.of|ResourceLocation)\s*\(\s*\"([^\"]+)\"\s*,\s*\"([^\"]+)\"\s*\)")

    for jf in java_files:
        try:
            content = jf.read_text(encoding="utf-8", errors="ignore")
        except Exception:
            continue

        class_name = jf.stem

        # Mixin detection
        if "@Mixin" in content:
            m_target = re_mixin.search(content)
            target_str = m_target.group(1).replace(".class", "").strip() if m_target else "Unknown"

            # Check injects
            for inj in re_inject.finditer(content):
                m_name = inj.group(1)
                window = content[inj.start():min(len(content), inj.end() + 250)]
                at_m = re_at.search(window)
                at_type = at_m.group(1) if at_m else "UNKNOWN"
                has_locals = bool(re_local_capture.search(window))

                t_enum = "Inject_HEAD" if at_type == "HEAD" else ("Inject_RETURN" if at_type == "RETURN" else "Inject_OTHER")
                mixins_found.append({
                    "class": class_name,
                    "target": target_str,
                    "type": t_enum,
                    "method": m_name,
                    "captures_locals": has_locals
                })

            for red in re_redirect.finditer(content):
                mixins_found.append({
                    "class": class_name,
                    "target": target_str,
                    "type": "Redirect",
                    "method": red.group(1),
                    "captures_locals": False
                })

            for mod in re_modify.finditer(content):
                mixins_found.append({
                    "class": class_name,
                    "target": target_str,
                    "type": "ModifyVariable",
                    "method": mod.group(1),
                    "captures_locals": False
                })

            if re_overwrite.search(content):
                mixins_found.append({
                    "class": class_name,
                    "target": target_str,
                    "type": "Overwrite",
                    "method": "all",
                    "captures_locals": False
                })

            if re_accessor.search(content):
                mixins_found.append({
                    "class": class_name,
                    "target": target_str,
                    "type": "Accessor",
                    "method": "property",
                    "captures_locals": False
                })

            if re_invoker.search(content):
                mixins_found.append({
                    "class": class_name,
                    "target": target_str,
                    "type": "Invoker",
                    "method": "invoke",
                    "captures_locals": False
                })

        # Fabric Callbacks
        for cb in re_callback.finditer(content):
            fabric_callbacks.add(cb.group(0))

        # Minecraft class references
        for mc in re_mc_class.finditer(content):
            ref = mc.group(1).split(".")[0]
            mc_refs[ref] = mc_refs.get(ref, 0) + 1

        # Threading / Reflection
        for th in re_thread.finditer(content):
            threads.add(th.group(0))
        for rf in re_reflection.finditer(content):
            reflections.add(rf.group(0))

        # Channels
        for ch in re_channel.finditer(content):
            network_channels.add(f"{ch.group(1)}:{ch.group(2)}")

    # 5. Shaders
    shader_exts = [".glsl", ".vsh", ".fsh", ".comp", ".frag", ".vert", ".mesh", ".task", ".wgsl"]
    shaders = []
    for ext in shader_exts:
        for sf in find_files(source_dir, f"*{ext}"):
            shaders.append(str(sf.relative_to(source_dir)).replace("\\", "/"))

    # 6. Assets
    asset_files = []
    assets_dir = find_files(source_dir, "assets")
    for ad in assets_dir:
        if ad.is_dir():
            for af in ad.glob("**/*"):
                if af.is_file():
                    asset_files.append(str(af.relative_to(source_dir)).replace("\\", "/"))

    # Ordina i dati per determinismo
    mixins_found.sort(key=lambda x: (x["class"], x["target"], x["method"]))
    shaders.sort()
    asset_files.sort()
    sorted_channels = sorted(list(network_channels))
    sorted_callbacks = sorted(list(fabric_callbacks))

    # Filtra dipendenze stringhe
    clean_depends = {}
    for k, v in depends.items():
        if isinstance(v, str):
            clean_depends[k] = v
        elif isinstance(v, list):
            clean_depends[k] = ", ".join(str(x) for x in v)

    manifest = {
        "mod_id": real_id,
        "name": name,
        "version": version,
        "license": license_str,
        "dependencies": clean_depends,
        "entrypoints": norm_entrypoints,
        "mixins": mixins_found,
        "fabric_callbacks": sorted_callbacks,
        "minecraft_references": dict(sorted(mc_refs.items(), key=lambda item: item[1], reverse=True)[:50]),
        "network_channels": sorted_channels,
        "shaders": shaders,
        "assets": asset_files[:100],  # Limita ai primi 100 per leggibilità
        "threading_and_reflection": {
            "uses_threads": len(threads) > 0,
            "uses_reflection": len(reflections) > 0,
            "thread_references": sorted(list(threads)),
            "reflection_references": sorted(list(reflections))
        }
    }

    manifest_path = out_dir / "mod-manifest.json"
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)

    print(f"[OK] Manifest generato: {manifest_path}")
    print(f"[*] Mixins rilevati: {len(mixins_found)}")
    print(f"[*] Shaders rilevati: {len(shaders)}")
    print(f"[*] Callbacks Fabric: {len(sorted_callbacks)}")

    # Validazione contro lo schema
    validate_manifest(manifest_path)

    return manifest_path


def validate_manifest(manifest_path: Path):
    schema_path = Path(__file__).parent / "manifest_schema.json"
    with open(schema_path, "r", encoding="utf-8") as f:
        schema = json.load(f)
    with open(manifest_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    # Validazione campi obbligatori
    reqs = schema.get("required", [])
    for r in reqs:
        if r not in data:
            raise ValueError(f"Validazione fallita: campo obbligatorio '{r}' assente in {manifest_path}")

    # Validazione mixins
    for m in data.get("mixins", []):
        for mr in ["class", "target", "type"]:
            if mr not in m:
                raise ValueError(f"Validazione fallita: campo mixin '{mr}' assente in {m}")

    print("[OK] mod-manifest.json valida con successo contro manifest_schema.json!")


def main():
    parser = argparse.ArgumentParser(description="Horizon Ingest & Static Analyzer")
    parser.add_argument("--source", required=True, help="Percorso sorgente o repository")
    parser.add_argument("--mod", required=True, help="ID mod (es. nvidium, distanthorizons)")
    parser.add_argument("--out", default="work", help="Cartella di output (default: work)")

    args = parser.parse_args()
    source_p = Path(args.source).resolve()
    out_p = (Path(args.out) / args.mod).resolve()

    manifest_p = analyze_mod(source_p, args.mod, out_p)
    h = sha256_file(manifest_p)
    print(f"[*] SHA-256 Manifest: {h}")


if __name__ == "__main__":
    main()
