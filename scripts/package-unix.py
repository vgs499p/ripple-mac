#!/usr/bin/env python3
"""Package a native build. Run on the target OS, never relabel foreign binaries."""
import argparse, json, os, platform, shutil, subprocess, tarfile, tempfile
from pathlib import Path
p=argparse.ArgumentParser(); p.add_argument("--format",choices=["tar","deb","rpm"],default="tar"); p.add_argument("--binary",type=Path,default=Path("target/release/ripple")); a=p.parse_args()
root=Path(__file__).resolve().parent.parent
version=json.loads(subprocess.check_output(["cargo","metadata","--no-deps","--format-version","1"],cwd=root))["packages"][0]["version"]
glibc_min=os.environ.get("RIPPLE_GLIBC_MIN","2.35")
if not all(part.isdigit() for part in glibc_min.split(".")): raise SystemExit("Invalid glibc minimum")
recommendations="xdg-utils" if os.environ.get("RIPPLE_NOTICE_FEATURES")=="software" else "mesa-vulkan-drivers, xdg-utils"
osname=platform.system().lower(); arch=platform.machine().lower(); out=root/"dist"; out.mkdir(exist_ok=True)
with tempfile.TemporaryDirectory() as tmp:
 stage=Path(tmp)/"root"; (stage/"usr/bin").mkdir(parents=True); shutil.copy2(a.binary,stage/"usr/bin/ripple"); (stage/"usr/bin/ripple").chmod(0o755)
 def copy(src,dst):
  target=stage/dst; target.parent.mkdir(parents=True,exist_ok=True); shutil.copy2(root/src,target)
 copy("packaging/ripple.desktop","usr/share/applications/tel.ripple.native.desktop")
 copy("assets/icon.png","usr/share/icons/hicolor/256x256/apps/tel.ripple.native.png")
 for f in ["LICENSE","COPYING-GPL-3.0","NOTICE","THIRD-PARTY-NOTICES.txt"]: copy(f,"usr/share/doc/ripple-native/"+f)
 if osname!="linux":
  (stage/"usr/local").mkdir();
  for item in list((stage/"usr").iterdir()):
   if item.name!="local": shutil.move(str(item),stage/"usr/local"/item.name)
 if a.format=="tar":
  name=out/f"ripple-native-{version}-{osname}-{arch}.tar.gz"
  with tarfile.open(name,"w:gz") as tar: tar.add(stage/"usr",arcname="usr")
 elif a.format=="deb":
  if osname!="linux": raise SystemExit("DEB builds require Linux")
  pkgarch={"x86_64":"amd64","aarch64":"arm64"}[arch]; debversion=version.replace("-","~",1)
  (stage/"DEBIAN").mkdir(); (stage/"DEBIAN/control").write_text(f"Package: ripple-native\nVersion: {debversion}\nArchitecture: {pkgarch}\nMaintainer: Ripple Native contributors <342574562+vgs499p@users.noreply.github.com>\nSection: net\nPriority: optional\nHomepage: https://github.com/vgs499p/ripple-desktop\nDepends: libc6 (>= {glibc_min}), libgcc-s1, libgtk-3-0, libxkbcommon0, libxkbcommon-x11-0, libfontconfig1\nRecommends: {recommendations}\nDescription: Native Ripple social client\n Direct HTTPS client with local themes and profile support.\n")
  name=out/f"ripple-native_{debversion}_{pkgarch}.deb"; subprocess.run(["dpkg-deb","--root-owner-group","-Zxz","--build",str(stage),str(name)],check=True)
 else:
  if osname!="linux": raise SystemExit("RPM builds require Linux")
  top=Path(tmp)/"rpm"; (top/"SPECS").mkdir(parents=True)
  spec=top/"SPECS/ripple.spec"; release=version.split("-",1)[1] if "-" in version else "1"; release="0."+release if "-" in version else release
  files="\n".join("/"+str(f.relative_to(stage)) for f in stage.rglob("*") if f.is_file())
  spec.write_text(f"Name: ripple-native\nVersion: {version.split('-')[0]}\nRelease: {release}\nSummary: Native Ripple social client\nLicense: GPL-3.0-or-later\nURL: https://github.com/vgs499p/ripple-desktop\nRequires: gtk3, libxkbcommon, libxkbcommon-x11, fontconfig\n%description\nNative direct HTTPS Ripple client with local themes.\n%install\nmkdir -p %{{buildroot}}\ncp -a {stage}/usr %{{buildroot}}/\n%files\n{files}\n")
  subprocess.run(["rpmbuild","-bb","--define",f"_topdir {top}","--define","_build_id_links none",str(spec)],check=True)
  names=list((top/"RPMS").rglob("*.rpm"));
  if len(names)!=1: raise SystemExit("Expected one RPM")
  name=out/names[0].name; shutil.copy2(names[0],name)
 print(name)
