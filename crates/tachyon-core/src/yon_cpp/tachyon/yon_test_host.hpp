namespace tachyon {

class TacHost : public YonHostStorage {
public:
    static constexpr const char* REALM = "yon";

    static void reset() {
        TacMemoryEntities::reset();
    }

    static double epochMilliseconds() {
        return 0;
    }

    static void waitMilliseconds(double) {}
};

}
