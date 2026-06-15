"""Exception hierarchy for internal Codex native helpers."""


class CodexNativeError(Exception):
    """Base class for internal wrapper errors."""


class SchemaError(CodexNativeError):
    """Raised when structured wrapper data fails validation."""


class IntegrityError(CodexNativeError):
    """Raised when an artifact or metadata integrity check fails."""


class TransactionError(CodexNativeError):
    """Raised when an atomic wrapper operation cannot be completed."""


class CollisionError(CodexNativeError):
    """Raised when immutable artifact identity collides with content."""
