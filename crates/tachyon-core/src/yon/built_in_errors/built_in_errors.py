

class BrokenRule:
    def __init__(self, value):
        self._field = str(value.get("field") or "")
        self._rule = str(value.get("rule") or "")

    @property
    def field(self):
        return self._field

    @property
    def rule(self):
        return self._rule


class YonBuiltInError(Exception):
    STATUS = 500

    def __init__(self, value):
        super().__init__(type(self).__name__)
        self._reference = str(value.get("reference") or "")

    @property
    def reference(self):
        return self._reference

    @property
    def status(self):
        return type(self).STATUS


class NotSignedIn(YonBuiltInError):
    STATUS = 401


class RoleMissing(YonBuiltInError):
    STATUS = 403


class IdentityCheckFailed(YonBuiltInError):
    STATUS = 500


class RequestRejected(YonBuiltInError):
    STATUS = 400

    def __init__(self, value):
        super().__init__(value)
        self._part = str(value.get("part") or "")
        self._field = str(value.get("field") or "")
        self._message = str(value.get("message") or "")

    @property
    def part(self):
        return self._part

    @property
    def field(self):
        return self._field

    @property
    def message(self):
        return self._message


class BodyTooLarge(YonBuiltInError):
    STATUS = 413

    def __init__(self, value):
        super().__init__(value)
        self._limit_bytes = int(value.get("limitBytes") or 0)

    @property
    def limit_bytes(self):
        return self._limit_bytes


class CheckRejected(YonBuiltInError):
    STATUS = 422

    def __init__(self, value):
        super().__init__(value)
        entries = value.get("violations")
        self._violations = [BrokenRule(entry) for entry in entries] if isinstance(entries, list) else []
        self._truncated = value.get("truncated") is True

    @property
    def violations(self):
        return list(self._violations)

    @property
    def truncated(self):
        return self._truncated


class UnexpectedError(YonBuiltInError):
    STATUS = 500


def __yon_built_in_error__(value):
    types = {
        "NotSignedIn": NotSignedIn,
        "RoleMissing": RoleMissing,
        "IdentityCheckFailed": IdentityCheckFailed,
        "RequestRejected": RequestRejected,
        "BodyTooLarge": BodyTooLarge,
        "CheckRejected": CheckRejected,
        "UnexpectedError": UnexpectedError,
    }
    kind = types.get(value.get("type")) if isinstance(value, dict) else None
    if kind is None:
        raise TypeError("Unknown built-in error.")
    return kind(value)
