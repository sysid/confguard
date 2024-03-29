import base64
import os
import pickle
import textwrap
from pathlib import Path
from typing import Any


def serialize_to_base64(obj: Any, line_length=80) -> str:
    # Serialize the object to a bytes object using pickle
    serialized = pickle.dumps(obj)
    # Encode the bytes object to a base64-encoded string
    encoded = base64.b64encode(serialized).decode("utf-8")
    # Wrap the base64-encoded string every `line_length` characters
    wrapped = textwrap.fill(encoded, width=line_length)
    return wrapped


def deserialize_from_base64(base64_str: str) -> Any:
    # Decode the base64-encoded string to a bytes object
    decoded = base64.b64decode(base64_str)
    # Deserialize the bytes object to a Python object using pickle
    obj = pickle.loads(decoded)
    return obj


def is_directory_containing_only(directory_path, allowed_names) -> bool:
    for item in Path(directory_path).iterdir():
        if item.is_file() and item.name not in allowed_names:
            return False
        elif item.is_dir():
            if is_directory_empty(item):
                return True
            if item.name not in allowed_names:
                return False
    return True


def is_directory_empty(directory_path) -> bool:
    return not any(Path(directory_path).iterdir())


def _create_relative_path(source: str, target: str) -> Path:
    source_path = Path(source).parent
    target_path = Path(target).parent

    if not (source_path.is_absolute() and target_path.is_absolute()):
        raise ValueError("Both source and target must be absolute paths")

    name = Path(
        target
    ).name  # Gotcha: source_path.name is not the same as target_path.name
    rel_path = os.path.relpath(target_path, source_path)
    return Path(rel_path) / name


def normalize_name(name: str) -> str:
    if name.startswith("."):
        return f"dot.{name[1:]}"
    return name


def denormalize_name(name: str) -> str:
    if name.startswith("dot."):
        return f".{name[4:]}"
    return name


def normalize_path(path: Path) -> Path:
    return Path(*[normalize_name(p) for p in path.parts])


def denormalize_path(path: Path) -> Path:
    return Path(*[denormalize_name(p) for p in path.parts])


if __name__ == "__main__":
    # Create a list of strings
    files = [
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

    # Serialize the list to a base64-encoded string
    serialized = serialize_to_base64(files)

    # Print the serialized string
    print(serialized)

    obj = deserialize_from_base64(serialized)
    print(obj)
