@Service
class AttachService {
    private val files = Files()
    private var selected = ""
    var name: String = ""
    var digest: String = ""
    var status: String = ""
    var problem: String = ""

    fun choose() {
        files.pick(".pdf,image/*", false)
    }

    fun upload() {
        files.upload(selected, "PUT", "https://uploads.example.com/inbox", "content-type: application/octet-stream")
    }

    @OnEvent
    fun picked(picked: FilePicked) {
        name = "${picked.name}, ${picked.size} bytes"
        digest = picked.sha256
        selected = picked.id
    }

    @OnEvent
    fun finished(finished: UploadFinished) {
        status = "HTTP ${finished.status} ${finished.error}"
    }
}
