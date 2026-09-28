from dataclasses import dataclass


@dataclass
class Configuration:
    pageSize: int
    port: int
