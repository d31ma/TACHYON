export default class Endpoint {
  @Setting("${yon.active.profile:development}")
  profile!: string;
}
