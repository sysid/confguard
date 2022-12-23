import logging
import shutil
import uuid
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from confguard.environment import CONFGUARD_BKP_DIR, CONFGUARD_CONFIG_FILE, config
from confguard.exceptions import BackupExistError, DirectoryNotDeleted
from confguard.helper import _create_relative_path

_log = logging.getLogger(__name__)


@dataclass(frozen=False, kw_only=True, repr=False)
class ConfGuard:
    source_dir: Path
    target_dir: Path = None
    targets: list[str] = None
    files: list[str] = None
    config_path: Path = field(init=False)
    sentinel: Optional[str] = None
    is_relative: bool = False

    # files: Files
    # links: Links

    def __post_init__(self):
        self.config_path = self.source_dir / CONFGUARD_CONFIG_FILE

    def create_sentinel(self) -> None:
        if self.sentinel is not None:
            _log.debug(f"Sentinel already exists: {self.sentinel=}")
            return

        self.sentinel = f"{self.source_dir.name}-{uuid.uuid4().hex[:8]}"
        self.target_dir = config.confguard_path / self.sentinel
        _log.debug(f"Sentinel created: {self.sentinel=}")

    def remove_sentinel(self) -> None:
        self.sentinel = None

    @staticmethod
    def _move_files(source_dir: Path, target_dir: Path, targets: list[str]) -> None:
        for rel_path in targets:
            tgt_path = target_dir / rel_path
            src_path = source_dir / rel_path

            if src_path.exists():
                _log.debug(f"Moving {src_path} to {tgt_path}")
                tgt_path.parent.exists() or tgt_path.parent.mkdir(parents=True)
                src_path.rename(tgt_path)
            else:
                _log.warning(f"{src_path} does not exist")

    def move_files(self) -> None:
        assert self.sentinel is not None, "Sentinel not created"
        Path(self.target_dir).mkdir(parents=True, exist_ok=True)
        self._move_files(self.source_dir, self.target_dir, targets=self.targets)

    def unmove_files(self) -> None:
        """Restore files from confguard directory, based on saved file list"""
        self._move_files(self.target_dir, self.source_dir, self.files)
        shutil.rmtree(self.target_dir)

    @staticmethod
    def _create_bkp(source_dir: Path, bkp_dir: Path, targets: list[str]) -> None:
        try:
            Path(bkp_dir).mkdir(parents=True, exist_ok=False)
        except FileExistsError:
            raise BackupExistError(f"Backup dir {bkp_dir} already exists.")

        for rel_path in targets:
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
                    _log.info(
                        f"{src_path} is a symlink. Skipping backup.",
                        extra={"highlighter": None},
                    )
            else:
                _log.warning(f"{src_path} does not exist")

    def create_bkp(self, dir_: Path, targets: list[str]) -> None:
        bkp_dir = dir_ / CONFGUARD_BKP_DIR
        self._create_bkp(dir_, bkp_dir, targets)

    @staticmethod
    def _restore_bkp(source_dir: Path, bkp_dir: Path, targets: list[str]) -> None:
        assert bkp_dir.exists(), f"Backup dir {bkp_dir} does not exist"
        for rel_path in targets:
            bkp_path = bkp_dir / rel_path
            src_path = source_dir / rel_path

            if bkp_path.exists():
                if bkp_path.is_file():
                    src_path.parent.exists() or src_path.parent.mkdir(
                        parents=True, exist_ok=True
                    )

                    if src_path.exists() and src_path.is_symlink():
                        _log.warning(
                            f"{src_path} is already symlinked, backup will overwrite it with original file"
                        )
                        src_path.unlink()

                    if src_path.exists():
                        _log.info(
                            f"{src_path} exists, do nothing",
                            extra={"highlighter": None},
                        )
                    else:
                        _log.info(f"Restoring {src_path}.", extra={"highlighter": None})
                        shutil.copy2(bkp_path, src_path)

                elif bkp_path.is_dir():
                    _log.info(f"Restoring {src_path}.", extra={"highlighter": None})
                    shutil.copytree(bkp_path, src_path, dirs_exist_ok=True)
            else:
                _log.warning(
                    f"File {bkp_path} does not exist in backup. Cannot restore."
                )

    def restore_bkp(self, dir_: Path, targets: list[str]) -> None:
        bkp_dir = dir_ / CONFGUARD_BKP_DIR
        self._restore_bkp(dir_, bkp_dir, targets)

    @staticmethod
    def delete_dir(dir_: Path) -> None:
        try:
            shutil.rmtree(dir_, ignore_errors=False)
        except FileNotFoundError:
            pass
        except Exception:
            raise DirectoryNotDeleted(
                f"{dir_} could not be deleted. Please delete it manually."
            )

    def create_lk(self, targets: list[str]) -> None:
        for rel_path in targets:
            tgt_path = self.target_dir / rel_path
            src_path = self.source_dir / rel_path

            if self.is_relative:
                tgt_path = _create_relative_path(str(src_path), str(tgt_path))

            _log.debug(f"Creating link {src_path} to {tgt_path}")
            src_path.symlink_to(tgt_path)
            _ = None

    def remove_lk(self, targets: list[str]) -> None:
        for rel_path in targets:
            src_path = self.source_dir / rel_path

            if src_path.is_symlink():
                _log.debug(f"Removing link {src_path}")
                src_path.unlink(missing_ok=True)
            else:
                _log.info(
                    f"File {str(src_path)} is not a symlink. Skipping removal.",
                    extra={"highlighter": None},
                )

    def back_create(self) -> None:
        target = self.source_dir
        source = self.target_dir / f".{self.sentinel}.confguard"

        if self.is_relative:
            target = _create_relative_path(str(source), str(target))

        _log.debug(f"Creating link {source} to {target}")
        source.symlink_to(target)

    def back_remove(self):
        source = self.target_dir / f".{self.sentinel}.confguard"
        _log.debug(f"Removing link {source}")
        source.unlink(missing_ok=True)

    def backup_toml(self) -> None:
        """Backup toml file
        IMPORTANT: ensure that the relevant state is saved in the toml file before backing up.
        """
        toml = self.source_dir / CONFGUARD_CONFIG_FILE
        toml_bkp = (self.target_dir / CONFGUARD_CONFIG_FILE).with_suffix(".bkp")
        shutil.copy2(toml, toml_bkp)

    @staticmethod
    def restore_toml(source_dir: Path, target_dir: Path) -> Path:
        toml = source_dir / CONFGUARD_CONFIG_FILE
        toml_bkp = (target_dir / CONFGUARD_CONFIG_FILE).with_suffix(".bkp")
        shutil.copy2(toml_bkp, toml)
        _log.info(f"Restored configuration file: {toml}")
        return toml

    def __repr__(self) -> str:
        return f"ConfGuard({self.source_dir}, {self.target_dir}, {self.targets})"
