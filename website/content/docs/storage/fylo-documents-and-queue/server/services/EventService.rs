#[Service]
struct EventService {
  #[Needs]
  data_source: std::sync::Arc<DataSource>,
}
impl EventService {
  const COLLECTION: &str = "hd-events";
  const KIND: &str = "kind";
  const COUNT: &str = "count";
  const PAGE_SIZE: u32 = 2;
  const TOPIC: &str = "app.render";
  const GROUP: &str = "renderers";

  fn report(&self) -> Result<Json, YonResourceError> {
    self.data_source.create_collection(Self::COLLECTION)?;
    let id = self.data_source.insert(Self::COLLECTION, Self::deploys())?.get("id").text();
    self.data_source.insert(Self::COLLECTION, Self::deploys())?;
    self.data_source.insert(Self::COLLECTION, Self::deploys())?;
    let found = self.data_source.find_by_id(Self::COLLECTION, &id)?;
    let patched = self.data_source.patch(Self::COLLECTION, &id, Self::change())?;
    let (pages, seen) = self.count_pages()?;
    let job = self.run_job()?;
    self.data_source.delete_by_id(Self::COLLECTION, &id)?;
    let gone = self.data_source.find_by_id(Self::COLLECTION, &id)?.is_null();
    let dropped = self.data_source.drop_collection(Self::COLLECTION)?;
    Ok(Json::map([
      ("found", found.get(Self::KIND).clone()), ("patched", patched.get(Self::COUNT).clone()),
      ("pages", Json::from(pages)), ("seen", Json::from(seen)), ("job", job),
      ("gone", Json::from(gone)), ("dropped", dropped),
    ]))
  }

  fn deploys() -> Json {
    Json::map([(Self::KIND, "deploy")])
  }

  fn change() -> Json {
    Json::map([(Self::COUNT, 7)])
  }

  fn count_pages(&self) -> Result<(usize, usize), YonResourceError> {
    let mut pages = 0;
    let mut seen = 0;
    let mut cursor = String::new();
    let mut more = true;
    while more {
      let window = self.data_source.scroll(Self::COLLECTION, Self::deploys(), &cursor, Self::PAGE_SIZE)?;
      pages += 1;
      seen += window.get("content").items().len();
      cursor = window.get("nextCursor").text();
      more = !cursor.is_empty();
    }
    Ok((pages, seen))
  }

  fn run_job(&self) -> Result<Json, YonResourceError> {
    self.data_source.publish(Self::TOPIC, Json::map([("app", "web")]), "render:web")?;
    let claimed = self.data_source.claim(Self::TOPIC, Self::GROUP, 1, 30000)?;
    let delivery = claimed.items().first().ok_or(YonResourceError)?;
    self.data_source.ack(Self::TOPIC, Self::GROUP, delivery)?;
    Ok(delivery.get("payload").clone())
  }
}
