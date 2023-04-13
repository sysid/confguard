import logging
import shutil
from pathlib import Path

import pytest

from confguard.environment import CONFGUARD_BKP_DIR, CONFGUARD_CONFIG_FILE
from confguard.exceptions import BackupExistError
from confguard.helper import denormalize_path, normalize_path
from confguard.model import ConfGuard
from tests.conftest import TARGET_DIR, TEST_PROJ


class TestSentinel:
    def test_conf_guard(self):
        cg = ConfGuard(source_dir=TEST_PROJ, targets=[".envrc", ".run", "xxx/xxx.txt"])
        assert cg.source_dir == TEST_PROJ
        assert isinstance(cg, ConfGuard)
        assert cg.sentinel is None

    def test_create_sentinel(self):
        cg = ConfGuard(source_dir=TEST_PROJ, targets=[".envrc", ".run", "xxx/xxx.txt"])
        cg.create_sentinel()
        assert "test_proj" in cg.sentinel

    def test_remove_sentinel(self):
        cg = ConfGuard(source_dir=TEST_PROJ, targets=[".envrc", ".run", "xxx/xxx.txt"])
        cg.create_sentinel()
        cg.remove_sentinel()
        assert cg.sentinel is None

    def test_backup_toml(self):
        cg = ConfGuard(source_dir=TEST_PROJ, targets=[".envrc", ".run", "xxx/xxx.txt"])
        cg.create_sentinel()
        cg.move_files()
        cg.backup_toml()
        toml_bkp = (cg.target_dir / CONFGUARD_CONFIG_FILE).with_suffix(".bkp")
        assert toml_bkp.exists()


class TestFiles:
    @pytest.mark.parametrize(
        ("targets", "expected"),
        (
            [["xxx/xxx.txt"], ["xxx/xxx.txt"]],
            [[".run"], ["dot.run"]],
            [[".run", ".envrc"], ["dot.run", "dot.envrc"]],
            [
                [".run", ".envrc", "xxx/xxx.txt"],
                ["dot.run", "dot.envrc", "xxx/xxx.txt"],
            ],
        ),
    )
    def test_move_files(self, targets, expected):
        cg = ConfGuard(source_dir=TEST_PROJ, targets=targets)
        cg.create_sentinel()
        cg.move_files()
        for t in targets:
            assert cg.target_dir.joinpath(normalize_path(Path(t))).exists()
            assert not cg.source_dir.joinpath(t).exists()

    @pytest.mark.parametrize(
        ("targets", "expected"),
        (
            # [["xxx/xxx.txt"], ["xxx/xxx.txt"]],
            [[".run"], ["dot.run"]],
            [
                [".run", ".envrc", "xxx/xxx.txt"],
                ["dot.run", "dot.envrc", "xxx/xxx.txt"],
            ],
        ),
    )
    def test_unmove_files(self, targets, expected):
        cg = ConfGuard(source_dir=TEST_PROJ, targets=targets)
        cg.files = targets  # would be loaded from toml state
        cg.create_sentinel()
        cg.move_files()

        # when: unmove files
        cg.unmove_files()
        for t in targets:
            assert not cg.target_dir.joinpath(normalize_path(Path(t))).exists()
            assert cg.source_dir.joinpath(t).exists()
        assert not cg.target_dir.exists()


