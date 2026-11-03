@Service
class ProductService {
  Object? accept(@Check(ProductInput) Object? input) {
    return input;
  }
}
