
class Files
  def pick(accept, multiple)
    TacWebHost.call("files", "pick", [accept, multiple])
    nil
  end

  def read(id, offset, count)
    TacWebHost.call("files", "read", [id, offset, count])
    nil
  end

  def release(id)
    TacWebHost.call("files", "release", [id])
    nil
  end

  def upload(id, method, url, headers)
    TacWebHost.call("files", "upload", [id, method, url, headers]).to_s
  end

  def cancel(upload)
    TacWebHost.call("files", "cancel", [upload])
    nil
  end
end
