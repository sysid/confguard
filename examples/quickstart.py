"""
This example script imports the confguard package and
prints out the version.
"""

import confguard


def main():
    print(
        f"confguard version: {confguard.__version__}"
    )


if __name__ == "__main__":
    main()
