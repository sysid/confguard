from pathlib import Path

import pytest
import tomlkit
from click.exceptions import Exit
from typer.testing import CliRunner

from confguard.environment import CONFGUARD_CONFIG_FILE, config
from confguard.main import _find_and_link, _guard, _unguard, app
from confguard.model import ConfGuard
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
    cg = _guard(source_dir=TEST_PROJ)

    # then confguard directory is there
    confguard = list(Path(config.confguard_path).glob("**/test_proj-*"))
    assert len(confguard) == 1
    confguard = confguard[0]
    assert confguard.is_dir()
    assert confguard.name == cg.sentinel

    # then: confguard directory contains the files and dirs
    assert (confguard / "dot.envrc").is_file()
    assert (confguard / "dot.run").is_dir()
    assert (confguard / "xxx/xxx.txt").is_file()

    # then: .confguard backup exists
    assert (cg.target_dir / CONFGUARD_CONFIG_FILE).with_suffix(".bkp").is_file()

    # then: in source dir the files and dirs are replaced by links
    assert (TEST_PROJ / ".envrc").is_symlink()
    assert (TEST_PROJ / ".run").is_symlink()
    assert (TEST_PROJ / "xxx/xxx.txt").is_symlink()

    # then: the links point to the confguard directory replacements
    assert Path(TEST_PROJ / ".envrc").resolve() == Path(confguard / "dot.envrc")
    assert Path(TEST_PROJ / ".run").resolve() == Path(confguard / "dot.run")

    # then backlink created
    assert Path(confguard / f".{cg.sentinel}.confguard").resolve() == TEST_PROJ


def test__unguard():
    # given
    _ = _guard(source_dir=TEST_PROJ)
    confguard = list(Path(config.confguard_path).glob("**/test_proj-*"))[0]

    # when
    cg = _unguard(source_dir=TEST_PROJ)

    # then confguard directory is gone
    assert not confguard.exists()
    # then source directory has got the original files back
    assert (TEST_PROJ / ".envrc").is_file()
    assert (TEST_PROJ / ".run").is_dir()
    assert (TEST_PROJ / "xxx/xxx.txt").is_file()
    assert cg.sentinel is None


def test__guard_already_guarded(caplog, capsys):
    cg = _guard(source_dir=TEST_PROJ)

    with pytest.raises(Exit):
        cg = _guard(source_dir=TEST_PROJ)
    captured = capsys.readouterr()
    assert "nothing to do" in captured.out


def test__guard_with_changed_targets():
    # given a guarded project
    cg = _guard(source_dir=TEST_PROJ)

    # when the targets are changed
    path = TEST_PROJ / CONFGUARD_CONFIG_FILE
    with open(path, mode="rt", encoding="utf-8") as fp:
        toml = tomlkit.load(fp)

    toml["config"]["targets"] = ["xxx/xxx.txt"]  # remove .envrc and .run from targets

    with open(path, mode="wt", encoding="utf-8") as fp:
        tomlkit.dump(toml, fp)

    # and the project is again guarded
    cg = _guard(source_dir=TEST_PROJ)

    # then confguard directory is there
    confguard = list(Path(config.confguard_path).glob("**/test_proj-*"))
    assert len(confguard) == 1
    confguard = confguard[0]
    assert confguard.is_dir()
    assert confguard.name == cg.sentinel

    # then: confguard directory contains only the xxx/xxx.txt file
    assert (confguard / "xxx/xxx.txt").is_file()
    assert not (confguard / ".envrc").exists()
    assert not (confguard / ".run").exists()

    # then: in source dir ony the xxx/xxx.txt file is replaced by a link
    assert (TEST_PROJ / ".envrc").is_file()
    assert (TEST_PROJ / ".run").is_dir()
    assert (TEST_PROJ / "xxx/xxx.txt").is_symlink()

    # then: the links point to the confguard directory replacements
    assert Path(TEST_PROJ / "xxx/xxx.txt").resolve() == Path(confguard / "xxx/xxx.txt")

    # then backlink created
    assert Path(confguard / f".{cg.sentinel}.confguard").resolve() == TEST_PROJ


