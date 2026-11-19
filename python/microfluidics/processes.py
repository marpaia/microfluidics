"""Fabrication processes: the rule deck, tolerances and mask preparation a design is held to."""

from microfluidics._engine import Process

__all__ = ["CONSERVATIVE_SU8_PDMS", "Process"]

CONSERVATIVE_SU8_PDMS = Process.bundled("conservative-su8-pdms")
"""Cautious rule-of-thumb limits for a two-layer SU-8 master and PDMS cast: a stand-in for a
cleanroom's own."""
