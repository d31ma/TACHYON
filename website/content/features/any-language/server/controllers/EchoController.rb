#[RequestRoute("/echo")]
#[Public]
class EchoController
  # @param message [Message]
  # @return [Message]
  #[PostRoute]
  def post_echo(
    #[FromBody]
    message
  )
    message
  end
end
