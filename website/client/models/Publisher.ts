export interface Publisher {
  publishEvent(event: object, retain?: boolean): void
}
