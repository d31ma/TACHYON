[Service]
class ProductService {
    public BehaviorJSON accept([Check(typeof(ProductInput))] BehaviorJSON input) {
        return input;
    }
}
