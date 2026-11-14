final class Peers {
  const Peers();

  static const _api = 'peers';

  bool supported() => TacWebHost.call(_api, 'supported') as bool;

  String open(String username, String credential) =>
      TacWebHost.call(_api, 'open', [username, credential]) as String;

  void offer(String connection) => TacWebHost.call(_api, 'offer', [connection]);

  void answer(String connection, String offerSdp) =>
      TacWebHost.call(_api, 'answer', [connection, offerSdp]);

  void accept(String connection, String answerSdp) =>
      TacWebHost.call(_api, 'accept', [connection, answerSdp]);

  void addCandidate(String connection, String candidateJson) =>
      TacWebHost.call(_api, 'addCandidate', [connection, candidateJson]);

  void restart(String connection) =>
      TacWebHost.call(_api, 'restart', [connection]);

  String openChannel(String connection, String label, bool ordered) =>
      TacWebHost.call(_api, 'openChannel', [connection, label, ordered])
          as String;

  bool send(String connection, String channel, String data) =>
      TacWebHost.call(_api, 'send', [connection, channel, data]) as bool;

  void closeChannel(String connection, String channel) =>
      TacWebHost.call(_api, 'closeChannel', [connection, channel]);

  void close(String connection) => TacWebHost.call(_api, 'close', [connection]);
}
