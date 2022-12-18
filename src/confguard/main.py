import logging
import tomllib
import uuid
from datetime import datetime
from pathlib import Path

import typer

from confguard.environment import CONFIG_TEMPLATE, config, ROOT_DIR

_log = logging.getLogger(__name__)
app = typer.Typer(help="Save sensitive configuration in a save place")


@app.command()
def configure():
    typer.edit(CONFIG_TEMPLATE, filename=str(config.config_path))  # not working with pytest
    typer.echo(f"Config file is: {config.config_path}\n")
    with config.config_path.open("r") as f:
        print(f.read())


def create_sentinel(msg: str) -> str:
    cwd = Path.cwd()
    try:
        p = cwd.parts[-1]   # get proj dir as part of sentinel filename
    except IndexError:
        p = "unknown-dir"

    sentinel = list(cwd.glob(".confguard-*"))
    if len(sentinel) > 0:
        _log.debug(f"Found sentinel: {sentinel}")
        return sentinel[0].name.split(".")[1]

    name = f"{p}-{uuid.uuid4().hex}"
    sentinel = f".{name}.{config.app_name}"
    with Path(sentinel).open("w") as f:
        msg = f"Created by {config.app_name}. DO NOT REMOVE.\n{datetime.utcnow()}"
        print(msg, file=f)
    _log.debug(f"Created sentinel: {name}")
    return name


def move_files(targets: list[str]):
    print(targets)


@app.command()
def guard(
    what: str = typer.Argument(default="default", help="files configuration"),
    verbose: bool = typer.Option(False, "-v", "--verbose", help="verbosity"),
):
    """
    must run in project directory where the config files are located.

    Create guarded config:
    1. move files to save location/directory
    2. create sentinel representation in local directory
    """
    name = create_sentinel(what)
    targets = config.confguard.get(what)
    if targets is None:
        typer.echo(f"Unknown target: {what}, Must be one of {config.confguard.keys()}")
        raise typer.Exit(1)
    move_files(targets.get("targets"))
    # create_links()

@app.command()
def check_source(source: str, verbose: bool = False):
    """
    Check the source:
    1. Exists
    """


if __name__ == "__main__":
    log_fmt = r'%(asctime)-15s %(levelname)s %(name)s %(funcName)s:%(lineno)d %(message)s'
    logging.basicConfig(format=log_fmt, level=logging.DEBUG, datefmt='%m-%d %H:%M:%S')
    app()
