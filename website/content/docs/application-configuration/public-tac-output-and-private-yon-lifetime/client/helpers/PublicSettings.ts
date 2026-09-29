export default class PublicSettings {
  @Setting("${tac.api.url:/api}")
  api!: string;
}
