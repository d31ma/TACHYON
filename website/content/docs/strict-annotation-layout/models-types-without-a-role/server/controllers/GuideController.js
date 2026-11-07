@RequestRoute("/guide")
@Public
export default class GuideController {
  /** @returns {Guide} */
  @GetRoute
  getGuide() {
    return { title: 'Models hold types', pages: 3 }
  }
}
