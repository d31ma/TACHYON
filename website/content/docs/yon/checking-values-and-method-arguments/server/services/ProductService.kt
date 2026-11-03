@Service
class ProductService {
    fun accept(@Check(ProductInput::class) input: Any?): Any? {
        return input
    }
}
