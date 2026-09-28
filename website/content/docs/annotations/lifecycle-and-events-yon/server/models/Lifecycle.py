from dataclasses import dataclass


@dataclass
class Lifecycle:
    log: list[str]
    settled: int
    audited: int
