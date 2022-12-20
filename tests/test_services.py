import logging
import shutil
from pathlib import Path

import pytest

from confguard.environment import CONFGUARD_BKP_DIR, ROOT_DIR, config
from confguard.exceptions import BackupExistError

# noinspection PyProtectedMember
from confguard.services import Files, Links, Sentinel
from tests.conftest import SENTINEL, TARGET_DIR

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


class TestFiles:
    @pytest.mark.parametrize(
        "targets",
        (
            ["xxx/xxx.txt"],
            [".run"],
            [".envrc", ".run"],
            [".envrc", ".run", "xxx/xxx.txt"],
        ),
    )
    def test_create_bkp(self, targets):
        test_proj = ROOT_DIR / "tests/resources/test_proj"
        bkp_dir = Path.cwd() / CONFGUARD_BKP_DIR
        f = Files(rel_target_dir=SENTINEL, source_dir=Path.cwd(), targets=targets)
        f.create_bkp(source_dir=Path.cwd(), bkp_dir=bkp_dir)

        assert bkp_dir.exists()
        assert len(list(bkp_dir.glob("*"))) == len(targets)
        for target in targets:
            assert (bkp_dir / target).exists()

    @pytest.mark.parametrize("targets", ([".envrc"],))
    def test_create_bkp_but_bkp_dir_exists(self, targets):
        bkp_dir = Path.cwd() / CONFGUARD_BKP_DIR
        f = Files(rel_target_dir=SENTINEL, source_dir=Path.cwd(), targets=targets)
        f.create_bkp(source_dir=Path.cwd(), bkp_dir=bkp_dir)

        with pytest.raises(BackupExistError) as e:
            f.create_bkp(source_dir=Path.cwd(), bkp_dir=bkp_dir)

    @pytest.mark.parametrize("targets", ([".envrc", ".run", "xxx/xxx.txt"],))
    def test_create_bkp_of_target_dir(self, targets):
        bkp_dir = TARGET_DIR / CONFGUARD_BKP_DIR
        f = Files(rel_target_dir=SENTINEL, source_dir=Path.cwd(), targets=targets)
        f.move_files(source_dir=Path.cwd(), target_dir=TARGET_DIR)

        f.create_bkp(source_dir=TARGET_DIR, bkp_dir=bkp_dir)

        assert bkp_dir.exists()
        assert len(list(bkp_dir.glob("*"))) == len(targets)
        for target in targets:
            assert (bkp_dir / target).exists()

    @pytest.mark.parametrize(
        "targets",
        (
            ["xxx/xxx.txt"],
            [".envrc", ".run", "xxx/xxx.txt"],
        ),
    )
    def test_restore_bkp(self, targets):
        # given: backup created
        bkp_dir = Path.cwd() / CONFGUARD_BKP_DIR
        f = Files(rel_target_dir=SENTINEL, source_dir=Path.cwd(), targets=targets)
        f.create_bkp(source_dir=Path.cwd(), bkp_dir=bkp_dir)

        # when: all files are moved/deleted
        test_proj = ROOT_DIR / "tests/resources/test_proj"
        shutil.rmtree(test_proj / ".run", ignore_errors=True)  # will be linked
        Path(test_proj / ".run").unlink(missing_ok=True)  # will be linked
        Path(test_proj / ".envrc").unlink(missing_ok=True)  # will be linked
        Path(test_proj / "xxx/xxx.txt").unlink(missing_ok=True)  # will be linked

        f.restore_bkp(source_dir=Path.cwd(), bkp_dir=bkp_dir)

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
        ),
    )
    def test_delete_bkp(self, targets):
        bkp_dir = Path.cwd() / CONFGUARD_BKP_DIR
        f = Files(rel_target_dir=SENTINEL, source_dir=Path.cwd(), targets=targets)
        f.create_bkp(source_dir=Path.cwd(), bkp_dir=bkp_dir)

        f.delete_dir(dir_=bkp_dir)
        assert not bkp_dir.exists()

    def test_delete_nonexisting_bkp(self):
        bkp_dir = Path.cwd() / CONFGUARD_BKP_DIR
        f = Files(rel_target_dir=SENTINEL, source_dir=Path.cwd(), targets=[])

        f.delete_dir(dir_=bkp_dir)
        assert not bkp_dir.exists()

    @pytest.mark.parametrize(
        "targets",
        (
            ["xxx/xxx.txt"],
            [".run"],
            [".envrc", ".run"],
            [".envrc", ".run", "xxx/xxx.txt"],
        ),
    )
    def test_move_files(self, targets):
        f = Files(rel_target_dir=SENTINEL, source_dir=Path.cwd(), targets=targets)
        f.move_files(source_dir=Path.cwd(), target_dir=TARGET_DIR)
        for t in targets:
            assert Path(config.confguard_path / SENTINEL / t).exists()
            assert not Path(f.source_dir / t).exists()
            assert Path(config.confguard_path / SENTINEL / t) in f.target_locations

    @pytest.mark.parametrize(
        "targets",
        (
            ["xxx/xxx.txt"],
            [".envrc", ".run"],
            [".envrc", ".run", "xxx/xxx.txt"],
        ),
    )
    def test_return_files(self, targets):
        # given
        f = Files(rel_target_dir=SENTINEL, source_dir=Path.cwd(), targets=targets)
        f.move_files(source_dir=Path.cwd(), target_dir=TARGET_DIR)
        # when
        f.return_files(source_dir=Path.cwd(), target_dir=TARGET_DIR)
        # then all files exist at their source destination again
        for t in targets:
            assert Path(f.source_dir / t).exists()
            assert not Path(config.confguard_path / SENTINEL / t).exists()
        # entire target dir should be removed
        assert not Path(config.confguard_path / SENTINEL).exists()


