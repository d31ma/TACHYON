from typing import Annotated


class PublicValues:
    title: Annotated[str, setting("${tac.catalog.title:Catalog}")] = ''
