class Peers
  API = "peers"

  def supported
    TacWebHost.call(API, "supported") == true
  end

  def open(username, credential)
    TacWebHost.call(API, "open", [username, credential]).to_s
  end

  def offer(connection)
    TacWebHost.call(API, "offer", [connection])
    nil
  end

  def answer(connection, offer_sdp)
    TacWebHost.call(API, "answer", [connection, offer_sdp])
    nil
  end

  def accept(connection, answer_sdp)
    TacWebHost.call(API, "accept", [connection, answer_sdp])
    nil
  end

  def add_candidate(connection, candidate_json)
    TacWebHost.call(API, "addCandidate", [connection, candidate_json])
    nil
  end

  def restart(connection)
    TacWebHost.call(API, "restart", [connection])
    nil
  end

  def open_channel(connection, label, ordered)
    TacWebHost.call(API, "openChannel", [connection, label, ordered == true]).to_s
  end

  def send_data(connection, channel, data)
    TacWebHost.call(API, "send", [connection, channel, data]) == true
  end

  def close_channel(connection, channel)
    TacWebHost.call(API, "closeChannel", [connection, channel])
    nil
  end

  def close(connection)
    TacWebHost.call(API, "close", [connection])
    nil
  end
end
