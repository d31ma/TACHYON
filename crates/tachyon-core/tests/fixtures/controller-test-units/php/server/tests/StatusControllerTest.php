<?php
#[Test(StatusTestSetup::class, {"partition":"tests"})]
class StatusControllerTest {
    public function testGetStatusSaysOk(): void {
        if ((new StatusController())->getStatus() !== 'ok') throw new Exception('expected ok');
    }
}
