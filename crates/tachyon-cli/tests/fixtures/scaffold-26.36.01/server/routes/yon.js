@Controller
export class RootController {
  static async GET() {
    return { ok: true, framework: 'Tachyon' }
  }
}
