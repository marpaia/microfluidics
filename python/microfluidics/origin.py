"""Where in a design's source something was asked for."""

import os
import sys

Origin = tuple[str, int]

_PACKAGE = os.path.dirname(os.path.abspath(__file__)) + os.sep
_REGIONS = os.path.join(_PACKAGE, "regions") + os.sep


def authored(file: str) -> bool:
    """Whether `file` holds a design or a region, the standard regions among them, and not the
    machinery of this package that builds them."""
    path = os.path.abspath(file)
    return path.startswith(_REGIONS) or not path.startswith(_PACKAGE)


def reported(file: str) -> str:
    """The name a report gives a source file: a standard region's by its place in this package,
    which is the same wherever the package is installed; any other by its path from the working
    directory."""
    path = os.path.abspath(file)
    if path.startswith(_PACKAGE):
        return os.path.relpath(path, os.path.dirname(os.path.dirname(_PACKAGE))).replace(os.sep, "/")
    return os.path.relpath(path)


def caller() -> Origin:
    """The file and line of the nearest call from a design or a region: the line that asked for
    what is being made."""
    frame = sys._getframe(1)
    while frame.f_back is not None and not authored(frame.f_code.co_filename):
        frame = frame.f_back
    return (frame.f_code.co_filename, frame.f_lineno)
