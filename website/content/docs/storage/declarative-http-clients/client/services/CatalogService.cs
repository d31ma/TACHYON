using System.Collections.Generic;
using System.Linq;

[Service]
public class CatalogService {
    [Needs]
    private readonly CatalogClient _catalog;
    public List<string> products = new List<string>();
    public string error = "";

    public async System.Threading.Tasks.Task Load() {
        Show(await Fetch());
    }

    private async System.Threading.Tasks.Task<List<string>> Fetch() {
        try {
            var answer = await _catalog.All();
            return answer.Items.Select(item => item.Text).ToList();
        } catch (BehaviorError) {
            return null;
        }
    }

    private void Show(List<string> names) {
        if (names is null) {
            ShowFailure();
            return;
        }
        products = names;
        error = "";
    }

    private void ShowFailure() {
        error = "Products could not be loaded. Please retry.";
    }
}
