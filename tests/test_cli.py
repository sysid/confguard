import shutil
from pathlib import Path

import pytest
import typer
from typer.testing import CliRunner

from confguard.environment import ROOT_DIR, config
from confguard.main import _guard, _unguard, app
from tests.conftest import TEST_PROJ

runner = CliRunner()


class TestGuard:
    def test_guard(self, caplog):
        caplog.set_level(
            100000
        )  # BUG: https://github.com/pallets/click/issues/824#issuecomment-562581313
        result = runner.invoke(app, ["guard", str(TEST_PROJ)])
        print(result.stdout)
        assert result.exit_code == 0
        # assert Path(config.confguard_path).exists()

    def test_unguard(self, caplog):
        # given guarded project
        caplog.set_level(
            100000
        )  # BUG: https://github.com/pallets/click/issues/824#issuecomment-562581313
        _guard(source_dir=TEST_PROJ)
        # when
        result = runner.invoke(app, ["unguard", str(TEST_PROJ)])
        # then
        print(result.stdout)
        assert result.exit_code == 0


def test__guard():
    _guard(source_dir=TEST_PROJ)

    # then confguard directory is there
    confguard = list(Path(config.confguard_path).glob("**/test_proj-*"))
    assert len(confguard) == 1
    confguard = confguard[0]
    assert confguard.is_dir()
    assert confguard.parts[-1] == config.sentinel

    # then: confguard directory contains the files and dirs
    assert (confguard / ".envrc").is_file()
    assert (confguard / ".run").is_dir()
    assert (confguard / "xxx/xxx.txt").is_file()

    # then: in source dir the files and dirs are replaced by links
    assert (TEST_PROJ / ".envrc").is_symlink()
    assert (TEST_PROJ / ".run").is_symlink()
    assert (TEST_PROJ / "xxx/xxx.txt").is_symlink()

    # then: the links point to the confguard directory replacements
    assert Path(TEST_PROJ / ".envrc").resolve() == Path(confguard / ".envrc")
    assert Path(TEST_PROJ / ".run").resolve() == Path(confguard / ".run")

    # then backlink created
    assert Path(confguard / f".{config.sentinel}.confguard").resolve() == TEST_PROJ


def test__unguard():
    # given
    _guard(source_dir=TEST_PROJ)
    confguard = list(Path(config.confguard_path).glob("**/test_proj-*"))[0]

    # when
    _unguard(source_dir=TEST_PROJ)

    # then confguard directory is gone
    assert not confguard.exists()
    # then source directory has got the original files back
    assert (TEST_PROJ / ".envrc").is_file()
    assert (TEST_PROJ / ".run").is_dir()
    assert (TEST_PROJ / "xxx/xxx.txt").is_file()
    assert config.sentinel is None