class TestLinks:
    @pytest.mark.parametrize(
        "targets",
        (
            ["xxx/xxx.txt"],
            [".envrc", ".run"],
            [".envrc", ".run", "xxx/xxx.txt"],
        ),
    )
    def test_create_links(self, clear_test_proj, targets):
        lk = Links(source_dir=Path.cwd(), target_dir=TARGET_DIR, targets=targets)
        lk.create()
        for rel_path in targets:
            tgt_path = TARGET_DIR / rel_path
            src_path = Path.cwd() / rel_path
            assert src_path.is_symlink()

    @pytest.mark.parametrize("targets", ([".envrc", ".run", "xxx/xxx.txt"],))
    def test_create_links(self, clear_test_proj, targets):
        lk = Links(source_dir=Path.cwd(), target_dir=TARGET_DIR, targets=targets)
        lk.create(is_relative=True)
        for rel_path in targets:
            tgt_path = TARGET_DIR / rel_path
            src_path = Path.cwd() / rel_path
            assert src_path.is_symlink()

    @pytest.mark.parametrize("targets", ([".envrc", ".run", "xxx/xxx.txt"],))
    def test_remove_links(self, clear_test_proj, targets):
        lk = Links(source_dir=Path.cwd(), target_dir=TARGET_DIR, targets=targets)
        lk.create()

        lk.remove()
        for rel_path in targets:
            src_path = Path.cwd() / rel_path
            assert not src_path.exists()

    @pytest.mark.parametrize("targets", ([".envrc", ".run", "xxx/xxx.txt"],))
    def test_remove_non_existing_links(self, caplog, clear_test_proj, targets):
        caplog.set_level(logging.DEBUG)
        lk = Links(source_dir=Path.cwd(), target_dir=TARGET_DIR, targets=targets)
        lk.create()
        lk.remove()

        # when: remove non existing links
        lk.remove()
        # then: no error
        for rel_path in targets:
            src_path = Path.cwd() / rel_path
            assert not src_path.exists()
            assert f"{str(src_path)} does not exist" in caplog.text

    def test_back_create(self, create_sentinel):
        # given
        lk = Links(source_dir=Path.cwd(), target_dir=TARGET_DIR, targets=[])
        # when
        lk.back_create()
        # then
        assert Path(TARGET_DIR / f".{config.sentinel}.confguard").exists()
        assert Path(TARGET_DIR / f".{config.sentinel}.confguard").is_symlink()
        assert (
            Path(TARGET_DIR / f".{config.sentinel}.confguard").resolve() == Path.cwd()
        )

    def test_back_remove(self, create_sentinel):
        # given
        lk = Links(source_dir=Path.cwd(), target_dir=TARGET_DIR, targets=[])
        lk.back_create()
        # when
        lk.back_remove()
        # then
        assert not Path(TARGET_DIR / f".{config.sentinel}.confguard").exists()
