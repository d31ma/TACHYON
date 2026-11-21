
class Media
  API = "media"

  def supported(feature)
    TacWebHost.call(API, "supported", [feature]) == true
  end

  def devices(request)
    TacWebHost.call(API, "devices", [request])
    nil
  end

  def choose_speaker(device)
    TacWebHost.call(API, "chooseSpeaker", [device])
    nil
  end

  def capture_microphone(track, device, processing)
    TacWebHost.call(API, "captureMicrophone", [track, device, processing])
    nil
  end

  def capture_camera(track, device, quality)
    TacWebHost.call(API, "captureCamera", [track, device, quality])
    nil
  end

  def capture_screen(track, with_audio)
    TacWebHost.call(API, "captureScreen", [track, with_audio])
    nil
  end

  def stop_track(track)
    TacWebHost.call(API, "stopTrack", [track])
    nil
  end

  def mute(track, muted)
    TacWebHost.call(API, "mute", [track, muted])
    nil
  end

  def use_frame_key(key_id, key)
    TacWebHost.call(API, "useFrameKey", [key_id, key])
    nil
  end

  def send_frame_key(key_id)
    TacWebHost.call(API, "sendFrameKey", [key_id])
    nil
  end

  def forget_frame_key(key_id)
    TacWebHost.call(API, "forgetFrameKey", [key_id])
    nil
  end

  def encrypt_frames(connection, on)
    TacWebHost.call(API, "encryptFrames", [connection, on])
    nil
  end

  def call_active(on, title)
    TacWebHost.call(API, "callActive", [on, title])
    nil
  end

  def send(connection, track, stream, simulcast)
    TacWebHost.call(API, "send", [connection, track, stream, simulcast])
    nil
  end

  def replace(connection, track, with_track)
    TacWebHost.call(API, "replace", [connection, track, with_track])
    nil
  end

  def unsend(connection, track)
    TacWebHost.call(API, "unsend", [connection, track])
    nil
  end

  def stats(request, connection)
    TacWebHost.call(API, "stats", [request, connection])
    nil
  end

  def speaking(on)
    TacWebHost.call(API, "speaking", [on])
    nil
  end

  def resume_audio
    TacWebHost.call(API, "resumeAudio")
    nil
  end
end
