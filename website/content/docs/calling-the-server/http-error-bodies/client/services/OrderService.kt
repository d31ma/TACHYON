import models.Problem

@Service
class OrderService {
    @Needs
    private val orderClient: OrderClient
    var status: Int = 0
    var title: String = ""
    var mistakes: Int = 0

    suspend fun send() {
        try {
            orderClient.send(mapOf("name" to "", "level" to 12))
        } catch (error: Behaviors.Failure) {
            val problem = Behaviors.errorBody<Problem>(error)
            status = error.status
            title = problem?.title ?: ""
            mistakes = problem?.errors?.size ?: 0
        }
    }
}
