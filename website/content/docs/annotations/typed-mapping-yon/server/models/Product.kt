import models.Detail
data class Product(val id: Int, val name: String, val detail: Detail, val items: List<Detail>, val secret: String)
