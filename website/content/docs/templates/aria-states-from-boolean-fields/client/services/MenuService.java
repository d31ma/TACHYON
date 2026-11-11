@Service
class MenuService {
    boolean expanded = false;

    void toggle() { expanded = !expanded; }
}
