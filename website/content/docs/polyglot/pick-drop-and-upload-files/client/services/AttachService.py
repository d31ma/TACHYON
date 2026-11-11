from typing import Annotated


@service
class AttachService:
    files: Annotated[Files, needs]
    selected: str = ''
    name: str = ''
    digest: str = ''
    status: str = ''
    problem: str = ''

    def choose(self) -> None:
        self.files.pick('.pdf,image/*', False)

    def upload(self) -> None:
        try:
            self.files.upload(self.selected, 'PUT', 'https://uploads.example.com/inbox', 'content-type: application/octet-stream')
        except RuntimeError as refusal:
            self.problem = str(refusal)

    @on_event
    def picked(self, picked: FilePicked) -> None:
        self.name = f'{picked.name}, {picked.size} bytes'
        self.digest = picked.sha256
        self.selected = picked.id

    @on_event
    def finished(self, finished: UploadFinished) -> None:
        self.status = f'HTTP {finished.status} {finished.error}'
