@Service
class ProductService {
    Object accept(@Check(ProductInput.class) Object input) {
        return input;
    }
}
