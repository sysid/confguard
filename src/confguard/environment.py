################################################################################
# Base Environment
################################################################################
import os
from enum import Enum
from pathlib import Path

from pydantic import BaseSettings

RUN_ENVS = ["local", "dev"]
ROOT_DIR = Path(__file__).parent.absolute()

RUN_ENV = os.environ.get("RUN_ENV", "local").lower()
assert RUN_ENV in RUN_ENVS, f"RUN_ENV must be one of {RUN_ENVS}"


class Environment(BaseSettings):
    log_level: str = "INFO"
    twbm_db_url: str = "sqlite:///db/bm.db"

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


config = Environment()
_ = None
