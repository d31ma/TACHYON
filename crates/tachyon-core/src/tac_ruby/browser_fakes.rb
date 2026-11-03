
class BrowserFakes
  MISSING = "This browser API needs the page. A test unit has no page: give the call an answer with BrowserFakes.answer in the test unit."

  def self.answer(api, call, value)
    answers["#{api}.#{call}"] = value
  end

  def self.reply(request)
    key = "#{request['api']}.#{request['call']}"
    return { "error" => MISSING } unless answers.key?(key)
    { "value" => answers[key] }
  end

  def self.answers
    @answers ||= {}
  end
end
