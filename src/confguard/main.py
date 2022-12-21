import logging
from pathlib import Path

import typer

from adapter import TomlRepoConfGuard
from confguard.environment import CONFGUARD_BKP_DIR, config, CONFGUARD_CONFIG_FILE
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
    """ Guards a directory.
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
    _ = _guard(source_dir)


def _guard(source_dir: Path) -> ConfGuard:
    repo = TomlRepoConfGuard(source_dir=source_dir)
    try:
        cg = repo.get()
    except InvalidConfigError as e:
        typer.secho(str(e), fg=typer.colors.RED, err=True)
        raise typer.Exit(1)

    if cg.sentinel is not None:
        typer.secho(
            f"Project is already guarded: {config.sentinel=}, unguard first.",
            fg=typer.colors.GREEN,
        )
        raise typer.Exit(1)

    _log.info(f"Guarding {source_dir=}")

    cg.create_sentinel()
    try:
        cg.create_bkp(cg.source_dir)
    except Exception as e:
        typer.secho(f"Error occurred, Aborting: {e}", fg=typer.colors.RED)
        cg.delete_dir(dir_=cg.source_dir / CONFGUARD_BKP_DIR)
        cg.remove_sentinel()
        repo.add(cg)  # save it
        raise typer.Exit(1)

    try:
        cg.move_files()
        cg.create_lk()
        cg.back_create()
    except Exception as e:
        typer.secho(f"Error occurred, rolling back: {e}", fg=typer.colors.RED)
        cg.remove_lk()
        cg.back_remove()
        cg.restore_bkp(cg.source_dir)
        cg.remove_sentinel()
        raise typer.Exit(1)
    finally:
        repo.add(cg)  # save it
        cg.delete_dir(dir_=cg.source_dir / CONFGUARD_BKP_DIR)
    return cg


@app.command()
def unguard(
    # path argument
    source_dir: Path = typer.Argument(
        ..., help="Path to the directory to guard", exists=True
    ),
):
    """ Un-guards a directory.
    Revert changes made by `guard`.
    """
    source_dir = Path(source_dir).expanduser().resolve()
    _ = _unguard(source_dir)


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

    _log.info(f"Un-guarding {source_dir=}")

    try:
        cg.create_bkp(cg.target_dir)
    except Exception as e:
        typer.secho(f"Error occurred, Aborting: {e}", fg=typer.colors.RED)
        cg.delete_dir(dir_=cg.target_dir / CONFGUARD_BKP_DIR)
        cg.remove_sentinel()
        repo.add(cg)  # save it
        raise typer.Exit(1)

    try:
        cg.remove_lk()
        cg.back_remove()
        cg.unmove_files()
        cg.remove_sentinel()
    except Exception as e:
        typer.secho(f"Error occurred, rolling back: {e}", fg=typer.colors.RED)
        cg.restore_bkp(cg.target_dir)
        typer.secho(f"Restoring links.")
        cg.create_lk()
        cg.back_create()
        raise typer.Exit(1)
    finally:
        repo.add(cg)  # save it
        cg.delete_dir(dir_=cg.target_dir / CONFGUARD_BKP_DIR)
    return cg


@app.command()
def check_source(
    source: str,
    verbose: bool = typer.Option(False, "-v", "--verbose", help="verbosity"),
):
    """
    Check the source:
    1. Exists
    """
    _log.info("aaaaa")
    _log.debug("xxxxx")

    # add params for all commands here:


@app.callback()
def main(
    ctx: typer.Context,
    verbose: bool = typer.Option(False, "-v", "--verbose", help="verbosity"),
):
    log_fmt = r"%(asctime)-15s %(levelname)-7s %(message)s"
    if verbose:
        logging.basicConfig(
            format=log_fmt, level=logging.DEBUG, datefmt="%m-%d %H:%M:%S"
        )
    else:
        logging.basicConfig(
            format=log_fmt, level=logging.INFO, datefmt="%m-%d %H:%M:%S"
        )


if __name__ == "__main__":
    app()
