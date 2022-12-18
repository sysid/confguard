import shutil
from pathlib import Path

import pytest
import typer
from typer.testing import CliRunner

from confguard.environment import config
from confguard.main import app

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
