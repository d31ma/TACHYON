@Checked()
class OrderInput {
  @Required()
  @NotEmpty()
  name = '';
  @Min(1)
  @Max(10)
  quantity = 1;
}

@Entity({"adapter":"data"})
@Table("orders")
class OrderRecord {
  @Id
  id = '';
  status = '';
}

@CacheSettings({"name":"orders","ttlMs":1000,"maxBytes":4096,"maxEntries":20})
class OrderService {
  @Cached
  async find(@CacheKey id: string, view: string): Promise<BehaviorJsonValue> {
    return { id, view };
  }

  @ClearCache({"method":"find"})
  async remove(id: string): Promise<BehaviorJsonValue> {
    return id;
  }

  async accept(@Check input: OrderInput): Promise<BehaviorJsonValue> {
    return { name: input.name, quantity: input.quantity };
  }

  list(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) page: BehaviorPage): number {
    return page.offset;
  }
}

@Store(OrderRecord, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
class OrderQueries {
  @Query({"operation":"find","where":[{"field":"status","operator":"eq","argument":0}],"orderBy":[]})
  async listed(_status: string, @PageDefaults _page: BehaviorPageInput) {}
}

// @ts-expect-error CacheKey takes no arguments.
CacheKey('id');
// @ts-expect-error Check reads the class from the parameter type, so it takes no arguments.
Check('OrderInput');
// @ts-expect-error Page defaults need a default size, a maximum size and a sort list.
PageDefaults({ maxSize: 100 });

void OrderService;
void OrderQueries;
