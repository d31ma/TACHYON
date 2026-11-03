<?php
use Models\Guide;

#[RequestRoute("/guide")]
#[Public]
class GuideController {
    #[GetRoute]
    public function getGuide(): Guide {
        return new Guide('Models hold types', 3);
    }
}
