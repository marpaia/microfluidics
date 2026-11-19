"""Check, or with --fix rewrite, the import layout of the workspace's Rust sources.

Imports are grouped in this order, with one blank line between groups:

    use std::...;             the standard library (std, core, alloc, proc_macro)

    use serde::...;           third-party crates

    use microfluidics_...;    other crates of this workspace

    use crate::...;           this crate

Every path into the current crate starts at `crate::`, never `super::`, `self::` or a bare child
module, so a file can move without its imports changing. rustfmt sorts within each group; its own
`group_imports` option is unstable and has no group for workspace crates.
"""

import argparse
import re
import sys
from pathlib import Path

STD = {"std", "core", "alloc", "proc_macro"}
WORKSPACE_PREFIX = "microfluidics_"
# The crate people depend on, which carries the workspace's name with nothing after it.
WORKSPACE_ROOT = "microfluidics"
USE = re.compile(r"^(\s*)((?:pub(?:\([a-z]+\))? )?use )(::)?([A-Za-z_][A-Za-z0-9_]*)")
INLINE_MODULE = re.compile(r"^(\s*)(?:pub(?:\([a-z]+\))? )?mod ([a-z_0-9]+) \{\s*$")
DEPENDENCY = re.compile(r"^([A-Za-z0-9_-]+)\s*=")


def external_crates(root: Path) -> set[str]:
    """Every dependency named in the workspace's manifests, as Rust spells it."""
    names: set[str] = set()
    for manifest in [root / "Cargo.toml", *root.glob("crates/*/Cargo.toml")]:
        in_dependencies = False
        for line in manifest.read_text().splitlines():
            if line.startswith("["):
                in_dependencies = "dependencies" in line
            elif in_dependencies and (match := DEPENDENCY.match(line)):
                names.add(match.group(1).replace("-", "_"))
    return names


def file_module(path: Path) -> list[str]:
    """The module a source file defines, as path segments below `crate`."""
    parts = list(path.parts)
    if "src" not in parts:
        return []
    below = parts[parts.index("src") + 1 :]
    below[-1] = below[-1].removesuffix(".rs")
    if below[-1] in ("lib", "main", "mod"):
        below.pop()
    return below


def group_of(root_segment: str) -> int:
    if root_segment in STD:
        return 0
    if root_segment.startswith(WORKSPACE_PREFIX) or root_segment == WORKSPACE_ROOT:
        return 2
    return 3 if root_segment == "crate" else 1


def statement_end(lines: list[str], start: int) -> int:
    """The index of the line that ends the `use` statement starting at `start`."""
    depth = 0
    for index in range(start, len(lines)):
        depth += lines[index].count("{") - lines[index].count("}")
        if depth <= 0 and lines[index].rstrip().endswith(";"):
            return index
    return len(lines) - 1


def rooted(line: str, module: list[str], externals: set[str]) -> str:
    """The first line of a `use` statement with its path starting at a crate."""
    match = USE.match(line)
    if not match or match.group(3):
        return line
    indent, keyword, _, first = match.groups()
    rest = line[match.end() :]
    workspace = first.startswith(WORKSPACE_PREFIX) or first == WORKSPACE_ROOT
    if first in STD or first == "crate" or first in externals or workspace:
        return line
    if first == "super":
        target = module[:-1]
    elif first == "self":
        target = module
    else:
        target = [*module, first]
    return f"{indent}{keyword}{'::'.join(['crate', *target])}{rest}"


def rewrite(path: Path, externals: set[str]) -> str:
    lines = path.read_text().splitlines()
    base = file_module(path)
    inline: list[tuple[int, str]] = []
    out: list[str] = []
    index = 0
    while index < len(lines):
        line = lines[index]
        indent = len(line) - len(line.lstrip())
        while inline and line.strip() == "}" and indent == inline[-1][0]:
            inline.pop()
        if opened := INLINE_MODULE.match(line):
            inline.append((len(opened.group(1)), opened.group(2)))
        module = [*base, *(name for _, name in inline)]
        match = USE.match(line)
        if not match:
            out.append(line)
            index += 1
            continue
        if match.group(2) != "use " or (out and out[-1].lstrip().startswith("#[")):
            # A re-export or an attributed import stays where it is; only its path is rooted.
            end = statement_end(lines, index)
            out.append(rooted(line, module, externals))
            out.extend(lines[index + 1 : end + 1])
            index = end + 1
            continue
        # A run of plain imports, broken only by blank lines.
        groups: dict[int, list[str]] = {}
        while index < len(lines):
            current = USE.match(lines[index])
            if current and current.group(2) == "use " and len(current.group(1)) == indent:
                end = statement_end(lines, index)
                first = rooted(lines[index], module, externals)
                root_segment = USE.match(first).group(4)  # type: ignore[union-attr]
                groups.setdefault(group_of(root_segment), []).extend([first, *lines[index + 1 : end + 1]])
                index = end + 1
            elif not lines[index].strip() and index + 1 < len(lines) and USE.match(lines[index + 1]):
                following = USE.match(lines[index + 1])
                if following and following.group(2) == "use " and len(following.group(1)) == indent:
                    index += 1
                else:
                    break
            else:
                break
        for position, key in enumerate(sorted(groups)):
            if position:
                out.append("")
            out.extend(groups[key])
    return "\n".join(out) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--fix", action="store_true", help="rewrite files instead of reporting them")
    arguments = parser.parse_args()

    root = Path(__file__).resolve().parent.parent
    externals = external_crates(root)
    failed = False
    for path in sorted(root.glob("crates/**/*.rs")):
        original = path.read_text()
        wanted = rewrite(path, externals)
        relative = path.relative_to(root)
        if wanted != original:
            if arguments.fix:
                path.write_text(wanted)
                print(f"rewrote {relative}")
            else:
                print(f"{relative}: imports are not grouped as std, third-party, microfluidics_*, crate")
                failed = True
        for number, line in enumerate(wanted.splitlines(), start=1):
            if re.search(r"\bsuper::", line):
                print(f"{relative}:{number}: path uses super::; start it at crate::")
                failed = True
    if failed and not arguments.fix:
        print("run `python3 scripts/check_imports.py --fix`, then `cargo fmt --all`")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
