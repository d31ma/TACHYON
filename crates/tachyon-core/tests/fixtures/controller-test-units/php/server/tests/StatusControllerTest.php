<?php
#[Test(StatusTestSetup::class, {"partition":"tests"})]
class StatusControllerTest {
    public function testGetStatusSaysOk(): void {
        Assert::equals('ok', (new StatusController())->getStatus());
    }
}
