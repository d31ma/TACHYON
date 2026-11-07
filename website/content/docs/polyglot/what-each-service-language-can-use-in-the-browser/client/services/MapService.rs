use crate::models::PlaceFound;

#[Service]
#[derive(Default)]
struct MapService {
    latitude: f64,
    longitude: f64,
}

impl MapService {
    #[OnEvent]
    fn show(&mut self, value: PlaceFound) {
        self.latitude = value.latitude;
        self.longitude = value.longitude;
    }
}
