import logging
import shutil
from pathlib import Path

import pytest

from confguard.environment import config, FINGERPRINT, ROOT_DIR
from confguard.exceptions import FileDoesNotExistError, BackupExistError
# noinspection PyProtectedMember
from confguard.services import _create_relative_path, Sentinel, Files, Links

_log = logging.getLogger(__name__)


class TestSentinel:
    def test_create(self):
        Sentinel.create()
        assert config.sentinel is not None

    def test_create_sentinel_exists(self):
        Sentinel.create()
        first = config.sentinel
        Sentinel.create()
        assert config.sentinel == first

    def test_remove(self):
        Sentinel.create()
        Sentinel.remove()
        assert config.sentinel is None


PYTHON_TARGETS = [".envrc", ".run", "xxx/xxx.txt"]
REL_TARGET_DIR = Path("test_proj-1234")
TARGET_DIR = config.confguard_path / REL_TARGET_DIR


class TestFiles:

    @pytest.mark.parametrize(
        "targets",
        (
            ["xxx/xxx.txt"],
            [".run"],
            [".envrc", ".run"],
            [".envrc", ".run", "xxx/xxx.txt"],
        )
    )
    def test_create_bkp(self, targets):
        f = Files(rel_target_dir=REL_TARGET_DIR, source_dir=Path.cwd(), targets=targets)
        f.create_bkp()

        assert f.bkp_dir.exists()
        assert len(list(f.bkp_dir.glob("*"))) == len(targets)
        for target in targets:
            assert (f.bkp_dir / target).exists()

    @pytest.mark.parametrize("targets", ([".envrc"],))
    def test_create_bkp_but_bkp_dir_exists(self, targets):
        f = Files(rel_target_dir=REL_TARGET_DIR, source_dir=Path.cwd(), targets=targets)
        f.create_bkp()

        with pytest.raises(BackupExistError) as e:
            f.create_bkp()

    @pytest.mark.parametrize(
        "targets",
        (
            ["xxx/xxx.txt"],
            [".envrc", ".run", "xxx/xxx.txt"],
        )
    )
    def test_restore_bkp(self, targets):
        # given: backup created
        f = Files(rel_target_dir=REL_TARGET_DIR, source_dir=Path.cwd(), targets=targets)
        f.create_bkp()

        # when: all files are moved/deleted
        test_proj = ROOT_DIR / "tests/resources/test_proj"
        shutil.rmtree(test_proj / ".run", ignore_errors=True)  # will be linked
        Path(test_proj / ".run").unlink(missing_ok=True)  # will be linked
        Path(test_proj / ".envrc").unlink(missing_ok=True)  # will be linked
        Path(test_proj / "xxx/xxx.txt").unlink(missing_ok=True)  # will be linked

        f.restore_bkp()

        # then: files are restored
        Path(test_proj / ".run").exists()
        Path(test_proj / ".run").is_dir()
        Path(test_proj / ".envrc").exists()
        Path(test_proj / ".envrc").is_file()
        Path(test_proj / "xxx/xxx.txt").exists()
        Path(test_proj / "xxx/xxx.txt").is_file()

    @pytest.mark.parametrize(
        "targets",
        (
            ["xxx/xxx.txt"],
            [".run"],
            [".envrc", ".run"],
            [".envrc", ".run", "xxx/xxx.txt"],
        )
    )
    def test_delete_bkp(self, targets):
        f = Files(rel_target_dir=REL_TARGET_DIR, source_dir=Path.cwd(), targets=targets)
        f.create_bkp()
        f.delete_bkp_dir()
        assert not f.bkp_dir.exists()

    def test_delete_nonexisting_bkp(self):
        f = Files(rel_target_dir=REL_TARGET_DIR, source_dir=Path.cwd(), targets=[])
        f.delete_bkp_dir()
        assert not f.bkp_dir.exists()

    @pytest.mark.parametrize(
        "targets",
        (
            ["xxx/xxx.txt"],
            [".run"],
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
        lk.create()
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
        lk.create(is_relative=True)
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
    def test_remove_links(self, clear_test_proj, source_locations, target_locations):
        lk = Links(source_locations=source_locations, target_locations=target_locations)
        lk.create()

        lk.remove()
        for lk in source_locations:
            assert not Path(lk).exists()

    @pytest.mark.parametrize(
        "source_locations, target_locations",
        (
            ([Path.cwd() / ".envrc"], [TARGET_DIR / '.envrc']),
        )
    )
    def test_remove_nonexisting_links(self, clear_test_proj, source_locations, target_locations):
        lk = Links(source_locations=source_locations, target_locations=target_locations)
        lk.create()
        Path(source_locations[0]).unlink()
        lk.remove()
        for lk in source_locations:
            assert not Path(lk).exists()

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
