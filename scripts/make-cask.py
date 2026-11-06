#!/usr/bin/env python3
"""Generate a Homebrew cask from the final, verified release archive."""
import argparse, hashlib
from pathlib import Path
from urllib.parse import urlparse
p=argparse.ArgumentParser();p.add_argument('--archive',type=Path,required=True);p.add_argument('--url',required=True);p.add_argument('--version',required=True);p.add_argument('--output',type=Path,required=True);a=p.parse_args()
if urlparse(a.url).scheme!='https' or any(c in a.url+a.version for c in ['"','\n','\r','\\']):p.error('Use an HTTPS release URL and a plain version string')
sha=hashlib.sha256(a.archive.read_bytes()).hexdigest()
a.output.parent.mkdir(parents=True,exist_ok=True)
a.output.write_text(f'''cask "ripple-mac" do
  version "{a.version}"
  sha256 "{sha}"

  url "{a.url}"
  name "Ripple Native"
  desc "Native Ripple client with local themes"
  homepage "https://github.com/vgs499p/ripple-mac"

  depends_on arch: :arm64
  depends_on macos: ">= :big_sur"
  app "Ripple Beta.app"
end
''')
