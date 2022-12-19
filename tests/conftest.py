import logging
import shutil
from pathlib import Path

import pytest

from confguard.environment import ROOT_DIR, config

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

    Path(test_proj / ".envrc").unlink(missing_ok=True)
    for p in Path(test_proj).glob("**/.test_proj-*"):
        p.unlink()

    shutil.rmtree(test_proj / ".run", ignore_errors=True)  # if still dir
    Path(test_proj / ".run").unlink(missing_ok=True)  # if already link

    shutil.copytree(ref_proj / ".run", test_proj / ".run")
    shutil.copyfile(ref_proj / ".envrc", test_proj / ".envrc")
    _ = None
