using System;
using System.Collections.Generic;
using System.Linq;
using Models;

[Service]
public class InventoryService
{
    public List<InventoryRecord> Records = new List<InventoryRecord>();

    [OnEvent]
    public void AddRecord(InventoryRecord record)
    {
        var older = Math.Max(0, Records.Count - 63);
        Records = Records.Skip(older).Append(record).ToList();
    }
}
