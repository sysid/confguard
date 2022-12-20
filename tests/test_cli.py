import shutil
from pathlib import Path

import pytest
import typer
from typer.testing import CliRunner

from confguard.environment import config, ROOT_DIR
from confguard.main import app, _guard

runner = CliRunner()


class TestGuard:
    def test_guard(self):
        config_path = Path(typer.get_app_dir(config.app_name)) / "config.toml"
        shutil.rmtree(config_path.parent, ignore_errors=True)
        result = runner.invoke(app, ["guard", "DEFAULT", "-v"])
        print(result.stdout)
        assert result.exit_code == 0
        assert Path(config.confguard_path).exists()

    def test_guard_wrong_target(self):
        config_path = Path(typer.get_app_dir(config.app_name)) / "config.toml"
        shutil.rmtree(config_path.parent, ignore_errors=True)
        result = runner.invoke(app, ["guard", "XXXXXX", "-v"])
        print(result.stdout)
        assert result.exit_code == 1


def test__guard():
    test_proj = ROOT_DIR / "tests/resources/test_proj"
    _guard()

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
    assert (test_proj / ".envrc").is_symlink()
    assert (test_proj / ".run").is_symlink()
    assert (test_proj / "xxx/xxx.txt").is_symlink()

    # then: the links point to the confguard directory replacements
    fixed_tmp_path = str(Path(test_proj / ".envrc").resolve()).replace("/private", "")  # macos fix
    assert Path(fixed_tmp_path).resolve() == Path(confguard / ".envrc")
    fixed_tmp_path = str(Path(test_proj / ".run").resolve()).replace("/private", "")  # macos fix
    assert Path(fixed_tmp_path).resolve() == Path(confguard / ".run")

    # then backlink created
    assert Path(confguard / f".{config.sentinel}.confguard").resolve() == Path.cwd()

