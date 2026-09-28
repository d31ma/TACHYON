@Store
class CatalogStore {
  @Needs
  val dataSource: DataSource
  fun find(): Any? = dataSource.query("SELECT 42 AS amount", emptyList())
}