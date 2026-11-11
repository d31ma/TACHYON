
class StorageSpace
  API = "storageSpace"

  def estimate
    TacWebHost.call(API, "estimate")
    nil
  end

  def persist
    TacWebHost.call(API, "persist")
    nil
  end
end
