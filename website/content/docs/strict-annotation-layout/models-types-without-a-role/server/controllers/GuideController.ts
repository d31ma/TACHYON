import type { Guide } from '/models/Guide.ts';

@RequestRoute("/guide")
@Public
export default class GuideController {
  @GetRoute
  getGuide(): Guide {
    return { title: 'Models hold types', pages: 3 };
  }
}
