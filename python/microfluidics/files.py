"""Reading and writing files for an agent, kept within one directory."""

from pathlib import Path

from microfluidics._engine import DesignError

KEPT = {
    ".waivers.toml": "a waiver is a person's to write: say what you would waive and why, and leave the file to them",
    ".tuned.toml": "tuned values are written by `close`, not by hand",
}
"""Files only a person, or only the tool, writes."""


def confined(root: str, path: str) -> Path:
    """`path` as a place within `root`, following links; a relative path is taken from `root`."""
    base = Path(root).resolve()
    wanted = (base / path).resolve()
    if wanted != base and base not in wanted.parents:
        raise DesignError(f"{path} is outside {base}, where this server works", None, None)
    return wanted


def read(root: str, path: str) -> str:
    try:
        return confined(root, path).read_text()
    except OSError as error:
        raise DesignError(f"{path}: {error.strerror}", None, None) from error


def write(root: str, path: str, content: str) -> None:
    for ending, why in KEPT.items():
        if path.endswith(ending):
            raise DesignError(f"{path} is not written here: {why}", None, None)
    file = confined(root, path)
    file.parent.mkdir(parents=True, exist_ok=True)
    file.write_text(content)


def listing(root: str, path: str) -> list[str]:
    """The files and directories in `path`, directories with a `/` after their name."""
    directory = confined(root, path)
    try:
        return sorted(f"{entry.name}/" if entry.is_dir() else entry.name for entry in directory.iterdir())
    except OSError as error:
        raise DesignError(f"{path}: {error.strerror}", None, None) from error
