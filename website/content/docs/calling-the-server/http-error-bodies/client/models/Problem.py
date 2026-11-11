from dataclasses import dataclass

from models import ProblemEntry


@dataclass
class Problem:
    title: str
    status: int
    errors: list[ProblemEntry]
