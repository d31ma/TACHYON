@service
@needs_all
class LanguagesService:
    entries: object

    def handlers(self) -> list[object]:
        return self._files_of(self._list_value(self.entries))

    def _files_of(self, entries: list[object]) -> list[object]:
        files = []
        for entry in entries:
            files.extend(self._entry_files(entry))
        return files

    def _entry_files(self, entry: object) -> list[object]:
        return self._list_value(self._value(self._map_value(entry), "files"))

    def _map_value(self, value: object) -> dict[str, object]:
        if isinstance(value, dict):
            return value
        return {}

    def _value(self, value: dict[str, object], key: str) -> object:
        return value.get(key)

    def _list_value(self, value: object) -> list[object]:
        if isinstance(value, list):
            return value
        return []
