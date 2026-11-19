"""The designs give the geometry captured for them, and everything in them knows its line."""

import json

from conftest import difference, golden

import microfluidics as mf


def test_each_design_builds_what_was_captured(named: tuple[str, type[mf.Design]]) -> None:
    name, design = named
    captured = golden(f"designs/{name}.json")
    built = json.loads(design.elaborate().fingerprint())
    assert difference(built, captured["untuned"]["fingerprint"], name) is None
    if captured["tuned"] is not None:
        tuned = mf.Overrides.parse("tuned", captured["tuned_values"])
        built = json.loads(design.elaborate(tuned).fingerprint())
        assert difference(built, captured["tuned"]["fingerprint"], f"{name} tuned") is None


def test_everything_a_design_makes_knows_the_line_that_made_it(named: tuple[str, type[mf.Design]]) -> None:
    name, design = named
    built = design.elaborate()
    origins = built.origins()
    source = (design._source, design._line)
    lines = open(design._source).read().splitlines()
    for path, (file, line) in origins.items():
        if "." in path:
            continue
        # What the design itself makes is found in its own file, on a line of its `build`.
        assert file == source[0], path
        assert line == source[1] or "self." in " ".join(lines[max(0, line - 4) : line]), (path, line)
    # What a standard region draws is found in the region's own file.
    assert any(file.endswith("microfluidics/regions/access_pad.py") for file, _ in origins.values())
