<?php
use Models\Identity;

#[RequestRoute("/me")]
#[SignedIn]
class MeController {
    #[GetRoute]
    public function getMe(#[FromIdentity] Identity $caller): string {
        return $caller->user;
    }
}
