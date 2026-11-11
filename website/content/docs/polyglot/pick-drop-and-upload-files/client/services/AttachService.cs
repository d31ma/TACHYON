[Service]
public class AttachService
{
    private readonly Files files = new();
    private string selected = "";
    public string Name = "";
    public string Digest = "";
    public string Status = "";
    public string Problem = "";

    public void Choose() => files.Pick(".pdf,image/*", false);

    public void Upload() => files.Upload(selected, "PUT", "https://uploads.example.com/inbox", "content-type: application/octet-stream");

    [OnEvent]
    public void Picked(FilePicked picked)
    {
        Name = $"{picked.Name}, {picked.Size} bytes";
        Digest = picked.Sha256;
        selected = picked.Id;
    }

    [OnEvent]
    public void Finished(UploadFinished finished)
    {
        Status = $"HTTP {finished.Status} {finished.Error}";
    }
}
