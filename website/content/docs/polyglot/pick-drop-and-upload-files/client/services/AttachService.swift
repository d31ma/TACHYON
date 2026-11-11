@Service
final class AttachService {
    private let files = Files()
    private var selected = ""
    var name: String = ""
    var digest: String = ""
    var status: String = ""
    var problem: String = ""

    func choose() throws {
        try files.pick(accept: ".pdf,image/*", multiple: false)
    }

    func upload() throws {
        _ = try files.upload(id: selected, method: "PUT", url: "https://uploads.example.com/inbox", headers: "content-type: application/octet-stream")
    }

    @OnEvent
    func picked(_ picked: FilePicked) {
        name = "\(picked.name), \(picked.size) bytes"
        digest = picked.sha256
        selected = picked.id
    }

    @OnEvent
    func finished(_ finished: UploadFinished) {
        status = "HTTP \(finished.status) \(finished.error)"
    }
}
