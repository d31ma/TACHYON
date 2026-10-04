use crate::models::SectionChosen;

#[Service]
#[derive(Default)]
struct SectionService {
    section: String,
    query: String,
}

impl SectionService {
    fn search(&mut self, query: String) {
        self.query = query;
    }

    #[OnEvent]
    fn chosen(&mut self, chosen: SectionChosen) {
        self.section = chosen.section;
    }
}
