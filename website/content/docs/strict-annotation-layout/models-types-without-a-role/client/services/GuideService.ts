import type { Guide } from '/models/Guide.ts';

@Service
export default class GuideService {
  private current: Guide = { title: 'Models hold types', pages: 3 };

  get title(): string {
    return this.current.title;
  }

  get pages(): number {
    return this.current.pages;
  }
}
