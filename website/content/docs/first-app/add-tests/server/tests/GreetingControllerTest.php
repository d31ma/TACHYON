<?php
#[Test(GreetingTestSetup::class, {"partition":"tests"})]
class GreetingControllerTest {
    public function testGetGreetingSaysHello(): void {
        $controller = new GreetingController();
        $greeting = $controller->getGreeting();
        Assert::equals('Hello from Yon', $greeting->message);
    }
}
