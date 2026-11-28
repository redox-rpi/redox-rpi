#!/usr/bin/env python3
"""Keep the cookbook's packages in step with their recipes (#772).

Run by scripts/build-image.sh in the upstream checkout (the folder with
recipes/ and repo/), not by hand.

    package-origins.py check --config FS [--touched REL]...
    package-origins.py stamp --config FS
    package-origins.py manifest [--applied EXP:REL]... NAME...

The cookbook decides whether to cook a recipe again from its source
(commit, source_info.toml), not from the recipe that built it. So when an
experiment's recipe.toml goes away, or its patches, or a config changes a
package from "source" to binary, the recipe's target/ keeps the package of
the old recipe and the cookbook publishes it again: the image ships a
package from a recipe that is no longer applied (Test 685: orbutils of #756
in a build with #756 reverted).

A recipe's origin is the rule this build wants for it (its value in the
filesystem config and the config's includes) plus the hashes of its
recipe.toml and of the patches it lists. `stamp`, after cooking, writes it
into target/.redox-rpi-origin of every recipe with a target/: what the
package there was made from.

check, before cooking, compares each target/ with what this build wants:
- the stamp differs: the recipe, its patches or its rule changed;
- no stamp (a target/ from a build without this check, also of an older
  commit): unknown, so cleaned if an experiment has a recipe or patches for
  it, or if it was built here (target/.../build/) although this build does
  not ask for "source" or "local". Any other gets its stamp.
Cleaning removes target/, the package in repo/, the patch files the recipe
no longer lists and .redox-rpi-cook-patches, and puts a git source back to
its commit, nested repositories included; the cookbook then builds or
fetches the package again. A config that cannot be read stops the build:
the rules are then unknown.

manifest: prints a [package_origins] table for the given packages, each
"<how> <recipe>": how is binary (fetched from upstream's binary repository)
or source (built here); recipe is "upstream", "upstream with our patches",
"experiment <name>" or "tools/<name>".
"""
import argparse
import glob
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tomllib

TARGET = "aarch64-unknown-redox"
REPO = os.path.join("repo", TARGET)


def load_rules(path, seen=None):
    """The [packages] values of a filesystem config, its includes first."""
    seen = seen if seen is not None else set()
    path = os.path.abspath(path)
    if path in seen:
        return {}
    seen.add(path)
    with open(path, "rb") as f:
        data = tomllib.load(f)
    rules = {}
    for inc in data.get("include", []):
        rules.update(load_rules(os.path.join(os.path.dirname(path), inc), seen))
    rules.update(data.get("packages", {}))
    return rules


def sha(path):
    try:
        with open(path, "rb") as f:
            return hashlib.sha256(f.read()).hexdigest()
    except OSError:
        return "missing"


def recipes():
    """recipes/<path>/recipe.toml -> <path>, for every recipe."""
    for toml in glob.glob("recipes/**/recipe.toml", recursive=True):
        yield os.path.relpath(os.path.dirname(toml), "recipes")


def origin(rel, rules):
    recipe = os.path.join("recipes", rel)
    toml = os.path.join(recipe, "recipe.toml")
    try:
        with open(toml, "rb") as f:
            src = tomllib.load(f).get("source", {})
    except (OSError, tomllib.TOMLDecodeError):
        src = {}
    patches = [[p, sha(os.path.join(recipe, p))] for p in src.get("patches", [])]
    rule = rules.get(os.path.basename(rel), "default")
    return hashlib.sha256(json.dumps([rule, sha(toml), patches], sort_keys=True).encode()).hexdigest()


STAMP = ".redox-rpi-origin"


def listed_patches(recipe):
    try:
        with open(os.path.join(recipe, "recipe.toml"), "rb") as f:
            return set(tomllib.load(f).get("source", {}).get("patches", []))
    except (OSError, tomllib.TOMLDecodeError):
        return set()


