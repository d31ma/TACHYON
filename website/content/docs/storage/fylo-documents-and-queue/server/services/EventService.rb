#[Service]
class EventService
  COLLECTION = "hd-events"
  DEPLOYS = { "kind" => "deploy" }.freeze
  CHANGE = { "count" => 7 }.freeze
  PAGE_SIZE = 2
  TOPIC = "app.render"
  GROUP = "renderers"

  #[Needs]
  # @return [DataSource]
  attr_reader :data_source

  # @return [Hash]
  def report
    data_source.create_collection(COLLECTION)
    id = data_source.insert(COLLECTION, DEPLOYS)["id"]
    data_source.insert(COLLECTION, DEPLOYS)
    data_source.insert(COLLECTION, DEPLOYS)
    found = data_source.find_by_id(COLLECTION, id)
    patched = data_source.patch(COLLECTION, id, CHANGE)
    scrolled = count_pages
    job = run_job
    data_source.delete_by_id(COLLECTION, id)
    gone = data_source.find_by_id(COLLECTION, id).nil?
    dropped = data_source.drop_collection(COLLECTION)
    { "found" => found["kind"], "patched" => patched["count"], "job" => job, "gone" => gone, "dropped" => dropped }.merge(scrolled)
  end

  private

  # @return [Hash]
  def count_pages
    pages = 0
    seen = 0
    cursor = nil
    more = true
    while more
      window = data_source.scroll(COLLECTION, DEPLOYS, cursor, PAGE_SIZE)
      pages += 1
      seen += window["content"].length
      cursor = window["nextCursor"]
      more = !cursor.nil?
    end
    { "pages" => pages, "seen" => seen }
  end

  # @return [Hash]
  def run_job
    data_source.publish(TOPIC, { "app" => "web" }, "render:web")
    delivery = data_source.claim(TOPIC, GROUP, 1, 30_000).first
    data_source.ack(TOPIC, GROUP, delivery)
    delivery["payload"]
  end
end
