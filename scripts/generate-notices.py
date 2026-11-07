#!/usr/bin/env python3
"""Collect dependency license texts from the locked, downloaded Cargo sources."""
import json, subprocess
from pathlib import Path
metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1']))
parts=['Third-party Rust dependencies for Ripple desktop.\nApplication source: GPL-2.0-or-later. Desktop binaries: GPL-3.0-or-later.\nDependency versions and checksums are pinned by Cargo.lock.\n']
for package in sorted(metadata['packages'],key=lambda p:(p['name'],p['version'])):
 if not package['source']: continue
 root=Path(package['manifest_path']).parent
 parts.append(f"\n{package['name']} {package['version']} — {package.get('license') or 'see license file'}\nSource: https://crates.io/crates/{package['name']}/{package['version']}\n")
 paths={p for pattern in ['LICENSE*','LICENCE*','COPYING*','NOTICE*','UNLICENSE*'] for p in root.glob(pattern) if p.is_file()}
 if package.get('license_file'): paths.add(root/package['license_file'])
 for path in sorted(paths):
  if path.is_file(): parts.append('\n'+path.name+'\n'+path.read_text(errors='replace'))
Path('THIRD-PARTY-NOTICES.txt').write_text('\n'.join(parts))
