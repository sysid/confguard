################################################################################
# Base Environment
################################################################################
import os
import sys
import tomllib
from enum import Enum
from pathlib import Path

import pydantic
import tomlkit
import typer
from pydantic import BaseSettings
from tomlkit import TOMLDocument, comment, nl, table

RUN_ENVS = ["local", "dev"]
ROOT_DIR = Path(__file__).parent.parent.parent.absolute()
FINGERPRINT = ".*.confguard"  # identifier for sentinel files
CONFGUARD_BKP_DIR = ".confguard.bkp"

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
    config_path: Path = Path(".confguard")

    # init
    def __init__(self, **data):
        super().__init__(**data)

        Path(self.confguard_path).mkdir(parents=True, exist_ok=True)

        # if not self.config_path.is_file():
        #     with open(self.config_path, "w") as textfile:
        #         print(CONFIG_TEMPLATE, file=textfile)

        # self.load_confguard()

    @property
    def dbfile(self):
        return f"{self.twbm_db_url.split('sqlite:///')[-1]}"

    @property
    def sentinel(self) -> str | None:
        try:
            return self.confguard["_internal_"]["sentinel"]
        except KeyError:
            return None

    def load_confguard(self):
        with open(self.config_path, mode="rt", encoding="utf-8") as fp:
            self.confguard = tomlkit.load(fp)

    def confguard_update_sentinel(self, sentinel: str) -> None:
        self.confguard["_internal_"]["sentinel"] = sentinel
        self._save_confguard()

    def confguard_remove_sentinel(self) -> None:
        del self.confguard["_internal_"]["sentinel"]
        del self.confguard["_internal_"]
        self._save_confguard()

    def confguard_add_sentinel(self, sentinel: str) -> None:
        # self.confguard.add(nl)
        tab = table()
        tab.add("sentinel", sentinel)
        self.confguard["_internal_"] = tab
        self.confguard["_internal_"].comment("DO NOT EDIT FROM HERE")
        self._save_confguard()

    def _save_confguard(self):
        with open(self.config_path, mode="wt", encoding="utf-8") as fp:
            tomlkit.dump(self.confguard, fp)

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
