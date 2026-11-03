<?php
use Models\Greeting;

#[RequestRoute("/api/greeting")]
#[Public]
class GreetingController {
    #[GetRoute]
    public function getGreeting(): Greeting {
        return new Greeting('Hello from Yon');
    }
}
