################################################################################
# Base Environment
################################################################################
import os
import tomllib
from enum import Enum
from pathlib import Path

import typer
from pydantic import BaseSettings

RUN_ENVS = ["local", "dev"]
ROOT_DIR = Path(__file__).parent.parent.parent.absolute()
FINGERPRINT = ".*.confguard"  # identifier for sentinel files

RUN_ENV = os.environ.get("RUN_ENV", "local").lower()
assert RUN_ENV in RUN_ENVS, f"RUN_ENV must be one of {RUN_ENVS}"

CONFIG_TEMPLATE = """\
[DEFAULT]
targets = [".envrc",]

[PYTHON]
targets = [".envrc", ".run", "xxx/xxx.txt"]
"""


class Environment(BaseSettings):
    app_name: str = "confguard"
    log_level: str = "INFO"
    twbm_db_url: str = "sqlite:///db/bm.db"
    confguard_path: Path = ROOT_DIR / "savedir"
    confguard: dict = {}
    config_path: Path = None

    # init
    def __init__(self, **data):
        super().__init__(**data)

        Path(self.confguard_path).mkdir(parents=True, exist_ok=True)

        self.config_path = Path(typer.get_app_dir(self.app_name)) / "config.toml"
        if not self.config_path.is_file():
            Path(self.config_path.parent).mkdir(parents=True, exist_ok=True)
            with open(self.config_path, "w") as textfile:
                print(CONFIG_TEMPLATE, file=textfile)

        with open(self.config_path, "rb") as f:
            self.confguard = tomllib.load(f)

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
