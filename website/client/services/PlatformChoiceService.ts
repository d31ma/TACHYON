import { rememberPlatform } from '/shared/scripts/platforms.js'
import type { Publisher } from '/models/Publisher.ts'
import type { PlatformChanged } from '/models/PlatformChanged.ts'

@Service
export default class PlatformChoiceService {
  @Needs
  applicationEventPublisher!: Publisher

  choose(_event: unknown, id: string): void {
    rememberPlatform(id)
    const changed: PlatformChanged = { platform: id }
    this.applicationEventPublisher.publishEvent(changed, true)
  }
}
