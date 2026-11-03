import models.CartTotal

@Service
class LiveTotalService {
    var seen: Int = 0

    @OnEvent({"replay":true})
    fun changed(changed: CartTotal) { seen = changed.total }
}
