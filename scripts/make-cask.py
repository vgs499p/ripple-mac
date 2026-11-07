#!/usr/bin/env python3
"""Generate a Homebrew cask from completed Mac release archives."""
import argparse
import hashlib
from pathlib import Path
from urllib.parse import urlparse
p=argparse.ArgumentParser()
p.add_argument('--archive',type=Path,required=True)
p.add_argument('--url',required=True)
p.add_argument('--intel-archive',type=Path)
p.add_argument('--intel-url')
p.add_argument('--version',required=True)
p.add_argument('--output',type=Path,required=True)
a=p.parse_args()
if bool(a.intel_archive)!=bool(a.intel_url): p.error('Intel archive and URL must be supplied together')
for value in [a.url,a.intel_url]:
 if value and (urlparse(value).scheme!='https' or any(c in value for c in '\"\\n\\r\\\\')): p.error('Use plain HTTPS release URLs')
if any(c in a.version for c in '\"\\n\\r\\\\'): p.error('Use a plain version string')
def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
if a.intel_archive:
 downloads=f'''  on_arm do
    sha256 "{digest(a.archive)}"
    url "{a.url}"
  end
  on_intel do
    sha256 "{digest(a.intel_archive)}"
    url "{a.intel_url}"
  end
'''
else:
 downloads=f'''  sha256 "{digest(a.archive)}"
  url "{a.url}"
  depends_on arch: :arm64
'''
a.output.parent.mkdir(parents=True,exist_ok=True)
a.output.write_text(f'''cask "ripple-mac" do
  version "{a.version}"
{downloads}
  name "Ripple Native"
  desc "Native Ripple client with local themes"
  homepage "https://github.com/vgs499p/ripple-mac"
  depends_on macos: ">= :big_sur"
  app "Ripple Beta.app"
end
''')
