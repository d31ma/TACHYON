
BrowserFakes = {
  MISSING = "This browser API needs the page. A test unit has no page: give the call an answer with BrowserFakes.answer in the test unit.",
  answers = {},
}

function BrowserFakes.answer(api, call, value)
  BrowserFakes.answers[api .. "." .. call] = { value = value }
end

function BrowserFakes.reply(request)
  local found = BrowserFakes.answers[tostring(request.api) .. "." .. tostring(request.call)]
  if found == nil then
    return { error = BrowserFakes.MISSING }
  end
  return { value = found.value }
end

TacWebHost = {}

function TacWebHost.call(api, call, arguments)
  local answer = TacWebHost.ask({ api = api, call = call, args = arguments or {} })
  if type(answer.error) == "string" then
    error(answer.error, 0)
  end
  return answer.value
end

function TacWebHost.ask(request)
  local ok, answer = pcall(YonJson.decode, TachyonHost.ask(YonJson.encode(request)) or "")
  if not ok or type(answer) ~= "table" then
    return { error = "Tachyon browser APIs did not answer." }
  end
  return answer
end

Clock = {}

function Clock:now_ms()
  return TacWebHost.call("clock", "now") + 0.0
end

function Clock:monotonic_ms()
  return TacWebHost.call("clock", "monotonic") + 0.0
end

function Clock:time_zone()
  return tostring(TacWebHost.call("clock", "timeZone"))
end

BehaviorHostClock = {
  NO_WAIT = "Lua services in the browser cannot wait between attempts. Write this service in another Tac language, or leave Retry out.",
}

function BehaviorHostClock.now_ms()
  return math.floor(Clock:now_ms())
end

function BehaviorHostClock.monotonic_ms()
  return Clock:monotonic_ms()
end

function BehaviorHostClock.wait()
  error(BehaviorHostClock.NO_WAIT, 0)
end

SecureRandom = {}

function SecureRandom:bytes(count)
  return TacWebHost.call("secureRandom", "bytes", { count })
end

function SecureRandom:uuid()
  return tostring(TacWebHost.call("secureRandom", "uuid"))
end

Connectivity = {}

function Connectivity:online()
  return TacWebHost.call("connectivity", "online") == true
end

PageVisibility = {}

function PageVisibility:visible()
  return TacWebHost.call("pageVisibility", "visible") == true
end

Preferences = {}

function Preferences:scheme()
  return tostring(TacWebHost.call("preferences", "scheme"))
end

function Preferences:motion()
  return tostring(TacWebHost.call("preferences", "motion"))
end

function Preferences:language()
  return tostring(TacWebHost.call("preferences", "language"))
end

Clipboard = {}

function Clipboard:write_text(text)
  TacWebHost.call("clipboard", "writeText", { text })
end

function Clipboard:read_text()
  TacWebHost.call("clipboard", "readText")
end

Share = {}

function Share:open(title, text, url)
  TacWebHost.call("share", "open", { title, text, url })
end

Focus = {}

function Focus:move(target)
  TacWebHost.call("focus", "move", { target })
end

function Focus:trap(container)
  TacWebHost.call("focus", "trap", { container })
end

function Focus:release()
  TacWebHost.call("focus", "release")
end

Files = {}

function Files:pick(accept, multiple)
  TacWebHost.call("files", "pick", { accept, multiple })
end

function Files:read(id, offset, count)
  TacWebHost.call("files", "read", { id, offset, count })
end

function Files:release(id)
  TacWebHost.call("files", "release", { id })
end

function Files:upload(id, method, url, headers)
  return tostring(TacWebHost.call("files", "upload", { id, method, url, headers }))
end

function Files:cancel(upload)
  TacWebHost.call("files", "cancel", { upload })
end

Notifications = {}

function Notifications:permission()
  return tostring(TacWebHost.call("notifications", "permission"))
end

function Notifications:request_permission()
  TacWebHost.call("notifications", "requestPermission")
end

