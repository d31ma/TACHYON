[Service]
public class MenuService
{
    public bool Expanded = false;

    public void Toggle() => Expanded = !Expanded;
}
