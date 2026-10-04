namespace tachyon {

class TacNeeds {
public:
    template <typename Provider>
    static Provider& of() {
        static Provider provider;
        return provider;
    }
};

}
