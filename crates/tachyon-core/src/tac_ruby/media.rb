
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
end
