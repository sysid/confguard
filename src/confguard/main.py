import logging
from pathlib import Path

import typer

from confguard.environment import CONFGUARD_BKP_DIR, config
from confguard.services import Files, Links, Sentinel

_log = logging.getLogger(__name__)
app = typer.Typer(help="Save sensitive configuration in a save place")


def _load_confguard_config(sentinel):
    try:
        sentinel.load_confguard()
        _log.debug(config.confguard)
    except Exception as e:
        typer.secho(f"Error loading configuration: {e}", fg=typer.colors.RED)
        raise typer.Exit(1)
    cfg = config.confguard.get("config")
    if cfg is None:
        typer.secho("Invalid config, check '.confguard' format. (config section)", fg=typer.colors.RED)
        raise typer.Exit(1)
    targets = cfg.get("targets")
    if targets is None:
        typer.secho("Invalid config, check '.confguard' format. (no targets)", fg=typer.colors.RED)
        raise typer.Exit(1)
    return targets


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
    _guard(source_dir)


def _guard(source_dir: Path) -> None:
    sentinel = Sentinel(source_dir=source_dir)
    targets = _load_confguard_config(sentinel)

    if config.sentinel is not None:
        typer.secho(
            f"Project is already guarded: {config.sentinel=}, unguard first.",
            fg=typer.colors.GREEN,
        )
        raise typer.Exit(1)

    _log.info(f"Guarding {source_dir=}")

    sentinel.create()
    bkp_dir = source_dir / CONFGUARD_BKP_DIR
    target_dir = config.confguard_path / config.sentinel

    files = Files(
        rel_target_dir=config.sentinel, source_dir=source_dir, targets=targets
    )
    try:
        files.create_bkp(source_dir=source_dir, bkp_dir=bkp_dir)
    except Exception as e:
        typer.secho(f"Error occurred, Aborting: {e}", fg=typer.colors.RED)
        files.delete_dir(dir_=bkp_dir)
        sentinel.remove()
        raise typer.Exit(1)

    lks = Links(source_dir=source_dir, target_dir=target_dir, targets=targets)
    try:
        files.move_files(source_dir=source_dir, target_dir=target_dir)
        lks.create()
        lks.back_create()
    except Exception as e:
        typer.secho(f"Error occurred, rolling back: {e}", fg=typer.colors.RED)
        lks.remove()
        lks.back_remove()
        files.restore_bkp(source_dir=source_dir, bkp_dir=bkp_dir)
        raise typer.Exit(1)
    finally:
        files.delete_dir(dir_=bkp_dir)


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
    _unguard(source_dir)


def _unguard(source_dir: Path) -> None:
    sentinel = Sentinel(source_dir=source_dir)
    targets = _load_confguard_config(sentinel)

    if config.sentinel is None:
        typer.secho(
            f"Project is un-guarded: {config.sentinel=}, nothing to do.",
            fg=typer.colors.GREEN,
        )
        raise typer.Exit(1)
    _log.info(f"Un-guarding {source_dir=}")

    _sentinel = config.sentinel  # save sentinel for rollback (TODO)

    target_dir = config.confguard_path / config.sentinel
    bkp_dir = config.confguard_path / config.sentinel / CONFGUARD_BKP_DIR

    files = Files(
        rel_target_dir=config.sentinel, source_dir=source_dir, targets=targets
    )
    try:
        files.create_bkp(source_dir=target_dir, bkp_dir=bkp_dir)
    except Exception as e:
        typer.secho(f"Error occurred, Aborting: {e}", fg=typer.colors.RED)
        files.delete_dir(dir_=bkp_dir)
        Sentinel(source_dir=source_dir).remove()
        raise typer.Exit(1)

    lks = Links(source_dir=source_dir, target_dir=target_dir, targets=targets)
    try:
        lks.remove()
        lks.back_remove()
        files.return_files(source_dir=source_dir, target_dir=target_dir)
        sentinel.remove()  # TODO: tx safety (should be recreated if rollback)
    except Exception as e:
        typer.secho(f"Error occurred, rolling back: {e}", fg=typer.colors.RED)
        files.restore_bkp(source_dir=target_dir, bkp_dir=bkp_dir)
        typer.secho(f"Restoring links.")
        lks.create()
        lks.back_create()
        raise typer.Exit(1)
    finally:
        files.delete_dir(dir_=bkp_dir)


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
