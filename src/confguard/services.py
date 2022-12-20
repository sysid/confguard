import logging
import os
import shutil
import uuid
from dataclasses import dataclass, field
from pathlib import Path

import tomlkit
from tomlkit import table

from confguard.environment import config, CONFGUARD_CONFIG_FILE
from confguard.exceptions import (
    BackupExistError,
    BackupNotDeleted,
)

_log = logging.getLogger(__name__)


@dataclass(frozen=False, kw_only=True)
class Sentinel:
    """
    Delegates sentinel management to environment class
    """
    source_dir: Path
    config_path: Path = field(init=False)

    # init
    def __post_init__(self):
        self.config_path = self.source_dir / CONFGUARD_CONFIG_FILE

    def create(self) -> None:
        if config.sentinel is not None:
            _log.debug(f"Sentinel already exists: {config.sentinel=}")
            return

        try:
            p = self.source_dir.parts[-1]  # get proj dir as part of sentinel filename
        except IndexError:
            p = "unknown-dir"

        sentinel = f"{p}-{uuid.uuid4().hex[:8]}"
        self.confguard_add_sentinel(sentinel)
        _log.debug(f"Sentinel created: {config.sentinel=}")

    def confguard_add_sentinel(self, sentinel: str) -> None:
        # self.confguard.add(nl)
        tab = table()
        tab.add("sentinel", sentinel)
        config.confguard["_internal_"] = tab
        config.confguard["_internal_"].comment("DO NOT EDIT FROM HERE")
        self._save_confguard()

    def remove(self) -> None:
        self.confguard_remove_sentinel()
        _log.debug(f"Sentinel removed: {config.sentinel=}")

    def load_confguard(self):
        with open(self.config_path, mode="rt", encoding="utf-8") as fp:
            config.confguard = tomlkit.load(fp)

    def confguard_update_sentinel(self, sentinel: str) -> None:
        config.confguard["_internal_"]["sentinel"] = sentinel
        self._save_confguard()

    def confguard_remove_sentinel(self) -> None:
        del config.confguard["_internal_"]["sentinel"]
        del config.confguard["_internal_"]
        self._save_confguard()

    def _save_confguard(self):
        config_path = self.source_dir / CONFGUARD_CONFIG_FILE
        with open(config_path, mode="wt", encoding="utf-8") as fp:
            tomlkit.dump(config.confguard, fp)


@dataclass(frozen=False, kw_only=True)
class Files:
    source_dir: Path
    rel_target_dir: Path
    target_dir: Path = field(init=False)
    # bkp_dir: Path = field(init=False)
    targets: list[str]

    # target_locations: list[Path] = field(default_factory=list)
    # source_locations: list[Path] = field(default_factory=list)

    # init
    def __post_init__(self):
        self.target_dir = config.confguard_path / self.rel_target_dir

    def move_files(self, source_dir: Path, target_dir: Path) -> None:
        Path(target_dir).mkdir(parents=True, exist_ok=True)

        for rel_path in self.targets:
            rel_path = Path(rel_path)
            tgt_path = target_dir / rel_path
            src_path = source_dir / rel_path

            if src_path.exists():
                _log.debug(f"Moving {src_path} to {tgt_path}")
                tgt_path.parent.exists() or tgt_path.parent.mkdir(parents=True)
                src_path.rename(tgt_path)
            else:
                _log.warning(f"File {src_path=} does not exist")

    def return_files(self, source_dir: Path, target_dir: Path) -> None:
        self.move_files(target_dir, source_dir)
        shutil.rmtree(target_dir)

    def create_bkp(self, source_dir: Path, bkp_dir: Path) -> None:
        try:
            Path(bkp_dir).mkdir(parents=True, exist_ok=False)
        except FileExistsError:
            raise BackupExistError(f"Backup dir {bkp_dir} already exists.")

        for rel_path in self.targets:
            bkp_path = bkp_dir / rel_path
            src_path = source_dir / rel_path

            if src_path.exists():
                if not src_path.is_symlink():
                    if src_path.is_file():
                        bkp_path.parent.exists() or bkp_path.parent.mkdir(parents=True)
                        shutil.copy2(src_path, bkp_path)
                    elif src_path.is_dir():
                        shutil.copytree(src_path, bkp_path)
                else:
                    _log.warning(f"File {src_path=} is a symlink. Skipping backup.")
            else:
                _log.warning(f"File {src_path=} does not exist")

    def restore_bkp(self, source_dir: Path, bkp_dir: Path) -> None:
        assert bkp_dir.exists(), f"Backup dir {bkp_dir} does not exist"
        for rel_path in self.targets:
            bkp_path = bkp_dir / rel_path
            src_path = source_dir / rel_path

            if bkp_path.exists():
                if bkp_path.is_file():
                    (self.source_dir / rel_path).parent.exists() or (
                        self.source_dir / rel_path
                    ).parent.mkdir(parents=True, exist_ok=True)

                    if src_path.exists() and src_path.is_symlink():
                        _log.warning(
                            f"{src_path=} is already symlinked, backup will overwrite it with original file"
                        )
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
                _log.warning(
                    f"File {bkp_path=} does not exist in backup. Cannot restore."
                )

    @staticmethod
    def delete_dir(dir_: Path) -> None:
        try:
            shutil.rmtree(dir_, ignore_errors=False)
        except FileNotFoundError:
            pass
        except Exception:
            raise BackupNotDeleted(
                f"Backup dir {dir_} could not be deleted. Please delete it manually."
            )


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
    targets: list[str]
    source_dir: Path
    target_dir: Path

    def create(self, is_relative: bool = False) -> None:
        for rel_path in self.targets:
            tgt_path = self.target_dir / rel_path
            src_path = self.source_dir / rel_path

            if is_relative:
                tgt_path = _create_relative_path(str(src_path), str(tgt_path))

            _log.debug(f"Creating link {src_path} to {tgt_path}")
            src_path.symlink_to(tgt_path)
            _ = None

    def remove(self) -> None:
        for rel_path in self.targets:
            # tgt_path = self.target_dir / rel_path
            src_path = self.source_dir / rel_path

            if src_path.exists():
                if src_path.is_symlink():
                    _log.debug(f"Removing link {src_path}")
                    src_path.unlink(missing_ok=True)
                else:
                    _log.warning(
                        f"File {src_path=} is not a symlink. Skipping removal."
                    )
            else:
                _log.warning(f"{src_path} does not exist")

    def back_create(self, is_relative: bool = False) -> None:
        target = self.source_dir
        source = self.target_dir / f".{config.sentinel}.confguard"

        if is_relative:
            target = _create_relative_path(str(source), str(target))

        _log.debug(f"Creating link {source} to {target}")
        source.symlink_to(target)

    def back_remove(self):
        source = self.target_dir / f".{config.sentinel}.confguard"
        _log.debug(f"Removing link {source}")
        source.unlink(missing_ok=True)
