"""Smoke-test a distribution installed in the current Python environment."""

import json
import os
import subprocess
import sys
import sysconfig
import tempfile
import tomllib
from importlib.metadata import distribution
from importlib.resources import files
from pathlib import Path


def main() -> None:
    manifest = Path(__file__).resolve().parents[1] / "pyproject.toml"
    expected = tomllib.loads(manifest.read_text(encoding="utf-8"))["project"]["version"]
    installed = distribution("microfluidics")
    assert installed.version == expected, (installed.version, expected)

    package = files("microfluidics")
    for resource in ("py.typed", "_engine.pyi", "guide.md"):
        assert package.joinpath(resource).is_file(), resource

    executable = "microfluidics.exe" if sys.platform == "win32" else "microfluidics"
    cli = Path(sysconfig.get_path("scripts")) / executable
    module = [sys.executable, "-I", "-X", "utf8", "-m", "microfluidics"]
    with tempfile.TemporaryDirectory() as workdir:
        # Run the example a reader copies from the package description on PyPI.
        description = installed.metadata["Description"]
        example = description.split("```python\n", 1)[1].split("\n```", 1)[0]
        Path(workdir, "test_channel.py").write_text(example, encoding="utf-8")
        solved = subprocess.check_output([*module, "solve", "test_channel.py", "--json"], cwd=workdir)
        checks = json.loads(solved)["checks"]
        assert any(check["id"] == "requirement/slow" and check["status"] == "satisfied" for check in checks)
        subprocess.run([*module, "export", "test_channel.py", "out"], cwd=workdir, check=True)
        for artifact in ("REPORT.md", "masks/preview.svg"):
            assert Path(workdir, "out", artifact).is_file(), artifact

        guide = subprocess.check_output(
            [cli, "guide"],
            cwd=workdir,
            encoding="utf-8",
            env={**os.environ, "PYTHONIOENCODING": "utf-8"},
        )
        assert guide.strip()
        result = subprocess.check_output(
            [*module, "process", "conservative-su8-pdms"],
            cwd=workdir,
            encoding="utf-8",
        )
        assert 'kind = "edge_margin"' in result
        reply = subprocess.check_output(
            [*module, "mcp"],
            cwd=workdir,
            input='{"jsonrpc": "2.0", "id": 1, "method": "initialize"}\n',
            encoding="utf-8",
        )
        assert json.loads(reply)["result"]["serverInfo"]["version"] == expected


if __name__ == "__main__":
    main()
