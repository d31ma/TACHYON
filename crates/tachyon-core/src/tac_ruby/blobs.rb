class Blobs
  API = "blobs"

  def save_held(file_id, key)
    TacWebHost.call(API, "saveHeld", [file_id, key])
    nil
  end

  def download(url, key, headers)
    TacWebHost.call(API, "download", [url, key, headers])
    nil
  end

  def list(prefix)
    TacWebHost.call(API, "list", [prefix])
    nil
  end

  def read(key, offset, count)
    TacWebHost.call(API, "read", [key, offset, count])
    nil
  end

  def remove(key)
    TacWebHost.call(API, "remove", [key])
    nil
  end

  def url(key)
    TacWebHost.call(API, "url", [key]).to_s
  end

  def open(key)
    TacWebHost.call(API, "open", [key])
    nil
  end

  def close(key)
    TacWebHost.call(API, "close", [key])
    nil
  end

  def upload(key, method, url, headers)
    TacWebHost.call(API, "upload", [key, method, url, headers]).to_s
  end

  def cancel(upload)
    TacWebHost.call(API, "cancel", [upload])
    nil
  end
end
