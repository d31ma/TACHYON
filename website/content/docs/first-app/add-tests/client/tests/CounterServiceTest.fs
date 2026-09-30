[<Test(typeof<HelloTestSetup>, {"partition":"tests"})>]
type CounterServiceTest() =
    member this.testIncrementAddsOne() : unit =
        let counter = CounterService()
        counter.Increment()
        Assert.equals(1, counter.Count)
