import logging
import os
import shutil
import uuid
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

from confguard.environment import FINGERPRINT, config, CONFGUARD_BKP_DIR
from confguard.exceptions import BackupExistError, FileDoesNotExistError, BackupNotDeleted

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
        Path(self.source_dir / sentinel).unlink(missing_ok=True)
        _log.debug(f"Removed sentinel: {self.source_dir / sentinel}")


@dataclass(frozen=False, kw_only=True)
class Files:
    source_dir: Path
    rel_target_dir: Path
    target_dir: Path = field(init=False)
    bkp_dir: Path = field(init=False)
    targets: list[str]
    target_locations: list[Path] = field(default_factory=list)
    source_locations: list[Path] = field(default_factory=list)

    # init
    def __post_init__(self):
        self.target_dir = config.confguard_path / self.rel_target_dir
        self.bkp_dir = self.source_dir / CONFGUARD_BKP_DIR

    def move_files(self) -> None:
        Path(self.target_dir).mkdir(parents=True, exist_ok=True)

        for rel_path in self.targets:
            rel_path = Path(rel_path)
            target_path = self.target_dir / rel_path
            src_path = self.source_dir / rel_path
            if rel_path.exists():

                _log.debug(f"Moving {rel_path} to {target_path}")
                target_path.parent.exists() or target_path.parent.mkdir(parents=True)
                rel_path.rename(target_path)
                self.source_locations.append(src_path)
                self.target_locations.append(target_path)
            else:
                _log.warning(f"File {rel_path=} does not exist")

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
                raise Exception(f"Target dir {self.target_dir} is not empty. Will not delete it.")
        shutil.rmtree(self.target_dir)

    def create_bkp(self) -> None:
        try:
            Path(self.bkp_dir).mkdir(parents=True, exist_ok=False)
        except FileExistsError:
            raise BackupExistError(f"Backup dir {self.bkp_dir} already exists.")

        for rel_path in self.targets:
            rel_path = Path(rel_path)
            bkp_path = self.bkp_dir / rel_path
            src_path = self.source_dir / rel_path

            if rel_path.exists():
                if not rel_path.is_symlink():
                    if rel_path.is_file():
                        bkp_path.parent.exists() or bkp_path.parent.mkdir(parents=True)
                        shutil.copy2(rel_path, bkp_path)
                    elif rel_path.is_dir():
                        shutil.copytree(rel_path, bkp_path)
            else:
                _log.warning(f"File {rel_path=} does not exist")

    def restore_bkp(self) -> None:
        assert self.bkp_dir.exists(), f"Backup dir {self.bkp_dir} does not exist"
        for rel_path in self.targets:
            bkp_path = self.bkp_dir / rel_path
            src_path = self.source_dir / rel_path

            if bkp_path.exists():
                if bkp_path.is_file():
                    (self.source_dir / rel_path).parent.exists() or (self.source_dir / rel_path).parent.mkdir(
                        parents=True, exist_ok=True)

                    if src_path.exists() and src_path.is_symlink():
                        _log.warning(f"{src_path=} is already symlinked, backup will overwrite it with original file")
                        src_path.unlink()

                    if src_path.exists():
                        _log.info(f"{src_path=} exists, do nothing")
                    else:
                        _log.info(f"Restoring {src_path=}.")
                        shutil.copy2(bkp_path, src_path)

                elif bkp_path.is_dir():
                    _log.info(f"Restoring {src_path=}.")
                    shutil.copytree(bkp_path, src_path, dirs_exist_ok=True)
            else:
                _log.warning(f"File {bkp_path=} does not exist in backup. Cannot restore.")

    def delete_bkp_dir(self) -> None:
        try:
            shutil.rmtree(self.bkp_dir, ignore_errors=False)
        except FileNotFoundError:
            pass
        except Exception:
            raise BackupNotDeleted(f"Backup dir {self.bkp_dir} could not be deleted. Please delete it manually.")

    def delete_target_dir(self) -> None:
        try:
            shutil.rmtree(self.target_dir, ignore_errors=False)
        except FileNotFoundError:
            pass
        except Exception:
            raise BackupNotDeleted(f"Target dir {self.target_dir} could not be deleted. Please delete it manually.")


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

    def create(self, is_relative: bool = False) -> None:
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

    def remove(self) -> None:
        for link in self.source_locations:
            _log.debug(f"Removing link {link}")
            Path(link).unlink(missing_ok=True)
