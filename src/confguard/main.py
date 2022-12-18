import logging
from pathlib import Path

import typer

_log = logging.getLogger(__name__)
app = typer.Typer()


@app.command()
def guard(
    file: Path = typer.Argument(default=None, help="Filename", exists=False),
    verbose: bool = typer.Option(False, "-v", "--verbose", help="verbosity"),
):
    selection: list = input().split()
    if verbose:
        typer.echo(f"{selection=}")
    typer.echo(f"Hello {file}")


@app.command()
def goodbye(name: str, formal: bool = False):
    if formal:
        typer.echo(f"Goodbye Ms. {name}. Have a good day.")
    else:
        typer.echo(f"Bye {name}!")


if __name__ == "__main__":
    log_fmt = r'%(asctime)-15s %(levelname)s %(name)s %(funcName)s:%(lineno)d %(message)s'
    logging.basicConfig(format=log_fmt, level=logging.DEBUG, datefmt='%m-%d %H:%M:%S')
    app()
