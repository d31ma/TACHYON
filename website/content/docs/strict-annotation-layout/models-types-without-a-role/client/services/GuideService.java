import models.Guide;

@Service
class GuideService {
    String title = "";
    int pages = 0;

    GuideService() {
        Guide guide = new Guide("Models hold types", 3);
        title = guide.title();
        pages = guide.pages();
    }
}
