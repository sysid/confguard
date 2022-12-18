import shutil
from pathlib import Path

from confguard.main import create_sentinel


def test_create_sentinel():
    existings = list(Path.cwd().glob(".confguard-*"))
    for e in existings:
        e.unlink()
    name = create_sentinel("default")
    assert Path(f".{name}.confguard").exists()


def test_create_sentinel_exists():
    existings = list(Path.cwd().glob(".confguard-*"))
    for e in existings:
        e.unlink()
    _ = create_sentinel("default")
    name = create_sentinel("default")
    assert Path(f".{name}.confguard").exists()
