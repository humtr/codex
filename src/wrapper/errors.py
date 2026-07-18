"""Exception hierarchy for internal Codex Termux helpers."""


class CodexTermuxError(Exception):
    """Base class for internal wrapper errors."""


class SchemaError(CodexTermuxError):
    """Raised when structured wrapper data fails validation."""


class IntegrityError(CodexTermuxError):
    """Raised when an artifact or metadata integrity check fails."""


class TransactionError(CodexTermuxError):
    """Raised when an atomic wrapper operation cannot be completed."""


class CollisionError(CodexTermuxError):
    """Raised when immutable artifact identity collides with content."""
