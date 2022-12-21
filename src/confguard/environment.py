################################################################################
# Base Environment
################################################################################
import os
import sys
from pathlib import Path

import pydantic
import typer
from pydantic import BaseSettings
from tomlkit import TOMLDocument

RUN_ENVS = ["local", "dev"]
ROOT_DIR = Path(__file__).parent.parent.parent.absolute()
FINGERPRINT = ".*.confguard"  # identifier for sentinel files
CONFGUARD_CONFIG_FILE = ".confguard"
CONFGUARD_BKP_DIR = "_confguard.tmp.bkp"

RUN_ENV = os.environ.get("RUN_ENV", "local").lower()
assert RUN_ENV in RUN_ENVS, f"RUN_ENV must be one of {RUN_ENVS}"

CONFIG_TEMPLATE = """\
#  vim: set ts=4 sw=4 tw=120 et ft=toml:
[config]
files = ['.envrc', '.run']
"""


class Environment(BaseSettings):
    app_name: str = "confguard"
    log_level: str = "INFO"
    twbm_db_url: str = "sqlite:///db/bm.db"
    confguard_path: Path
    confguard: TOMLDocument = {}

    # init
    def __init__(self, **data):
        super().__init__(**data)
        Path(self.confguard_path).mkdir(parents=True, exist_ok=True)

    @property
    def dbfile(self):
        return f"{self.twbm_db_url.split('sqlite:///')[-1]}"

    def log_config(self) -> dict:
        cfg = self.dict()
        skip_keys = (
            "secret_key",
            "sqlalchemy_database_uri",
        )
        sanitized_cfg = {k: v for k, v in cfg.items() if k not in skip_keys}
        return sanitized_cfg


try:
    config = Environment()
except pydantic.error_wrappers.ValidationError as e:
    typer.secho(
        f"CONFIGURATION ERROR: Make sure environment variable CONFGUARD_PATH is set.",
        fg=typer.colors.RED,
        err=True,
    )
    sys.exit(1)
