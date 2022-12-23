import logging
from pathlib import Path

import typer
from rich.console import Console
from rich.logging import RichHandler
from rich.theme import Theme

from confguard.adapter import TomlRepoConfGuard
from confguard.environment import CONFGUARD_BKP_DIR, CONFGUARD_CONFIG_FILE, config
from confguard.exceptions import InvalidConfigError
from confguard.model import ConfGuard

_log = logging.getLogger(__name__)
app = typer.Typer(help="Save sensitive configuration in a save place")


@app.command()
def guard(
    source_dir: Path = typer.Argument(
        ..., help="Path to the directory to guard", exists=True
    ),
):
    """Guards a directory.
    Configuration: `.confguard` in project directory

    CAVEAT: relative linking cannot span mounts, absolute linking can
    """
    source_dir = Path(source_dir).expanduser().resolve()
    if not (source_dir / CONFGUARD_CONFIG_FILE).exists():
        typer.secho(
            f"Configuration file {CONFGUARD_CONFIG_FILE} not found in {source_dir}",
            fg=typer.colors.RED,
            err=True,
        )
        raise typer.Exit(1)
    cg = _guard(source_dir)
    typer.secho(
        f"Project {source_dir} is now guarded. Sensitive files are now in {cg.target_dir}",
        fg=typer.colors.GREEN,
    )


def _guard(source_dir: Path) -> ConfGuard:
    repo = TomlRepoConfGuard(source_dir=source_dir)
    try:
        cg = repo.get()
    except InvalidConfigError as e:
        typer.secho(str(e), fg=typer.colors.RED, err=True)
        raise typer.Exit(1)

    if cg.sentinel is not None:
        if cg.files == cg.targets:
            typer.secho(
                f"Project is already guarded, nothing to do.",
                fg=typer.colors.GREEN,
            )
            raise typer.Exit(0)
        else:
            _log.debug(f"Project is already guarded, but not all files are guarded.")
            _unguard(source_dir)  # get everything back and recreate with new config

    _log.info(f"Guarding {source_dir}")

    cg.create_sentinel()
    try:
        cg.create_bkp(cg.source_dir, cg.targets)
    except Exception as e:
        typer.secho(f"Error occurred, Aborting: {e}", fg=typer.colors.RED)
        cg.delete_dir(dir_=cg.source_dir / CONFGUARD_BKP_DIR)
        cg.remove_sentinel()
        repo.add(cg)  # save it
        raise typer.Abort(1)

    try:
        cg.move_files()
        cg.create_lk(cg.targets)
        cg.back_create()
    except Exception as e:
        typer.secho(f"Error occurred, rolling back: {e}", fg=typer.colors.RED)
        cg.remove_lk(cg.targets)
        cg.back_remove()
        cg.restore_bkp(cg.source_dir, cg.targets)
        cg.remove_sentinel()
        raise typer.Abort(1)
    finally:
        repo.add(cg)  # save it
        cg.delete_dir(dir_=cg.source_dir / CONFGUARD_BKP_DIR)
    cg.backup_toml()
    return cg


@app.command()
def unguard(
    # path argument
    source_dir: Path = typer.Argument(
        ..., help="Path to the directory to guard", exists=True
    ),
):
    """Un-guards a directory.
    Revert changes made by `guard`.
    """
    source_dir = Path(source_dir).expanduser().resolve()
    _ = _unguard(source_dir)
    typer.secho(
        f"Project {source_dir} is now un-guarded.",
        fg=typer.colors.GREEN,
    )


def _unguard(source_dir: Path) -> ConfGuard:
    repo = TomlRepoConfGuard(source_dir=source_dir)
    try:
        cg = repo.get()
    except InvalidConfigError as e:
        typer.secho(str(e), fg=typer.colors.RED, err=True)
        raise typer.Exit(1)

    if cg.sentinel is None:
        typer.secho(
            f"Project is not guarded, nothing to do.",
            fg=typer.colors.GREEN,
        )
        raise typer.Exit(1)

    _log.info(f"Un-guarding {source_dir}")

    try:
        cg.create_bkp(cg.target_dir, cg.files)
    except Exception as e:
        typer.secho(f"Error occurred, Aborting: {e}", fg=typer.colors.RED)
        cg.delete_dir(dir_=cg.target_dir / CONFGUARD_BKP_DIR)
        cg.remove_sentinel()
        repo.add(cg)  # save it
        raise typer.Abort(1)

    try:
        cg.remove_lk(cg.files)
        cg.back_remove()
        cg.unmove_files()
        cg.remove_sentinel()
    except Exception as e:
        _log.error(f"Error occurred, rolling back: {e}")
        cg.restore_bkp(cg.target_dir, cg.files)
        try:
            cg.create_lk(cg.files)
        except Exception as e:
            _log.warning(f"Manual intervention required: {e}")
        try:
            cg.back_create()
        except Exception as e:
            _log.error(f"Manual intervention required: {e}")
        raise typer.Abort(1)
    finally:
        repo.add(cg)  # save it
        cg.delete_dir(dir_=cg.target_dir / CONFGUARD_BKP_DIR)
    return cg


@app.command()
def find_and_link(
    source_dir: Path = typer.Argument(
        ..., help="Path to the directory to guard", exists=True
    ),
) -> None:
    """Missing .confguard file, try to find it and link it
    Searches CONFGUARD_PATH for project and re-links it.
    This allows moving the source directory. The links will be recreated correctly.
    """
    cg = _find_and_link(source_dir)
    typer.secho(
        f"Project {source_dir} is now re-linked and guarded. Sensitive Files are in {cg.target_dir}.",
        fg=typer.colors.GREEN,
    )


def _find_and_link(source_dir: Path) -> ConfGuard:
    projects = [
        p
        for p in Path(config.confguard_path).glob("*")
        if p.name.split("-")[0] == source_dir.name
    ]
    if len(projects) > 1:
        typer.secho(
            f"Found more than one project for {source_dir.name}, resolve manually.",
            fg=typer.colors.RED,
        )
        raise typer.Exit(1)
    if len(projects) == 0:
        typer.secho(
            f"No matching project found in {config.confguard_path} for {source_dir.name}. Start guarding your project.",
            fg=typer.colors.RED,
        )
        raise typer.Exit(1)
    project = projects[0]
    _log.info(f"Found guarded project files for {project}, re-linking it.")

    ConfGuard.restore_toml(source_dir, project)
    _ = _unguard(source_dir)
    return _guard(source_dir)


@app.callback()
def main(
    verbose: bool = typer.Option(False, "-v", "--verbose", help="verbosity"),
):
    # log_fmt = r"%(asctime)-15s %(levelname)-7s %(message)s"
    log_fmt = r"%(message)s"
    # https://github.com/Textualize/rich/issues/1161#issuecomment-813882224
    # https://stackoverflow.com/questions/69348880/is-it-possible-to-use-background-aware-color-choices
    console = Console(
        theme=Theme(
            {
                "logging.level.debug": "yellow",
                "logging.level.info": "bright_black",
                "logging.level.warning": "bright_black",
                "logging.level.error": "bright_red",
            }
        ),
        highlight=False,
    )
    if verbose:
        logging.basicConfig(
            format=log_fmt,
            level=logging.DEBUG,
            datefmt="%m-%d %H:%M:%S",
            handlers=[RichHandler(show_time=False, show_path=False, console=console)],
        )
    else:
        logging.basicConfig(
            format=log_fmt,
            level=logging.INFO,
            datefmt="%m-%d %H:%M:%S",
            handlers=[RichHandler(show_time=False, show_path=False, console=console)],
        )


if __name__ == "__main__":
    app()
