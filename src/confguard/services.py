import logging
import os
import shutil
import uuid
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

from confguard.environment import FINGERPRINT, config

_log = logging.getLogger(__name__)


@dataclass(frozen=True, kw_only=True)
class Sentinel:
    name: str
    source_dir: Path

    @staticmethod
    def create() -> 'Sentinel':
        source_dir = Path.cwd()
        try:
            p = source_dir.parts[-1]  # get proj dir as part of sentinel filename
        except IndexError:
            p = "unknown-dir"
        sentinel = list(source_dir.glob(FINGERPRINT))  # check existence
        if len(sentinel) > 0:
            _log.debug(f"Found sentinel: {sentinel}")
            return Sentinel(source_dir=source_dir, name=sentinel[0].name.split(".")[1])

        name = f"{p}-{uuid.uuid4().hex}"
        sentinel = f".{name}.{config.app_name}"
        with Path(sentinel).open("w") as f:
            msg = f"Created and managed by {config.app_name}. DO NOT REMOVE.\n{datetime.utcnow()}"
            print(msg, file=f)
        _log.debug(f"Created sentinel: {name}")
        return Sentinel(source_dir=source_dir, name=name)

    def remove(self) -> None:
        assert self.source_dir is not None, f"Sentinel {self.name} has no source_dir"
        sentinel = f".{self.name}.{config.app_name}"
        Path(self.source_dir / sentinel).unlink()
        _log.debug(f"Removed sentinel: {self.source_dir / sentinel}")


@dataclass(frozen=False, kw_only=True)
class Files:
    source_dir: Path
    rel_target_dir: Path
    target_dir: Path = field(init=False)
    targets: list[str]
    target_locations: list[Path] = field(default_factory=list)
    source_locations: list[Path] = field(default_factory=list)

    # init
    def __post_init__(self):
        self.target_dir = config.confguard_path / self.rel_target_dir

    def move_files(self) -> None:
        Path(self.target_dir).mkdir(parents=True, exist_ok=True)

        for t in self.targets:
            p = Path(t)
            if p.exists():
                _log.debug(f"Moving {p} to {self.target_dir / p}")
                (self.target_dir / p).parent.exists() or (self.target_dir / p).parent.mkdir(parents=True)
                p.rename(self.target_dir / p)
                self.source_locations.append(self.source_dir / p)
                self.target_locations.append(self.target_dir / p)
            else:
                _log.warning(f"File {p=} does not exist")  # TODO: better error handling

    def return_files(self) -> None:
        for target_location in self.target_locations:
            _log.debug(f"Moving {target_location} to {self.source_dir}")
            for target in self.targets:
                if target == str(target_location)[-len(target):]:
                    target_location.rename(self.source_dir / target)
                    break
        self.target_locations = []

        # check target dir only contains empty directories, use pathlib
        for p in self.target_dir.glob("**/*"):
            if p.is_file():
                raise Exception(f"Target dir {self.target_dir} is not empty")
        shutil.rmtree(self.target_dir)


def _create_relative_path(source: str, target: str) -> Path:
    source_path = Path(source).parent
    target_path = Path(target).parent

    if not (source_path.is_absolute() and target_path.is_absolute()):
        raise ValueError("Both source and target must be absolute paths")

    name = Path(source).name
    rel_path = os.path.relpath(target_path, source_path)
    return Path(rel_path) / name


@dataclass(frozen=True, kw_only=True)
class Links:
    source_locations: list[Path] = field(default_factory=list)
    target_locations: list[Path] = field(default_factory=list)
    links: list[str] = field(default_factory=list)

    def create_links(self, is_relative: bool = False) -> None:
        assert len(self.source_locations) == len(self.target_locations), \
            f"Source and target locations must be the same size, {self.source_locations=}, {self.target_locations=}"
        for source, target in zip(self.source_locations, self.target_locations):

            if is_relative:
                target = _create_relative_path(str(source), str(target))

            _log.debug(f"Creating link {source} to {target}")
            source.symlink_to(target)
            self.links.append(str(target))
            _ = None

        _log.debug(f"{self.links=}")
