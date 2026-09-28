@Test(GreetingTestSetup, {"partition":"tests"})
export default class GreetingControllerTest {
  async testGetGreetingSaysHello() {
    const controller = this.construct(GreetingController)
    const greeting = controller.getGreeting()
    if (greeting.message !== 'Hello from Yon') throw Error(`message is ${greeting.message}`)
  }
}
