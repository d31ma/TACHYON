@Service
class TallyService {
    int count = 6;
    final String runtime = "Java, compiled to WebAssembly by TeaVM";

    int doubled() {
        return count * 2;
    }
}
