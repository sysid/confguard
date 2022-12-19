import logging
import os
from pathlib import Path

import pytest

from confguard.environment import config, FINGERPRINT, ROOT_DIR
from confguard.main import (
    create_sentinel,
    move_files,
    create_links,
    _create_relative_path, _guard,
)

_log = logging.getLogger(__name__)


def test_create_sentinel():
    name = create_sentinel()
    assert Path(f".{name}.confguard").exists()


def test_create_sentinel_exists():
    _ = create_sentinel()
    name = create_sentinel()
    assert Path(f".{name}.confguard").exists()
    assert len(list(Path.cwd().glob(FINGERPRINT))) == 1


def test_move_files():
    targets = [".envrc", ".run"]
    name = "test_proj-1234"
    target_locations = move_files(name, targets)
    for t in targets:
        assert Path(config.confguard_path / name / t).exists()
        assert str(Path(config.confguard_path / name / t)) in target_locations


def test_create_links():
    targets = [".envrc", ".run"]
    name = "test_proj-1234"
    target_locations = move_files(name, targets)
    links = create_links(target_locations)
    for t in targets:
        assert Path(t).is_symlink()
        fixed_tmp_path = str(Path(t).resolve()).replace("/private", "")  # macos fix
        assert Path(fixed_tmp_path).resolve() == Path(config.confguard_path / name / t)
    _ = None


@pytest.mark.parametrize(
    ("source", "target", "expected"),
    (
        ("/c/b/a/xxx/.envrc", "/c/y/xxx-123/.envrc", "../../../y/xxx-123/.envrc"),
        (
            "/c/b/a/xxx/.envrc",
            "/tmp/y/xxx-123/.envrc",
            "../../../../tmp/y/xxx-123/.envrc",
        ),
        ("/c/xxx/.envrc", "/c/y/xxx-123/.envrc", "../y/xxx-123/.envrc"),
    ),
)
def test_find_relative_path(source, target, expected):
    _log.info(f"{source=} {target=} {expected=}")

    if not (Path(source).is_absolute() and Path(target).is_absolute()):
        raise ValueError("Both source and target must be absolute paths")

    new_sp = []
    new_tp = []
    name = Path(source).name
    source_parts = Path(source).parent.parts
    target_parts = Path(target).parent.parts
    for i, sp in enumerate(source_parts):
        if sp == target_parts[i]:  # remove common leading parts
            continue
        else:
            removed_p = source_parts[:i]
            new_sp = source_parts[i:]
            new_tp = target_parts[i:]
            break

    rel_path = [".."] * len(new_sp) + list(new_tp) + [name]
    rel_path = Path(*rel_path)
    assert rel_path == Path(expected)
    _ = None


@pytest.mark.parametrize(
    ("source", "target", "expected"),
    (
        ("/c/b/a/xxx/.envrc", "/c/y/xxx-123/.envrc", "../../../y/xxx-123/.envrc"),
        (
            "/c/b/a/xxx/.envrc",
            "/tmp/y/xxx-123/.envrc",
            "../../../../tmp/y/xxx-123/.envrc",
        ),
        ("/c/xxx/.envrc", "/c/y/xxx-123/.envrc", "../y/xxx-123/.envrc"),
    ),
)
def test_find_relative_path_builtin(source, target, expected):
    rel_path = _create_relative_path(source, target)
    assert Path(rel_path) == Path(expected)


def test__guard():
    test_proj = ROOT_DIR / "tests/resources/test_proj"
    _guard("PYTHON")

    sentinel = list(Path(test_proj).glob("**/.test_proj-*"))
    assert len(sentinel) == 1

    confguard = list(Path(config.confguard_path).glob("**/test_proj-*"))
    assert len(confguard) == 1
    confguard = confguard[0]
    assert confguard.is_dir()
    assert confguard / ".envrc" in confguard.iterdir()
    assert confguard / ".run" in confguard.iterdir()
