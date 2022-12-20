import logging
import shutil
from pathlib import Path

import pytest
from tomlkit import table

from confguard.environment import CONFGUARD_BKP_DIR, ROOT_DIR, config

_log = logging.getLogger(__name__)
log_fmt = r"%(asctime)-15s %(levelname)s %(name)s %(funcName)s:%(lineno)d %(message)s"
logging.basicConfig(format=log_fmt, level=logging.DEBUG, datefmt="%Y-%m-%d %H:%M:%S")

SENTINEL = Path("test_proj-1234")
TARGET_DIR = config.confguard_path / SENTINEL
TEST_PROJ = ROOT_DIR / "tests/resources/test_proj"
REF_PROJ = ROOT_DIR / "tests/resources/ref_proj"


# run fixture before all tests
@pytest.fixture(autouse=True)
def test_proj():
    shutil.rmtree(config.confguard_path, ignore_errors=True)
    Path(config.confguard_path).mkdir(parents=True, exist_ok=True)

    #### NOT WORKING: LOADING config before results in lost file-pointer ####
    # shutil.rmtree(test_proj, ignore_errors=True)
    # shutil.copytree(REF_PROJ, test_proj)

    Path(TEST_PROJ / ".envrc").unlink(missing_ok=True)
    Path(TEST_PROJ / ".confguard").unlink(missing_ok=True)
    # delete existing sentinels
    for p in Path(TEST_PROJ).glob("**/.test_proj-*"):
        p.unlink()

    shutil.rmtree(TEST_PROJ / "xxx", ignore_errors=True)
    shutil.rmtree(TEST_PROJ / ".run", ignore_errors=True)  # if still dir
    shutil.rmtree(TEST_PROJ / CONFGUARD_BKP_DIR, ignore_errors=True)
    Path(TEST_PROJ / ".run").unlink(missing_ok=True)  # if already link

    shutil.copytree(REF_PROJ / "xxx", TEST_PROJ / "xxx")
    shutil.copytree(REF_PROJ / ".run", TEST_PROJ / ".run")
    shutil.copyfile(REF_PROJ / ".envrc", TEST_PROJ / ".envrc")
    shutil.copyfile(REF_PROJ / ".confguard", TEST_PROJ / ".confguard")
    _ = None


@pytest.fixture(autouse=False)
def clear_test_proj():
    shutil.rmtree(TEST_PROJ / ".run", ignore_errors=True)  # will be linked
    Path(TEST_PROJ / ".run").unlink(missing_ok=True)  # will be linked
    Path(TEST_PROJ / ".envrc").unlink(missing_ok=True)  # will be linked
    Path(TEST_PROJ / "xxx/xxx.txt").unlink(missing_ok=True)  # will be linked


@pytest.fixture(autouse=False)
def create_sentinel():
    tab = table()
    tab.add("sentinel", str(SENTINEL))
    config.confguard["_internal_"] = tab
    config.confguard["_internal_"].comment("DO NOT EDIT FROM HERE")
    # noinspection PyProtectedMember
    config._save_confguard()
    Path(TARGET_DIR).mkdir(parents=True, exist_ok=True)
