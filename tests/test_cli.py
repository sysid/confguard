import pytest
from typer.testing import CliRunner

from confguard.main import app

runner = CliRunner()


class TestSearch:
    def test_search_d(self):
        result = runner.invoke(app, ["guard", "xxx", "-v"], input="1 2\n")
        print(result.stdout)
        assert result.exit_code == 0
