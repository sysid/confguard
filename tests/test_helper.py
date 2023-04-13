import logging
from pathlib import Path

import pytest

# noinspection PyProtectedMember
from confguard.helper import (
    _create_relative_path,
    denormalize_name,
    denormalize_path,
    deserialize_from_base64,
    normalize_name,
    normalize_path,
    serialize_to_base64,
)

_log = logging.getLogger(__name__)


@pytest.mark.parametrize(
    ("source", "target", "expected"),
    (
        ("/c/b/a/xxx/.envrc", "/c/y/xxx-123/.envrc", "../../../y/xxx-123/.envrc"),
        (
            "/c/b/a/xxx/.envrc",
            "/tmp/y/xxx-123/.envrc",
            "../../../../tmp/y/xxx-123/.envrc",
        ),
        ("/c/xxx/.envrc", "/c/y/xxx-123/.envrc", "../y/xxx-123/.envrc"),
    ),
)
def test_find_relative_path(source, target, expected):
    _log.info(f"{source=} {target=} {expected=}")

    if not (Path(source).is_absolute() and Path(target).is_absolute()):
        raise ValueError("Both source and target must be absolute paths")

    new_sp = []
    new_tp = []
    name = Path(source).name
    source_parts = Path(source).parent.parts
    target_parts = Path(target).parent.parts
    for i, sp in enumerate(source_parts):
        if sp == target_parts[i]:  # remove common leading parts
            continue
        else:
            _ = source_parts[:i]
            new_sp = source_parts[i:]
            new_tp = target_parts[i:]
            break

    rel_path = [".."] * len(new_sp) + list(new_tp) + [name]
    rel_path = Path(*rel_path)
    assert rel_path == Path(expected)
    _ = None


@pytest.mark.parametrize(
    ("source", "target", "expected"),
    (
        ("/c/b/a/xxx/.envrc", "/c/y/xxx-123/.envrc", "../../../y/xxx-123/.envrc"),
        (
            "/c/b/a/xxx/.envrc",
            "/tmp/y/xxx-123/.envrc",
            "../../../../tmp/y/xxx-123/.envrc",
        ),
        ("/c/xxx/.envrc", "/c/y/xxx-123/.envrc", "../y/xxx-123/.envrc"),
    ),
)
def test_find_relative_path_builtin(source, target, expected):
    rel_path = _create_relative_path(source, target)
    assert Path(rel_path) == Path(expected)


FILES = [
    "file1____________________________________________________________",
    "file2____________________________________________________________",
    "file3____________________________________________________________",
    "file4____________________________________________________________",
    "file5____________________________________________________________",
    "file1____________________________________________________________",
    "file2____________________________________________________________",
    "file3____________________________________________________________",
    "file4____________________________________________________________",
    "file5____________________________________________________________",
]


def test_serialize_to_base64():
    # Serialize the list to a base64-encoded string
    serialized = serialize_to_base64(FILES)

    # Print the serialized string
    print(f"\n{serialized}")
    assert isinstance(serialized, str)


def test_deserialize_to_base64():
    serialized = """
gASVYwEAAAAAAABdlCiMQWZpbGUxX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19f
X19fX19fX19fX19fX19fX19fX19flIxBZmlsZTJfX19fX19fX19fX19fX19fX19fX19fX19fX19fX19f
X19fX19fX19fX19fX19fX19fX19fX19fX19fX1+UjEFmaWxlM19fX19fX19fX19fX19fX19fX19fX19f
X19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX5SMQWZpbGU0X19fX19fX19fX19fX19f
X19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19flIxBZmlsZTVfX19fX19f
X19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX19fX1+UaAFoAmgD
aARoBWUu
    """
    obj = deserialize_from_base64(serialized)
    print(f"\n{obj}")
    assert obj == FILES


def test_normalize_name():
    assert normalize_name(".xxx") == "dot.xxx"
    assert normalize_name("xxx") == "xxx"


def test_denormalize_name():
    assert denormalize_name("dot.xxx") == ".xxx"
    assert denormalize_name("xxx") == "xxx"


@pytest.mark.parametrize(
    ("path", "expected"),
    (
        ["xxx/xxx.txt", "xxx/xxx.txt"],
        [".run", "dot.run"],
        [".run/.env", "dot.run/dot.env"],
        ["./.run/.env", "./dot.run/dot.env"],
    ),
)
def test_normalize_path(path, expected):
    assert normalize_path(Path(path)) == Path(expected)


@pytest.mark.parametrize(
    ("path", "expected"),
    (
        ["xxx/xxx.txt", "xxx/xxx.txt"],
        ["dot.run", ".run"],
        ["dot.run/dot.env", ".run/.env"],
        ["./dot.run/dot.env", "./.run/.env"],
    ),
)
def test_denormalize_path(path, expected):
    assert denormalize_path(Path(path)) == Path(expected)