class TestBackup:
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
        cg = ConfGuard(source_dir=TEST_PROJ, targets=targets)
        cg.create_bkp(TEST_PROJ, targets, normalizer=lambda x: x)

        bkp_dir = TEST_PROJ / CONFGUARD_BKP_DIR
        assert bkp_dir.exists()
        assert len(list(bkp_dir.glob("*"))) == len(targets)
        for target in targets:
            assert (bkp_dir / target).exists()

    @pytest.mark.parametrize("targets", ([".envrc"],))
    def test_create_bkp_but_bkp_dir_exists(self, targets):
        cg = ConfGuard(source_dir=TEST_PROJ, targets=targets)
        cg.create_bkp(TEST_PROJ, targets, normalizer=lambda x: x)

        with pytest.raises(BackupExistError) as e:
            cg.create_bkp(TEST_PROJ, targets, normalizer=lambda x: x)

    @pytest.mark.parametrize("targets", ([".envrc", ".run", "xxx/xxx.txt"],))
    def test_create_bkp_of_target_dir(self, targets):
        cg = ConfGuard(source_dir=TEST_PROJ, targets=targets)
        cg.create_sentinel()
        cg.move_files()

        cg.create_bkp(cg.target_dir, targets, normalizer=normalize_path)

        bkp_dir = cg.target_dir / CONFGUARD_BKP_DIR
        assert bkp_dir.exists()
        assert len(list(bkp_dir.glob("*"))) == len(targets)
        for target in targets:
            assert (normalize_path(bkp_dir / target)).exists()

    @pytest.mark.parametrize(
        "targets",
        (
            ["xxx/xxx.txt"],
            [".envrc", ".run", "xxx/xxx.txt"],
        ),
    )
    def test_restore_bkp(self, targets):
        # given: backup created
        cg = ConfGuard(source_dir=TEST_PROJ, targets=targets)
        cg.create_bkp(TEST_PROJ, targets, normalizer=lambda x: x)

        # when: all files are moved/deleted
        shutil.rmtree(TEST_PROJ / ".run", ignore_errors=True)  # will be linked
        (TEST_PROJ / ".run").unlink(missing_ok=True)  # will be linked
        (TEST_PROJ / ".envrc").unlink(missing_ok=True)  # will be linked
        (TEST_PROJ / "xxx/xxx.txt").unlink(missing_ok=True)  # will be linked

        cg.restore_bkp(TEST_PROJ, targets, normalizer=lambda x: x)

        # then: files are restored
        (TEST_PROJ / ".run").exists()
        (TEST_PROJ / ".run").is_dir()
        (TEST_PROJ / ".envrc").exists()
        (TEST_PROJ / ".envrc").is_file()
        (TEST_PROJ / "xxx/xxx.txt").exists()
        (TEST_PROJ / "xxx/xxx.txt").is_file()

    @pytest.mark.parametrize(
        "targets",
        ([".envrc", ".run", "xxx/xxx.txt"],),
    )
    def test_delete_bkp(self, targets):
        bkp_dir = TEST_PROJ / CONFGUARD_BKP_DIR
        cg = ConfGuard(source_dir=TEST_PROJ, targets=targets)
        cg.create_bkp(TEST_PROJ, targets, normalizer=lambda x: x)

        cg.delete_dir(dir_=bkp_dir)
        assert not bkp_dir.exists()

    def test_delete_nonexisting_bkp_should_pass_silently(self):
        bkp_dir = TEST_PROJ / CONFGUARD_BKP_DIR
        cg = ConfGuard(source_dir=TEST_PROJ, targets=[])

        cg.delete_dir(dir_=bkp_dir)
        assert not bkp_dir.exists()


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
        cg = ConfGuard(source_dir=TEST_PROJ, targets=targets)
        cg.create_sentinel()
        cg.create_lk(targets, normalizer=normalize_path)
        for rel_path in targets:
            tgt_path = normalize_path(TARGET_DIR / rel_path)
            src_path = TEST_PROJ / rel_path
            assert src_path.is_symlink()

    @pytest.mark.parametrize("targets", ([".envrc", ".run", "xxx/xxx.txt"],))
    def test_create_links_relative(self, clear_test_proj, targets):
        cg = ConfGuard(source_dir=TEST_PROJ, targets=targets, is_relative=True)
        cg.create_sentinel()
        cg.create_lk(targets, normalizer=normalize_path)
        for rel_path in targets:
            tgt_path = normalize_path(TARGET_DIR / rel_path)
            src_path = TEST_PROJ / rel_path
            assert src_path.is_symlink()

    @pytest.mark.parametrize("targets", ([".envrc", ".run", "xxx/xxx.txt"],))
    def test_remove_links(self, clear_test_proj, targets):
        cg = ConfGuard(source_dir=TEST_PROJ, targets=targets, is_relative=True)
        cg.create_sentinel()
        cg.create_lk(targets, normalizer=normalize_path)

        cg.remove_lk(targets)
        for rel_path in targets:
            src_path = TEST_PROJ / rel_path
            assert not src_path.exists()

    @pytest.mark.parametrize("targets", ([".envrc", ".run", "xxx/xxx.txt"],))
    def test_remove_non_existing_links(self, caplog, clear_test_proj, targets):
        caplog.set_level(logging.DEBUG)
        cg = ConfGuard(source_dir=TEST_PROJ, targets=targets, is_relative=True)
        cg.create_sentinel()
        cg.create_lk(targets, normalizer=normalize_path)
        cg.remove_lk(targets)

        # when: remove non existing links
        cg.remove_lk(targets)
        # then: no error
        for rel_path in targets:
            src_path = TEST_PROJ / rel_path
            assert not src_path.exists()
            assert f"{str(src_path)} is not a symlink" in caplog.text

    def test_back_create(self):
        # given
        cg = ConfGuard(source_dir=TEST_PROJ, targets=[])
        cg.create_sentinel()
        cg.move_files()
        # when
        cg.back_create()
        # then
        assert (cg.target_dir / f".{cg.sentinel}.confguard").exists()
        assert (cg.target_dir / f".{cg.sentinel}.confguard").is_symlink()
        assert (cg.target_dir / f".{cg.sentinel}.confguard").resolve() == TEST_PROJ

    def test_back_remove(self):
        # given
        cg = ConfGuard(source_dir=TEST_PROJ, targets=[])
        cg.create_sentinel()
        cg.move_files()
        cg.back_create()
        assert (cg.target_dir / f".{cg.sentinel}.confguard").exists()
        # when
        cg.back_remove()
        # then
        assert not (cg.target_dir / f".{cg.sentinel}.confguard").exists()
