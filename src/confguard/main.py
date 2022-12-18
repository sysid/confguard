import logging
import os
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
        p = cwd.parts[-1]  # get proj dir as part of sentinel filename
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


def move_files(name: str, targets: list[str]) -> list[str]:
    target_path = config.confguard_path / name
    Path(target_path).mkdir(parents=True, exist_ok=True)
    target_locations = []

    for t in targets:
        p = Path(t)
        if p.exists():
            _log.debug(f"Moving {p} to {target_path}")
            p.rename(target_path / p.name)
            target_locations.append(str(target_path / p.name))
        else:
            _log.debug(f"File {p} does not exist")
    return target_locations


def _create_relative_path(source: str, target: str) -> Path:
    source_path = Path(source).parent
    target_path = Path(target).parent
    name = Path(source).name
    rel_path = os.path.relpath(target_path, source_path)
    return Path(rel_path) / name


def create_links(target_locations: list[str], is_relative: bool = False) -> list[str]:
    links = []
    for t in target_locations:
        p = Path(t)
        link = Path.cwd() / p.name
        p = _create_relative_path(str(link), str(p))
        _log.debug(f"Creating link {link} to {p}")
        link.symlink_to(p)
        links.append(str(p))
        _ = None

    _log.debug(f"{links=}")
    return links


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
    target_locations = move_files(name=name, targets=targets.get("targets"))
    create_links(target_locations)


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
