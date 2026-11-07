#[Service]
class ItemService
  # @param path [String]
  # @return [String]
  def id_from(path)
    path.split("/").last.to_s
  end
end
