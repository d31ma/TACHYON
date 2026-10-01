@Service
class InstallService {
    private String system = "unix";

    void pick(String chosen) {
        system = chosen;
    }

    boolean shows(String candidate) {
        return java.util.Objects.equals(candidate, system);
    }
}
