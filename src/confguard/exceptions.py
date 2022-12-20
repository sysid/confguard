class ConfGuardError(Exception):
    """A base class for MyProject exceptions."""


class BackupExistError(ConfGuardError):
    """A custom exception class for MyProject."""


class BackupNotDeleted(ConfGuardError):
    """A custom exception class for MyProject."""


class FileDoesNotExistError(ConfGuardError):
    """A custom exception class for MyProject."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args)
        self.target = kwargs.get("target")
