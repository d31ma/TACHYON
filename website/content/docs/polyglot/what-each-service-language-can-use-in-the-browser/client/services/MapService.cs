using Models;

[Service]
public class MapService
{
    public double Latitude = 0;
    public double Longitude = 0;

    [OnEvent]
    public void Show(PlaceFound value)
    {
        Latitude = value.Latitude;
        Longitude = value.Longitude;
    }
}
