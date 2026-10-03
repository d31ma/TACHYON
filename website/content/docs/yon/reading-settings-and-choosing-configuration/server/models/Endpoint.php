<?php

class Endpoint {
    #[Setting('${yon.active.profile:development}')]
    public string $profile = '';
}
