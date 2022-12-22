import logging
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Protocol, TypeVar, runtime_checkable

import tomlkit
from tomlkit import TOMLDocument, table
from tomlkit.exceptions import NonExistentKey

from confguard.environment import CONFGUARD_CONFIG_FILE, config
from confguard.exceptions import InvalidConfigError
from confguard.helper import deserialize_from_base64, serialize_to_base64
from confguard.model import ConfGuard

_log = logging.getLogger(__name__)

AggT = TypeVar("AggT")


@runtime_checkable
class AbstractRepoSentinel(Protocol[AggT]):
    def add(self, agg: AggT) -> None:
        ...

    def get(self, id_: int) -> AggT:
        ...

    def flush(self) -> None:
        ...


@dataclass(frozen=False, kw_only=True)
class TomlRepoConfGuard:
    source_dir: Path
    path: Path = field(init=False)
    toml: TOMLDocument = field(init=False)

    def __post_init__(self) -> None:
        self.path = self.source_dir / CONFGUARD_CONFIG_FILE
        if not self.path.exists():
            raise FileNotFoundError(f"{self.path} does not exist")
        with open(self.path, mode="rt", encoding="utf-8") as fp:
            self.toml = tomlkit.load(fp)
            _log.debug(f"{self.toml=}")

    def get(self) -> ConfGuard:
        try:
            targets = self.toml["config"]["targets"]
        except NonExistentKey:
            raise InvalidConfigError(
                f"Invalid config in {self.path}, targets are missing."
            )
        try:
            is_relative = self.toml["config"]["relative"]
        except NonExistentKey:
            is_relative = False
        cg = ConfGuard(
            source_dir=self.source_dir,
            targets=targets,
            is_relative=is_relative,
        )
        try:
            sentinel = self.toml["_internal_"]["sentinel"]
            files = deserialize_from_base64(self.toml["_internal_"]["files"])
        except NonExistentKey:
            return cg

        cg.sentinel = sentinel
        cg.target_dir = config.confguard_path / sentinel
        cg.files = files
        return cg

    def add(self, confguard: ConfGuard) -> None:
        if confguard.sentinel is not None:
            if self.toml.get("_internal_") is not None:  # Update
                self.toml["_internal_"]["sentinel"] = confguard.sentinel
                self.toml["_internal_"]["files"] = tomlkit.string(
                    serialize_to_base64(confguard.targets), multiline=True
                )
            else:  # new
                intern = table()
                intern.add("sentinel", confguard.sentinel)
                intern.add("timestamp", datetime.utcnow())
                intern.add(
                    "files",
                    tomlkit.string(
                        serialize_to_base64(confguard.targets),
                        multiline=True,
                    ),
                )
                self.toml["_internal_"] = intern
                self.toml["_internal_"].comment("DO NOT EDIT FROM HERE")

        else:  # delete sentinel
            try:
                del self.toml["_internal_"]
            except NonExistentKey:
                pass

        with open(self.path, mode="wt", encoding="utf-8") as fp:
            tomlkit.dump(self.toml, fp)
        _log.debug(f"Saved config confguard: {self.path}")

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}({self.source_dir=})"

    def __str__(self) -> str:
        return f"{self.__class__.__name__}({self.source_dir=})"
