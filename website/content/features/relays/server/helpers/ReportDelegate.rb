require 'json'

class ReportDelegate
  def self.run
    request = JSON.parse($stdin.read) rescue {}
    body = JSON.generate({ 'language' => 'Ruby', 'route' => request['route'] })
    puts JSON.generate({
      'status' => 200,
      'headers' => { 'content-type' => ['application/json'] },
      'body' => body
    })
  end
end

ReportDelegate.run
