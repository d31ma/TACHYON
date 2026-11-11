from dataclasses import dataclass


@dataclass
class ClaimConflict:
    reason: str
    holder: str
