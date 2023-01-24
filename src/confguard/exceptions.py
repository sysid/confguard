class ConfGuardError(Exception):
    """A base class for MyProject exceptions."""


class BackupExistError(ConfGuardError):
    """A custom exception class for MyProject."""


class DirectoryNotDeleted(ConfGuardError):
    """A custom exception class for MyProject."""


class InvalidConfigError(ConfGuardError):
    """A custom exception class for MyProject."""
