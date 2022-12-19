import logging
from pathlib import Path

import pytest

from confguard.environment import config, FINGERPRINT
# noinspection PyProtectedMember
from confguard.services import _create_relative_path, Sentinel, Files, Links

_log = logging.getLogger(__name__)


class TestSentinel:
    def test_create(self):
        s = Sentinel.create()
        assert isinstance(s, Sentinel)
        assert Path(f".{s.name}.confguard").exists()

    def test_create_sentinel_exists(self):
        _ = Sentinel.create()
        s = Sentinel.create()
        assert Path(f".{s.name}.confguard").exists()
        assert len(list(Path.cwd().glob(FINGERPRINT))) == 1

    def test_remove(self):
        s = Sentinel.create()
        s.remove()
        assert not Path(f".{s.name}.confguard").exists()


PYTHON_TARGETS = [".envrc", ".run", "xxx/xxx.txt"]
REL_TARGET_DIR = Path("test_proj-1234")
TARGET_DIR = config.confguard_path / REL_TARGET_DIR


class TestFiles:

    @pytest.mark.parametrize(
        "targets",
        (
            ["xxx/xxx.txt"],
            [".envrc", ".run"],
            [".envrc", ".run", "xxx/xxx.txt"],
        )
    )
    def test_move_files(self, targets):
        f = Files(rel_target_dir=REL_TARGET_DIR, source_dir=Path.cwd(), targets=targets)
        f.move_files()
        for t in targets:
            assert Path(config.confguard_path / REL_TARGET_DIR / t).exists()
            assert not Path(f.source_dir / t).exists()
            assert Path(config.confguard_path / REL_TARGET_DIR / t) in f.target_locations

    @pytest.mark.parametrize(
        "targets",
        (
            ["xxx/xxx.txt"],
            [".envrc", ".run"],
            [".envrc", ".run", "xxx/xxx.txt"],
        )
    )
    def test_return_files(self, targets):
        # given
        f = Files(rel_target_dir=REL_TARGET_DIR, source_dir=Path.cwd(), targets=targets)
        f.move_files()
        # when
        f.return_files()
        # then all files exist at their source destination again
        for t in targets:
            assert Path(f.source_dir / t).exists()
            assert not Path(config.confguard_path / REL_TARGET_DIR / t).exists()
        # entire target dir should be removed
        assert not Path(config.confguard_path / REL_TARGET_DIR).exists()


class TestLinks:
    @pytest.mark.parametrize(
        "source_locations, target_locations",
        (
            ([Path.cwd() / "xxx/xxx.txt"], [TARGET_DIR / 'xxx/xxx.txt']),
            ([Path.cwd() / ".envrc"], [TARGET_DIR / '.envrc']),
            ([Path.cwd() / ".envrc", Path.cwd() / ".run"], [TARGET_DIR / '.envrc', TARGET_DIR / '.run']),
        )
    )
    def test_create_links(self, clear_test_proj, source_locations, target_locations):
        lk = Links(source_locations=source_locations, target_locations=target_locations)
        lk.create_links()
        for lk in source_locations:
            assert Path(lk).is_symlink()

    @pytest.mark.parametrize(
        "source_locations, target_locations",
        (
            ([Path.cwd() / "xxx/xxx.txt"], [TARGET_DIR / 'xxx/xxx.txt']),
            ([Path.cwd() / ".envrc"], [TARGET_DIR / '.envrc']),
            ([Path.cwd() / ".envrc", Path.cwd() / ".run"], [TARGET_DIR / '.envrc', TARGET_DIR / '.run']),
        )
    )
    def test_create_rel_links(self, clear_test_proj, source_locations, target_locations):
        lk = Links(source_locations=source_locations, target_locations=target_locations)
        lk.create_links(is_relative=True)
        for lk in source_locations:
            assert Path(lk).is_symlink()


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
