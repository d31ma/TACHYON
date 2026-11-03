namespace tachyon::yon {

class YonText {
public:
    static constexpr const char* REPLACEMENT = "\xEF\xBF\xBD";

    static bool valid(const std::string& text) {
        std::size_t at = 0;
        while (at < text.size()) {
            std::size_t length = sequence(text, at);
            if (length == 0) {
                return false;
            }
            at += length;
        }
        return true;
    }

    static std::string repaired(const std::string& text) {
        if (valid(text)) {
            return text;
        }
        std::string kept;
        std::size_t at = 0;
        while (at < text.size()) {
            std::size_t length = sequence(text, at);
            if (length == 0) {
                kept += REPLACEMENT;
                at += 1;
                continue;
            }
            kept.append(text, at, length);
            at += length;
        }
        return kept;
    }

    static std::string base64(const std::string& bytes) {
        static const char ALPHABET[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        std::string written;
        std::size_t at = 0;
        while (at + 2 < bytes.size()) {
            unsigned group = (byte(bytes, at) << 16) | (byte(bytes, at + 1) << 8) | byte(bytes, at + 2);
            written += ALPHABET[(group >> 18) & 63];
            written += ALPHABET[(group >> 12) & 63];
            written += ALPHABET[(group >> 6) & 63];
            written += ALPHABET[group & 63];
            at += 3;
        }
        std::size_t left = bytes.size() - at;
        if (left > 0) {
            unsigned group = byte(bytes, at) << 16;
            if (left == 2) {
                group |= byte(bytes, at + 1) << 8;
            }
            written += ALPHABET[(group >> 18) & 63];
            written += ALPHABET[(group >> 12) & 63];
            written += left == 2 ? ALPHABET[(group >> 6) & 63] : '=';
            written += '=';
        }
        return written;
    }

private:
    static unsigned byte(const std::string& text, std::size_t at) {
        return static_cast<unsigned char>(text[at]);
    }

    static bool follows(const std::string& text, std::size_t at) {
        return at < text.size() && (byte(text, at) & 0xC0) == 0x80;
    }

    static std::size_t sequence(const std::string& text, std::size_t at) {
        unsigned first = byte(text, at);
        if (first < 0x80) {
            return 1;
        }
        std::size_t length = first >= 0xF0 ? 4 : first >= 0xE0 ? 3 : first >= 0xC2 ? 2 : 0;
        if (length == 0 || first > 0xF4) {
            return 0;
        }
        for (std::size_t next = 1; next < length; next += 1) {
            if (!follows(text, at + next)) {
                return 0;
            }
        }
        unsigned second = byte(text, at + 1);
        bool overlong = (first == 0xE0 && second < 0xA0) || (first == 0xF0 && second < 0x90);
        bool surrogate = first == 0xED && second >= 0xA0;
        bool tooLarge = first == 0xF4 && second >= 0x90;
        return overlong || surrogate || tooLarge ? 0 : length;
    }
};

}
