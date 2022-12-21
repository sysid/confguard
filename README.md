# confguard

Project can be re-guarded any time, potential changes will be updated if necessary.
When the targets in .confguard change re-run the guarding process.

[![PyPI Version][pypi-image]][pypi-url]
[![Build Status][build-image]][build-url]
[![Code Coverage][coverage-image]][coverage-url]

> Save configuration files outside project in save place


Quickstart
==========

confguard is available on PyPI and can be installed with `pip <https://pip.pypa.io>`_.

.. code-block:: console

    $ pip install confguard

After installing confguard you can use it like any other Python module.

Here is a simple example:

.. code-block:: python

    import confguard
    # Fill this section in with the common use-case.

The `API Reference <http://confguard.readthedocs.io>`_ provides API-level documentation.


## Changelog
[CHANGELOG.md](https://github.com/sysid/playbook/blob/master/CHANGELOG.md)

## Scratch
for f in *; do cp --remove-destination source/$f $f; done
find ./ -type l -print0|xargs -0 -n1 -i sh -c 'cp --remove-destination $(readlink "{}") "{}" '
for f in *; do cp --remove-destination $(readlink "$f") "$f"; done

find ./ -not -path './.venv/*' -not -path './.git/*' -type l -print0|xargs -0 -i sh -c 'echo $(readlink "{}") "{}" '



<!-- Badges -->

[pypi-image]: https://badge.fury.io/py/confguard.svg
[pypi-url]: https://pypi.org/project/confguard/
[build-image]: https://github.com/sysid/confguard/actions/workflows/build.yml/badge.svg
[build-url]: https://github.com/sysid/confguard/actions/workflows/build.yml
[coverage-image]: https://codecov.io/gh/sysid/confguard/branch/master/graph/badge.svg
[coverage-url]: https://codecov.io/gh/sysid/confguard