def clean(rel):
    recipe = os.path.join("recipes", rel)
    name = os.path.basename(rel)
    shutil.rmtree(os.path.join(recipe, "target"), ignore_errors=True)
    source = os.path.join(recipe, "source")
    # A same_as recipe's source is a link to another recipe's (relibc-tests-bins
    # to relibc): leave it, or that recipe loses its patches.
    if not os.path.islink(source) and os.path.isdir(os.path.join(source, ".git")):
        subprocess.run(["git", "-C", source, "reset", "-q", "--hard"], check=False)
        # -ff: also nested repositories, which -f leaves.
        subprocess.run(["git", "-C", source, "clean", "-ffdxq"], check=False)
    keep = listed_patches(recipe)
    for patch in glob.glob(os.path.join(recipe, "*.patch")):
        if os.path.basename(patch) in keep:
            continue
        tracked = subprocess.run(["git", "ls-files", "--error-unmatch", "--", patch],
                                 capture_output=True).returncode == 0
        if not tracked:
            os.remove(patch)
    for leftover in [os.path.join(recipe, ".redox-rpi-cook-patches"),
                     os.path.join(REPO, f"{name}.pkgar"), os.path.join(REPO, f"{name}.toml")]:
        try:
            os.remove(leftover)
        except OSError:
            pass


def rules_or_exit(config):
    try:
        return load_rules(config)
    except (OSError, tomllib.TOMLDecodeError) as e:
        sys.exit(f"package-origins: cannot read {config}: {e}")


def read(path):
    try:
        with open(path) as f:
            return f.read().strip()
    except OSError:
        return None


def check(args):
    rules = rules_or_exit(args.config)
    touched = set(args.touched)  # an experiment has a recipe or patches for it
    for rel in recipes():
        target = os.path.join("recipes", rel, "target")
        if not os.path.isdir(target):
            continue
        ident = origin(rel, rules)
        stamp = read(os.path.join(target, STAMP))
        if stamp == ident:
            continue
        if stamp is not None:
            why = "its recipe, patches or rule changed since its package was made"
        elif rel in touched:
            why = "no record of what made its package, and an experiment changes it"
        elif os.path.isdir(os.path.join(target, TARGET, "build")) and \
                rules.get(os.path.basename(rel)) not in ("source", "local"):
            why = "built here, but this build does not ask for source"
        else:
            with open(os.path.join(target, STAMP), "w") as f:
                f.write(ident + "\n")
            continue
        clean(rel)
        print(f"recipes/{rel}: {why}: package removed, it is built or fetched again")


def stamp(args):
    rules = rules_or_exit(args.config)
    for rel in recipes():
        target = os.path.join("recipes", rel, "target")
        if os.path.isdir(target):
            with open(os.path.join(target, STAMP), "w") as f:
                f.write(origin(rel, rules) + "\n")


def ours(recipe):
    """Our patches: core recipes (.redox-rpi-patches) or experiment patches (exp-*.patch)."""
    stamp = os.path.join(recipe, ".redox-rpi-patches")
    if os.path.isfile(stamp) and os.path.getsize(stamp) > 0:
        return True
    return bool(glob.glob(os.path.join(recipe, "exp-*.patch")))


def manifest(args):
    by_name = {}
    for rel in sorted(recipes(), key=lambda r: r.startswith("wip/")):
        by_name.setdefault(os.path.basename(rel), rel)  # a wip/ copy never hides the real one
    exp_of = {}
    for item in args.applied:
        exp, rel = item.split(":", 1)
        exp_of[rel] = exp
    print("# How each package was made and where its recipe came from (#772).")
    print("[package_origins]")
    for name in args.names:
        rel = by_name.get(name)
        if rel is None:
            print(f'{name} = "? (no recipe)"')
            continue
        recipe = os.path.join("recipes", rel)
        target = os.path.join(recipe, "target", TARGET)
        how = "binary" if os.path.isfile(os.path.join(target, "source.pkgar")) \
            and not os.path.isdir(os.path.join(target, "build")) else "source"
        try:
            with open(os.path.join(recipe, "recipe.toml"), "rb") as f:
                path_src = tomllib.load(f).get("source", {}).get("path", "")
        except (OSError, tomllib.TOMLDecodeError):
            path_src = ""
        if rel in exp_of:
            where = f"experiment {exp_of[rel]}"
        elif path_src and os.path.basename(os.path.dirname(path_src.rstrip("/"))) == "tools":
            where = f"tools/{name}"
        elif ours(recipe):
            where = "upstream with our patches"
        else:
            where = "upstream"
        print(f'{name} = "{how} {where}"')


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("check")
    c.add_argument("--config", required=True)
    c.add_argument("--touched", action="append", default=[], metavar="REL")
    st = sub.add_parser("stamp")
    st.add_argument("--config", required=True)
    m = sub.add_parser("manifest")
    m.add_argument("--applied", action="append", default=[], metavar="EXP:REL")
    m.add_argument("names", nargs="*")
    args = ap.parse_args()
    {"check": check, "stamp": stamp, "manifest": manifest}[args.cmd](args)


if __name__ == "__main__":
    main()
