#!/usr/bin/env python3

from glob import glob
from hashlib import file_digest
import json
from os import path, rename
from typing import List

PUBLISH_DIR = "publish"
PUBLIC_DIR = path.join(PUBLISH_DIR, "public")
INCLUDE = ["icons/*.svg", "js/*.js", "main.css"]


def hash_files():
    assets = {p: path_with_hash(p) for p in find_assets()}
    asset_map = {"/" + plain: "/" + hashed for plain, hashed in assets.items()}
    import_map = {
        "imports": {
            "/" + plain: "/" + hashed
            for plain, hashed in assets.items()
            if plain.endswith(".js")
        }
    }

    with open(path.join(PUBLISH_DIR, "import-map.json"), "w+") as f:
        json.dump(import_map, f)

    with open(path.join(PUBLISH_DIR, "asset-map.json"), "w+") as f:
        json.dump(asset_map, f)

    for src, dest in assets.items():
        rename(path.join(PUBLIC_DIR, src), path.join(PUBLIC_DIR, dest))


def find_assets() -> List[str]:
    return [f for pattern in INCLUDE for f in glob(pattern, root_dir=PUBLIC_DIR)]


def short_hash(asset_path: str) -> str:
    with open(path.join(PUBLIC_DIR, asset_path), "rb") as f:
        return file_digest(f, "sha256").hexdigest()[:8]


def path_with_hash(asset_path: str) -> str:
    root, ext = path.splitext(asset_path)
    return f"{root}-{short_hash(asset_path)}{ext}"


if __name__ == "__main__":
    hash_files()
