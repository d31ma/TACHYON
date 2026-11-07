class Middleware
  def self.before(request)
    return nil if !request.path.start_with?("/private") || request["authorization"] == "let-me-in"

    [401, { "content-type" => "application/json" }, [JSON.generate({ "error" => "Unauthorized" })]]
  end

  def self.after(request)
    [204, { "x-served-by" => "tachyon" }, []]
  end
end
