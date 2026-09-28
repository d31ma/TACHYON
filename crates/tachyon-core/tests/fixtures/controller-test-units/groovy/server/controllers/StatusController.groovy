@RequestRoute('/status')
@Public
class StatusController {
    @GetRoute
    String getStatus() {
        return 'ok'
    }
}
