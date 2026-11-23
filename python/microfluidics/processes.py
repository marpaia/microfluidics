"""Fabrication processes: the rule deck, tolerances and mask preparation a design is held to."""

from microfluidics._engine import Process

__all__ = ["COSINC_SU8_PDMS", "FLOWJEM_SU8_PDMS", "Process"]

FLOWJEM_SU8_PDMS = Process.bundled("flowjem-su8-pdms")
"""FlowJEM, by mail order: an SU-8 master and PDMS cast, with features and gaps of 5 um and more."""

COSINC_SU8_PDMS = Process.bundled("cosinc-su8-pdms")
"""The COSINC cleanroom at the University of Colorado Boulder, by hand: a two-layer SU-8 master and
PDMS cast. COSINC has published none of its limits, so every one is an assumption."""
