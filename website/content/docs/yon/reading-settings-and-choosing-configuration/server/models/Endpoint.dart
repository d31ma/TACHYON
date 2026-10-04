class Endpoint {
  @Setting("${yon.active.profile:development}")
  final String profile;

  const Endpoint({this.profile = ''});
}
