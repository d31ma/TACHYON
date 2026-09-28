<?php
#[RequestRoute("/status")]
#[Public]
class StatusController {
    #[GetRoute]
    public function getStatus(): string {
        return 'ok';
    }
}