function Notifications:show(title, body)
  TacWebHost.call("notifications", "show", { title, body })
end

Push = {}

function Push:supported()
  return TacWebHost.call("push", "supported") == true
end

function Push:subscribe(application_server_key)
  TacWebHost.call("push", "subscribe", { application_server_key })
end

function Push:subscription()
  TacWebHost.call("push", "subscription")
end

function Push:unsubscribe()
  TacWebHost.call("push", "unsubscribe")
end

StorageSpace = {}

function StorageSpace:estimate()
  TacWebHost.call("storageSpace", "estimate")
end

function StorageSpace:persist()
  TacWebHost.call("storageSpace", "persist")
end

Passkeys = {}

function Passkeys:supported()
  return TacWebHost.call("passkeys", "supported") == true
end

function Passkeys:check()
  TacWebHost.call("passkeys", "check")
end

function Passkeys:create(options_json, prf_key)
  TacWebHost.call("passkeys", "create", { options_json, prf_key })
end

function Passkeys:get(options_json, prf_key)
  TacWebHost.call("passkeys", "get", { options_json, prf_key })
end

function Passkeys:offer(options_json, prf_key)
  TacWebHost.call("passkeys", "offer", { options_json, prf_key })
end

function Passkeys:cancel()
  TacWebHost.call("passkeys", "cancel")
end

Media = {}

function Media:supported(feature)
  return TacWebHost.call("media", "supported", { feature }) == true
end

function Media:devices(request)
  TacWebHost.call("media", "devices", { request })
end

function Media:choose_speaker(device)
  TacWebHost.call("media", "chooseSpeaker", { device })
end

function Media:capture_microphone(track, device, processing)
  TacWebHost.call("media", "captureMicrophone", { track, device, processing })
end

function Media:capture_camera(track, device, quality)
  TacWebHost.call("media", "captureCamera", { track, device, quality })
end

function Media:capture_screen(track, with_audio)
  TacWebHost.call("media", "captureScreen", { track, with_audio })
end

function Media:stop_track(track)
  TacWebHost.call("media", "stopTrack", { track })
end

function Media:mute(track, muted)
  TacWebHost.call("media", "mute", { track, muted })
end

function Media:use_frame_key(key_id, key)
  TacWebHost.call("media", "useFrameKey", { key_id, key })
end

function Media:send_frame_key(key_id)
  TacWebHost.call("media", "sendFrameKey", { key_id })
end

function Media:forget_frame_key(key_id)
  TacWebHost.call("media", "forgetFrameKey", { key_id })
end

function Media:encrypt_frames(connection, on)
  TacWebHost.call("media", "encryptFrames", { connection, on })
end

function Media:call_active(on, title)
  TacWebHost.call("media", "callActive", { on, title })
end

function Media:send(connection, track, stream, simulcast)
  TacWebHost.call("media", "send", { connection, track, stream, simulcast })
end

function Media:replace(connection, track, with_track)
  TacWebHost.call("media", "replace", { connection, track, with_track })
end

function Media:unsend(connection, track)
  TacWebHost.call("media", "unsend", { connection, track })
end

function Media:stats(request, connection)
  TacWebHost.call("media", "stats", { request, connection })
end

function Media:speaking(on)
  TacWebHost.call("media", "speaking", { on })
end

function Media:resume_audio()
  TacWebHost.call("media", "resumeAudio")
end

CodeScanner = {}

function CodeScanner:supported()
  return TacWebHost.call("codeScanner", "supported") == true
end

function CodeScanner:scan()
  TacWebHost.call("codeScanner", "scan")
end

function CodeScanner:cancel()
  TacWebHost.call("codeScanner", "cancel")
end

Crypto = {}

function Crypto:supported()
  return TacWebHost.call("crypto", "supported") == true
end

function Crypto:request(call, values)
  return tostring(TacWebHost.call("crypto", call, values))
end

function Crypto:check()
  return Crypto:request("check", {})
end

function Crypto:make_key_pair(key, algorithm, keep)
  return Crypto:request("makeKeyPair", { key, algorithm, keep == true })
