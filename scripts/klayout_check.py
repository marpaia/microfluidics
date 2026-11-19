"""Measure mask files with KLayout, for comparison with this project's own checks.

Reads a GDSII file and, for each layer given as NUMBER=DRAWING.dxf, reports as JSON what KLayout
makes of them: how many outlines the layer has when flattened, their area, how many places are
narrower than --width or closer than --space, and the area by which the DXF drawing differs
from the GDSII layer.

    python scripts/klayout_check.py chip.gds 1=mask_1_trench.dxf 2=mask_2_flow.dxf --width 1.0 --space 2.0

Needs the `klayout` Python module.
"""

import argparse
import json

import klayout.db as db


def flat_region(layout: db.Layout, index: int) -> db.Region:
    """Every shape of one layer under the top cell, references expanded."""
    return db.Region(layout.top_cell().begin_shapes_rec(index))


def drawing_region(path: str) -> db.Region:
    """Every shape of a DXF drawing in micrometres, on a nanometre grid."""
    options = db.LoadLayoutOptions()
    options.dxf_dbu = 0.001
    options.dxf_unit = 1.0
    layout = db.Layout()
    layout.read(path, options)
    region = db.Region()
    for info in layout.layer_infos():
        region += flat_region(layout, layout.layer(info))
    return region


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("gds")
    parser.add_argument("drawings", nargs="*", help="NUMBER=DRAWING.dxf for each layer")
    parser.add_argument("--width", type=float, default=0.0, help="micrometres")
    parser.add_argument("--space", type=float, default=0.0, help="micrometres")
    arguments = parser.parse_args()

    layout = db.Layout()
    layout.read(arguments.gds)
    to_dbu = 1.0 / layout.dbu
    drawings = dict(item.split("=", 1) for item in arguments.drawings)
    layers = {}
    for info in layout.layer_infos():
        region = flat_region(layout, layout.layer(info))
        measured = {
            "outlines": region.count(),
            "area_um2": region.area() * layout.dbu**2,
            "narrow": region.width_check(round(arguments.width * to_dbu)).count() if arguments.width > 0 else 0,
            "close": region.space_check(round(arguments.space * to_dbu)).count() if arguments.space > 0 else 0,
        }
        drawing = drawings.get(str(info.layer))
        if drawing is not None:
            measured["drawing_differs_um2"] = (region ^ drawing_region(drawing)).area() * layout.dbu**2
        layers[str(info.layer)] = measured
    result = {
        "database_unit_um": layout.dbu,
        "cells": sorted(cell.name for cell in layout.each_cell()),
        "layers": layers,
    }
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
