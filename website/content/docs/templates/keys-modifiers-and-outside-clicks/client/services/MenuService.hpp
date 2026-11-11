#pragma once

class [[Service]] MenuService {
public:
    bool expanded = false;

    void toggle() {
        expanded = !expanded;
    }

    void close() {
        expanded = false;
    }
};
