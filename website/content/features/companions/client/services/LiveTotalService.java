import models.CartTotal;

@Service
class LiveTotalService {
    int seen = 0;

    @OnEvent({"replay":true})
    void changed(CartTotal changed) { seen = changed.total(); }
}
