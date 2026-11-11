#[Service]
class AttachService
  attr_accessor :name, :digest, :status, :problem

  def initialize
    @selected = ""
    @name = ""
    @digest = ""
    @status = ""
    @problem = ""
  end

  def choose
    Files.new.pick(".pdf,image/*", false)
  end

  def upload
    Files.new.upload(@selected, "PUT", "https://uploads.example.com/inbox", "content-type: application/octet-stream")
  end

  # @param picked [FilePicked]
  #[OnEvent]
  def picked(picked)
    @name = "#{picked.name}, #{picked.size} bytes"
    @digest = picked.sha256
    @selected = picked.id
  end

  # @param finished [UploadFinished]
  #[OnEvent]
  def finished(finished)
    @status = "HTTP #{finished.status} #{finished.error}"
  end
end
