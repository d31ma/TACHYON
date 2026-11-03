from typing import Annotated

@service
class EventService:
    _COLLECTION = "hd-events"
    _DEPLOYS = {"kind": "deploy"}
    _CHANGE = {"count": 7}
    _PAGE_SIZE = 2
    _TOPIC = "app.render"
    _GROUP = "renderers"
    data_source: Annotated[DataSource, needs]

    def report(self) -> dict[str, object]:
        collection = self._COLLECTION
        self.data_source.create_collection(collection)
        event_id = self.data_source.insert(collection, self._DEPLOYS)["id"]
        self.data_source.insert(collection, self._DEPLOYS)
        self.data_source.insert(collection, self._DEPLOYS)
        found = self.data_source.find_by_id(collection, event_id)
        patched = self.data_source.patch(collection, event_id, self._CHANGE)
        scrolled = self._count_pages()
        job = self._run_job()
        self.data_source.delete_by_id(collection, event_id)
        gone = self.data_source.find_by_id(collection, event_id) is None
        dropped = self.data_source.drop_collection(collection)
        return {"found": found["kind"], "patched": patched["count"], **scrolled, "job": job, "gone": gone, "dropped": dropped}

    def _count_pages(self) -> dict[str, int]:
        pages, seen, cursor, more = 0, 0, None, True
        while more:
            window = self.data_source.scroll(self._COLLECTION, self._DEPLOYS, cursor, self._PAGE_SIZE)
            pages += 1
            seen += len(window["content"])
            cursor = window["nextCursor"]
            more = cursor is not None
        return {"pages": pages, "seen": seen}

    def _run_job(self) -> dict[str, object]:
        topic = self._TOPIC
        self.data_source.publish(topic, {"app": "web"}, "render:web")
        delivery = self.data_source.claim(topic, self._GROUP, 1, 30000)[0]
        self.data_source.ack(topic, self._GROUP, delivery)
        return delivery["payload"]