end

function Crypto:public_key(key)
  return Crypto:request("publicKey", { key })
end

function Crypto:sign(key, data)
  return Crypto:request("sign", { key, data })
end

function Crypto:verify(public_key, data, signature)
  return Crypto:request("verify", { public_key, data, signature })
end

function Crypto:agree(key, peer_public_key, salt, info, new_key, use, keep)
  return Crypto:request("agree", { key, peer_public_key, salt, info, new_key, use, keep == true })
end

function Crypto:derive_key(from_key, salt, info, new_key, use, keep)
  return Crypto:request("deriveKey", { from_key, salt, info, new_key, use, keep == true })
end

function Crypto:import_key(new_key, secret, use, keep)
  return Crypto:request("importKey", { new_key, secret, use, keep == true })
end

function Crypto:from_code(new_key, code, salt, use, keep)
  return Crypto:request("fromCode", { new_key, code, salt, use, keep == true })
end

function Crypto:encrypt(key, data, aad)
  return Crypto:request("encrypt", { key, data, aad })
end

function Crypto:decrypt(key, sealed, aad)
  return Crypto:request("decrypt", { key, sealed, aad })
end

function Crypto:hkdf(input, salt, info, length)
  return Crypto:request("hkdf", { input, salt, info, length })
end

function Crypto:hash(data)
  return Crypto:request("hash", { data })
end

function Crypto:forget(key)
  return Crypto:request("forget", { key })
end

function Crypto:forget_all()
  return Crypto:request("forgetAll", {})
end

Blobs = {}

function Blobs:save_held(file_id, key)
  TacWebHost.call("blobs", "saveHeld", { file_id, key })
end

function Blobs:download(url, key, headers)
  TacWebHost.call("blobs", "download", { url, key, headers })
end

function Blobs:list(prefix)
  TacWebHost.call("blobs", "list", { prefix })
end

function Blobs:read(key, offset, count)
  TacWebHost.call("blobs", "read", { key, offset, count })
end

function Blobs:remove(key)
  TacWebHost.call("blobs", "remove", { key })
end

function Blobs:url(key)
  return tostring(TacWebHost.call("blobs", "url", { key }))
end

function Blobs:open(key)
  TacWebHost.call("blobs", "open", { key })
end

function Blobs:close(key)
  TacWebHost.call("blobs", "close", { key })
end

function Blobs:upload(key, method, url, headers)
  return tostring(TacWebHost.call("blobs", "upload", { key, method, url, headers }))
end

function Blobs:cancel(upload)
  TacWebHost.call("blobs", "cancel", { upload })
end

Peers = {}

function Peers:supported()
  return TacWebHost.call("peers", "supported") == true
end

function Peers:open(username, credential)
  return TacWebHost.call("peers", "open", { username, credential })
end

function Peers:offer(connection)
  TacWebHost.call("peers", "offer", { connection })
end

function Peers:answer(connection, offer_sdp)
  TacWebHost.call("peers", "answer", { connection, offer_sdp })
end

function Peers:accept(connection, answer_sdp)
  TacWebHost.call("peers", "accept", { connection, answer_sdp })
end

function Peers:add_candidate(connection, candidate_json)
  TacWebHost.call("peers", "addCandidate", { connection, candidate_json })
end

function Peers:restart(connection)
  TacWebHost.call("peers", "restart", { connection })
end

function Peers:open_channel(connection, label, ordered)
  return TacWebHost.call("peers", "openChannel", { connection, label, ordered == true })
end

function Peers:send(connection, channel, data)
  return TacWebHost.call("peers", "send", { connection, channel, data }) == true
end

function Peers:close_channel(connection, channel)
  TacWebHost.call("peers", "closeChannel", { connection, channel })
end

function Peers:close(connection)
  TacWebHost.call("peers", "close", { connection })
end

Geolocation = {}

function Geolocation:current_position()
  TacWebHost.call("geolocation", "currentPosition")
end
