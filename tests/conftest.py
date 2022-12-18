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


@pytest.fixture()
def test_proj():
    shutil.rmtree(config.confguard_path, ignore_errors=True)
    Path(config.confguard_path).mkdir(parents=True, exist_ok=True)

    Path(ROOT_DIR / "tests/resources/test_proj/.envrc").unlink(missing_ok=True)

    shutil.rmtree(ROOT_DIR / "tests/resources/test_proj/.run", ignore_errors=True)
    Path(ROOT_DIR / "tests/resources/test_proj/.run").unlink(missing_ok=True)

    shutil.copytree(ROOT_DIR / "tests/resources/ref_proj/.run", ROOT_DIR / "tests/resources/test_proj/.run")
    shutil.copyfile(ROOT_DIR / "tests/resources/ref_proj/.envrc", ROOT_DIR / "tests/resources/test_proj/.envrc")
    _ = None
