[<Test(typeof<GreetingTestSetup>, {"partition":"tests"})>]
type GreetingControllerTest() =
    member this.testGetGreetingSaysHello() : unit =
        let controller = GreetingController()
        let greeting = controller.GetGreeting()
        Assert.equals("Hello from Yon", greeting.Message)
