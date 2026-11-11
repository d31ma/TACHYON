#pragma once
#include <string>

class [[Service]] AttachService {
public:
    [[Needs]] Files& files;
    std::string selected;
    std::string name;
    std::string digest;
    std::string status;
    std::string problem;

    void choose() { files.pick(".pdf,image/*", false); }

    void upload() { files.upload(selected, "PUT", "https://uploads.example.com/inbox", "content-type: application/octet-stream"); }

    [[OnEvent]] void picked(const FilePicked& picked) {
        name = picked.name + ", " + std::to_string(static_cast<long long>(picked.size)) + " bytes";
        digest = picked.sha256;
        selected = picked.id;
    }

    [[OnEvent]] void finished(const UploadFinished& finished) {
        status = "HTTP " + std::to_string(static_cast<int>(finished.status)) + " " + finished.error;
    }
};
