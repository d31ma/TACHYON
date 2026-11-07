namespace tachyon::yon {

class YonFakes {
public:
    template <typename Provider>
    static Provider* of() {
        Provider* fake = TacFake<Provider>::slot();
        return TacSharedMaking::active() ? nullptr : fake;
    }
};

}
