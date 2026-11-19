"""The standard regions: what a design places that it does not draw itself."""

from microfluidics.regions.access_pad import AccessPad
from microfluidics.regions.alignment_mark import AlignmentMark
from microfluidics.regions.junction import Junction
from microfluidics.regions.line_monitor import LineMonitor
from microfluidics.regions.manifold import Manifold
from microfluidics.regions.process_monitor import ProcessMonitor
from microfluidics.regions.serpentine import Serpentine
from microfluidics.regions.straight_channel import StraightChannel
from microfluidics.regions.trap_chamber import TrapChamber
from microfluidics.regions.trench_lane import TrenchLane
from microfluidics.regions.vernier import Vernier

__all__ = [
    "AccessPad",
    "AlignmentMark",
    "Junction",
    "LineMonitor",
    "Manifold",
    "ProcessMonitor",
    "Serpentine",
    "StraightChannel",
    "TrapChamber",
    "TrenchLane",
    "Vernier",
]
