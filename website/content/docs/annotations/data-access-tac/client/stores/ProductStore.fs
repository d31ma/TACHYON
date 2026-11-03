open System.Threading.Tasks

[<Store(typeof<ProductEntity>, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})>]
type ProductStore =
    inherit Repository<ProductEntity, int>

    abstract findByStatus: status: BehaviorJSON -> Task<BehaviorJSON>

    abstract countByStatus: status: BehaviorJSON -> Task<BehaviorJSON>

    [<Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})>]
    abstract excluding: status: BehaviorJSON * [<PageDefaults>] page: BehaviorJSON -> Task<BehaviorJSON>
