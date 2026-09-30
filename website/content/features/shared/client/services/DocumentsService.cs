using System.Collections.Generic;

[Service]
[NeedsAll]
public class DocumentsService
{
    public required List<string> Order;

    public int Guides() => Order.Count;
}