def test__find_and_link():
    # given a guarded project with borken links
    _ = _guard(source_dir=TEST_PROJ)
    (TEST_PROJ / CONFGUARD_CONFIG_FILE).unlink()
    assert not (TEST_PROJ / CONFGUARD_CONFIG_FILE).exists()
    (TEST_PROJ / ".envrc").unlink()
    assert not (TEST_PROJ / ".envrc").exists()
    (TEST_PROJ / "xxx/xxx.txt").unlink()
    assert not (TEST_PROJ / "xxx/xxx.txt").exists()

    # when project is relinked
    cg = _find_and_link(source_dir=TEST_PROJ)

    # then correct links are recreated
    # then confguard directory is there
    confguard = list(Path(config.confguard_path).glob("**/test_proj-*"))
    assert len(confguard) == 1
    confguard = confguard[0]
    assert confguard.is_dir()
    assert confguard.name == cg.sentinel

    # then: confguard directory contains the files and dirs
    assert (confguard / "dot.envrc").is_file()
    assert (confguard / "dot.run").is_dir()
    assert (confguard / "xxx/xxx.txt").is_file()

    # then: .confguard backup exists
    assert (cg.target_dir / CONFGUARD_CONFIG_FILE).with_suffix(".bkp").is_file()

    # then: in source dir the files and dirs are replaced by links
    assert (TEST_PROJ / ".envrc").is_symlink()
    assert (TEST_PROJ / ".run").is_symlink()
    assert (TEST_PROJ / "xxx/xxx.txt").is_symlink()

    # then: the links point to the confguard directory replacements
    assert Path(TEST_PROJ / ".envrc").resolve() == Path(confguard / "dot.envrc")
    assert Path(TEST_PROJ / ".run").resolve() == Path(confguard / "dot.run")

    # # then backlink created
    assert Path(confguard / f".{cg.sentinel}.confguard").resolve() == TEST_PROJ


def test_restore_toml():
    # given a guarded project with missing .confguard file
    cg = _guard(source_dir=TEST_PROJ)
    (TEST_PROJ / CONFGUARD_CONFIG_FILE).unlink()
    assert not (TEST_PROJ / CONFGUARD_CONFIG_FILE).exists()

    # when restore_toml is called
    ConfGuard.restore_toml(cg.source_dir, cg.target_dir)
    # then
    assert (TEST_PROJ / CONFGUARD_CONFIG_FILE).exists()


def test__guard_relative():

    # when relative paths are configured
    path = TEST_PROJ / CONFGUARD_CONFIG_FILE
    with open(path, mode="rt", encoding="utf-8") as fp:
        toml = tomlkit.load(fp)

    toml["config"]["relative"] = True

    with open(path, mode="wt", encoding="utf-8") as fp:
        tomlkit.dump(toml, fp)

    cg = _guard(source_dir=TEST_PROJ)

    # then confguard directory is there
    confguard = list(Path(config.confguard_path).glob("**/test_proj-*"))
    assert len(confguard) == 1
    confguard = confguard[0]
    assert confguard.is_dir()
    assert confguard.name == cg.sentinel

    # then: confguard directory contains the files and dirs
    assert (confguard / "dot.envrc").is_file()
    assert (confguard / "dot.run").is_dir()
    assert (confguard / "xxx/xxx.txt").is_file()

    # then: .confguard backup exists
    assert (cg.target_dir / CONFGUARD_CONFIG_FILE).with_suffix(".bkp").is_file()

    # then: in source dir the files and dirs are replaced by links
    assert (TEST_PROJ / ".envrc").is_symlink()
    assert (TEST_PROJ / ".run").is_symlink()
    assert (TEST_PROJ / "xxx/xxx.txt").is_symlink()

    # then: the links point to the confguard directory replacements
    assert Path(TEST_PROJ / ".envrc").resolve() == Path(confguard / "dot.envrc")
    assert Path(TEST_PROJ / ".run").resolve() == Path(confguard / "dot.run")

    # then backlink created
    assert Path(confguard / f".{cg.sentinel}.confguard").resolve() == TEST_PROJ
