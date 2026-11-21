
class Recorder
  API = "recorder"

  def supported
    TacWebHost.call(API, "supported") == true
  end

  def start(recording, audio_tracks, video_track, file_name, format)
    TacWebHost.call(API, "start", [recording, audio_tracks, video_track, file_name, format])
    nil
  end

  def add_audio(recording, track)
    TacWebHost.call(API, "addAudio", [recording, track])
    nil
  end

  def remove_audio(recording, track)
    TacWebHost.call(API, "removeAudio", [recording, track])
    nil
  end

  def show_video(recording, track)
    TacWebHost.call(API, "showVideo", [recording, track])
    nil
  end

  def pause(recording)
    TacWebHost.call(API, "pause", [recording])
    nil
  end

  def resume(recording)
    TacWebHost.call(API, "resume", [recording])
    nil
  end

  def stop(recording)
    TacWebHost.call(API, "stop", [recording])
    nil
  end
end
