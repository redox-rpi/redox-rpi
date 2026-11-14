#!/usr/bin/env python3
"""ABI check of Redox' prebuilt aarch64 packages against a redox-rpi image.

For every ELF program and library in each extracted package: the libraries
it needs (DT_NEEDED) and its undefined dynamic symbols, compared with what
the image's /usr/lib and the package's dependencies (transitively) provide.
Usage: abi-check.py <work dir with x/<pkg>/, pkgs/<pkg>.toml, img/lib/>
"""
import os, re, subprocess, sys, tomllib
work = sys.argv[1]
RE = "aarch64-linux-gnu-readelf"

def is_elf(p):
    try:
        with open(p, "rb") as f:
            return f.read(4) == b"\x7fELF"
    except OSError:
        return False

def elf_info(p):
    dyn = subprocess.run([RE, "-dW", p], capture_output=True, text=True).stdout
    needed = re.findall(r"\(NEEDED\)\s+Shared library: \[(.+?)\]", dyn)
    soname = re.findall(r"\(SONAME\)\s+Library soname: \[(.+?)\]", dyn)
    hdr = subprocess.run([RE, "-lW", p], capture_output=True, text=True).stdout
    interp = re.search(r"Requesting program interpreter: (.+?)\]", hdr)
    syms = subprocess.run([RE, "--dyn-syms", "-W", p], capture_output=True, text=True).stdout
    undef, defined = set(), set()
    for line in syms.splitlines():
        f = line.split()
        if len(f) < 8 or not f[0].rstrip(":").isdigit():
            continue
        name = f[7].split("@")[0]
        if f[6] == "UND":
            if f[4] == "GLOBAL" and name:
                undef.add(name)
        elif f[4] in ("GLOBAL", "WEAK"):
            defined.add(name)
    return needed, soname, interp.group(1) if interp else None, undef, defined

def files(root):
    for d, _, fs in os.walk(root):
        for n in fs:
            p = os.path.join(d, n)
            if not os.path.islink(p) and is_elf(p):
                yield p

# Libraries of the image.
libs = {}  # soname or file name -> defined symbols
for p in files(os.path.join(work, "img/lib")):
    if "/drivers/" in p or "/boot/" in p:
        continue
    _, so, _, _, d = elf_info(p)
    for name in set(so + [os.path.basename(p)]):
        libs.setdefault(("image", name), set()).update(d)

pkgdir = os.path.join(work, "x")
meta = {}
for n in sorted(os.listdir(pkgdir)):
    try:
        with open(os.path.join(work, "pkgs", n + ".toml"), "rb") as f:
            meta[n] = tomllib.load(f)
    except FileNotFoundError:
        meta[n] = {}
pkglibs, pkgelfs = {}, {}
for n in meta:
    pkglibs[n], pkgelfs[n] = {}, []
    for p in files(os.path.join(pkgdir, n)):
        info = elf_info(p)
        rel = os.path.relpath(p, os.path.join(pkgdir, n))
        pkgelfs[n].append((rel, info))
        if info[1] or ".so" in os.path.basename(p):
            for name in set(info[1] + [os.path.basename(p)]):
                pkglibs[n].setdefault(name, set()).update(info[4])

def deps(n, seen=None):
    seen = seen if seen is not None else set()
    for d in meta.get(n, {}).get("depends", []):
        if d not in seen:
            seen.add(d)
            deps(d, seen)
    return seen

image_names = {k[1] for k in libs}
image_syms = set().union(*libs.values())
for n in meta:
    progs = [(r, i) for r, i in pkgelfs[n] if i[2]]  # with an interpreter
    statics = [r for r, i in pkgelfs[n] if not i[2] and not i[1] and ".so" not in r and ".o" not in r and ".a" not in r]
    avail_names, avail_syms = set(image_names), set(image_syms)
    for d in deps(n) | {n}:
        for name, s in pkglibs.get(d, {}).items():
            avail_names.add(name); avail_syms |= s
    missing_libs, missing_syms, interps = set(), {}, set()
    for rel, (needed, so, interp, undef, _) in [(r, i) for r, i in pkgelfs[n] if i[2] or i[1]]:
        if interp: interps.add(interp)
        missing_libs |= {x for x in needed if x not in avail_names}
        m = undef - avail_syms
        if m: missing_syms[rel] = sorted(m)
    from_deps = sorted(d for d in deps(n) if d in meta)
    status = "ok" if not missing_libs and not missing_syms else "BROKEN"
    if not progs and not statics and not pkglibs[n]: status = "no ELF"
    print(f"## {n}: {status}; {len(progs)} dynamic, {len(statics)} static programs; interp {sorted(interps)}; deps {meta[n].get('depends', [])}")
    if missing_libs: print("   missing libraries:", sorted(missing_libs))
    for rel, m in list(missing_syms.items())[:4]:
        print(f"   {rel}: {len(m)} missing symbols: {' '.join(m[:12])}")
