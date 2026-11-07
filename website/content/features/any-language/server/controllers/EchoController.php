<?php
use Models\Message;

#[RequestRoute("/echo")]
#[Public]
class EchoController {
    #[PostRoute]
    public function postEcho(#[FromBody] Message $message): Message {
        return $message;
    }
}
