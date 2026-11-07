import type { ReleaseChanged } from '/models/ReleaseChanged.ts';

@Service
export default class ReleaseService {
  applicationEventPublisher!: ApplicationEventPublisher;
  release: string = '';
  announce(): void {
    const changed: ReleaseChanged = { release: 'TACHYON' };
    this.applicationEventPublisher.publishEvent(changed, true);
  }
  @OnEvent({"replay":true})
  receive(changed: ReleaseChanged): void { this.release = changed.release; }
}
