import models.ShortDetail
data class Summary(val id: Int, val name: String, val detail: ShortDetail, val items: List<ShortDetail>, val kind: String)
