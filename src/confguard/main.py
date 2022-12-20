import logging
from pathlib import Path

import typer

from confguard.environment import CONFIG_TEMPLATE, config
from confguard.services import Sentinel, Files, Links

_log = logging.getLogger(__name__)
app = typer.Typer(help="Save sensitive configuration in a save place")


@app.command()
def configure():
    typer.edit(
        CONFIG_TEMPLATE, filename=str(config.config_path)
    )  # not working with pytest
    typer.echo(f"Config file is: {config.config_path}\n")
    with config.config_path.open("r") as f:
        print(f.read())


@app.command()
def guard(
    what: str = typer.Argument(default="default", help="files configuration"),
    verbose: bool = typer.Option(False, "-v", "--verbose", help="verbosity"),
):
    """
    must run in project directory where the config files are located.
    relative linking cannot span mounts, absolute linking can

    Create guarded config:
    1. move files to save location/directory
    2. create sentinel representation in local directory
    """
    _guard(what)


def _guard() -> None:
    cfg = config.confguard.get("config")
    if cfg is None:
        typer.secho("Invalid config, check '.confguard' format.", fg=typer.colors.RED)
        return
    targets = cfg.get("targets")
    if cfg is None:
        typer.secho("Invalid config, check '.confguard' format.", fg=typer.colors.RED)
        return

    # backup as tx prerequisite
    Sentinel.create()
    files = Files(rel_target_dir=config.sentinel, source_dir=Path.cwd(), targets=targets)
    try:
        files.create_bkp()
    except Exception as e:
        typer.secho(f"Error occurred, Aborting: {e}", fg=typer.colors.RED)
        files.delete_bkp_dir()
        Sentinel.remove()
        raise typer.Exit(1)

    lks = Links(source_locations=files.source_locations, target_locations=files.target_locations)
    try:
        files.move_files()
        lks.create()
        lks.back_create()
    except Exception as e:
        typer.secho(f"Error occurred, rolling back: {e}", fg=typer.colors.RED)
        lks.remove()
        lks.back_remove()
        files.restore_bkp()
        raise typer.Exit(1)
    finally:
        files.delete_bkp_dir()


@app.command()
def check_source(source: str, verbose: bool = False):
    """
    Check the source:
    1. Exists
    """


if __name__ == "__main__":
    log_fmt = (
        r"%(asctime)-15s %(levelname)s %(name)s %(funcName)s:%(lineno)d %(message)s"
    )
    logging.basicConfig(format=log_fmt, level=logging.DEBUG, datefmt="%m-%d %H:%M:%S")
    app()
