extern "C" {

__attribute__((export_name("tac_wasm_alloc"))) void* tac_wasm_alloc(int length) {
    return std::malloc(length > 0 ? static_cast<std::size_t>(length) : 1);
}

__attribute__((export_name("tac_wasm_dealloc"))) void tac_wasm_dealloc(void* pointer, int) {
    std::free(pointer);
}

__attribute__((export_name("tac_native_invoke"))) char* tac_native_invoke(const char* text) {
    std::string answer = tachyon::TacPage::invoke(std::string(text));
    char* copy = static_cast<char*>(std::malloc(answer.size() + 1));
    std::memcpy(copy, answer.c_str(), answer.size() + 1);
    return copy;
}

__attribute__((export_name("tac_native_free"))) void tac_native_free(char* pointer) {
    std::free(pointer);
}

__attribute__((export_name("tac_http_settle"))) void tac_http_settle(std::uint32_t ticket, const char* text, int length) {
    tachyon::TacTickets::settle(ticket, std::string(text, static_cast<std::size_t>(length)));
}

}
