use crate::models::UserSaved;

#[Service]
#[derive(Default)]
pub struct UserListService {
    pub names: Vec<String>,
}

impl UserListService {
    #[OnEvent]
    fn user_saved(&mut self, saved: UserSaved) {
        self.names.push(saved.name);
    }
}
