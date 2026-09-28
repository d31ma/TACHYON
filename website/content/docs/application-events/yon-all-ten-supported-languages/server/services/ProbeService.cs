[Service]
class ProbeService {
  public Models.ProbeReady Last;

  [OnEvent]
  public void Observe(Models.ProbeReady ready) { Last = ready; }

  [OnStop]
  public void Destroy() { Last = null; }
}
