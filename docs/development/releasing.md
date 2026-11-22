# Python releases

Set the same version in `pyproject.toml` and the workspace package in `Cargo.toml`. Run `cargo check --workspace` to refresh `Cargo.lock`, then commit the manifests and lockfile together.

Build the source archive and a wheel from that archive with CPython 3.11 or newer and the pinned Rust toolchain.

```sh
python -m pip install 'maturin>=1.16,<2' twine
maturin build --release --locked --compatibility pypi --sdist --out dist
python -m twine check --strict dist/*
```

CI checks fresh installs of the source archive and CPython stable-ABI wheels for Linux (manylinux 2.28, x86-64 and ARM64), macOS (Intel and Apple silicon), and Windows (x86-64). The source archive contains the Python package, Rust workspace and bundled process definitions needed to build the extension. Review its contents before publishing.

After CI passes on the release commit, create an annotated `v<version>` tag on that commit and publish a GitHub release for the tag. The publish workflow checks the tag against both manifests, reruns CI, and uploads that run’s checked artifacts through PyPI Trusted Publishing using the `pypi` environment.

After the workflow succeeds, install `microfluidics==<version>` in a fresh environment and run `python -I scripts/check_install.py` from the matching checkout.
