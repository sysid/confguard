import logging
import shutil
from pathlib import Path

import pytest

from confguard.environment import ROOT_DIR, config, CONFGUARD_BKP_DIR

_log = logging.getLogger(__name__)
log_fmt = r"%(asctime)-15s %(levelname)s %(name)s %(funcName)s:%(lineno)d %(message)s"
datefmt = "%Y-%m-%d %H:%M:%S"
logging.basicConfig(format=log_fmt, level=logging.DEBUG, datefmt=None)
logging.getLogger("urllib3").setLevel(logging.WARNING)
logging.getLogger("asyncio").setLevel(logging.WARNING)
logging.getLogger("paramiko").setLevel(logging.INFO)


# run fixture before all tests
@pytest.fixture(autouse=True)
def test_proj():
    shutil.rmtree(config.confguard_path, ignore_errors=True)
    Path(config.confguard_path).mkdir(parents=True, exist_ok=True)

    test_proj = ROOT_DIR / "tests/resources/test_proj"
    ref_proj = ROOT_DIR / "tests/resources/ref_proj"

    #### NOT WORKING: LOADING config before results in lost file-pointer ####
    # shutil.rmtree(test_proj, ignore_errors=True)
    # shutil.copytree(ref_proj, test_proj)

    Path(test_proj / ".envrc").unlink(missing_ok=True)
    Path(test_proj / ".confguard").unlink(missing_ok=True)
    # delete existing sentinels
    for p in Path(test_proj).glob("**/.test_proj-*"):
        p.unlink()

    shutil.rmtree(test_proj / "xxx", ignore_errors=True)
    shutil.rmtree(test_proj / ".run", ignore_errors=True)  # if still dir
    shutil.rmtree(test_proj / CONFGUARD_BKP_DIR, ignore_errors=True)
    Path(test_proj / ".run").unlink(missing_ok=True)  # if already link

    shutil.copytree(ref_proj / "xxx", test_proj / "xxx")
    shutil.copytree(ref_proj / ".run", test_proj / ".run")
    shutil.copyfile(ref_proj / ".envrc", test_proj / ".envrc")
    shutil.copyfile(ref_proj / ".confguard", test_proj / ".confguard")
    _ = None


@pytest.fixture(autouse=False)
def clear_test_proj():
    test_proj = ROOT_DIR / "tests/resources/test_proj"
    # shutil.rmtree(test_proj / "xxx", ignore_errors=True)  # must exist
    shutil.rmtree(test_proj / ".run", ignore_errors=True)  # will be linked
    Path(test_proj / ".run").unlink(missing_ok=True)  # will be linked
    Path(test_proj / ".envrc").unlink(missing_ok=True)  # will be linked
    Path(test_proj / "xxx/xxx.txt").unlink(missing_ok=True)  # will be linked
