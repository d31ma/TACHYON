[Service]
class OrderService
{
    public static string[] List() => OrderStore.All();
}
