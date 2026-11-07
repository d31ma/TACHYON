#pragma once
#include <stdexcept>
#include <string>

class [[Service]] ReportService {
public:
    std::string problem;
    [[Needs]] WindowControls& windowControls;

    void minimize() {
        try {
            windowControls.minimize();
        } catch (const std::runtime_error& refusal) {
            problem = refusal.what();
        }
    }

    void resize() {
        try {
            windowControls.resize(1280, 800);
        } catch (const std::runtime_error& refusal) {
            problem = refusal.what();
        }
    }

    void rename() {
        try {
            windowControls.setTitle("Quarterly report");
        } catch (const std::runtime_error& refusal) {
            problem = refusal.what();
        }
    }
};
