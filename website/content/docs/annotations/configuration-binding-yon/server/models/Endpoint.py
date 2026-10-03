from typing import Annotated
import json
import os

class Endpoint:
    profile: Annotated[str, setting("${yon.active.profile:development}")] = ""
