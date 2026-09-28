using Models;

public class GuideService {
    public string title;
    public int pages;

    public GuideService() {
        var guide = new Guide("Models hold types", 3);
        title = guide.Title;
        pages = guide.Pages;
    }
}
