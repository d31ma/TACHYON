pub(super) struct NativeCryptoChecks;

impl NativeCryptoChecks {
    pub(super) const HELPERS: &'static str = r"let last = 'nothing';
const call = async (name, ...values) => {
  const raw = await window.webkit.messageHandlers.tachyon.postMessage({ capability: 'browserApi.crypto.' + name, payload: JSON.stringify(values) });
  last = name + ' ' + raw;
  const reply = JSON.parse(raw);
  return 'value' in reply ? reply.value : { reason: 'error' };
};
const word = (answer, field) => answer.reason ?? (field in answer ? 'ok' : 'missing');
const b64 = (bytes) => btoa(String.fromCharCode(...new Uint8Array(bytes))).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
const unb64 = (text) => Uint8Array.from(atob(text.replace(/-/g, '+').replace(/_/g, '/') + '==='.slice((text.length + 3) % 4)), (c) => c.charCodeAt(0));
const hex = (text) => Uint8Array.from(text.match(/../g), (pair) => parseInt(pair, 16));
const text = (value) => b64(new TextEncoder().encode(value));
const subtle = crypto.subtle;
const sealWith = async (aesKey, plain, aad) => {
  const nonce = crypto.getRandomValues(new Uint8Array(12));
  const cipher = new Uint8Array(await subtle.encrypt({ name: 'AES-GCM', iv: nonce, additionalData: aad }, aesKey, new TextEncoder().encode(plain)));
  return b64(new Uint8Array([1, ...nonce, ...cipher]));
};
const openWith = async (aesKey, sealed, aad) => {
  const bytes = unb64(sealed);
  const plain = await subtle.decrypt({ name: 'AES-GCM', iv: bytes.subarray(1, 13), additionalData: aad }, aesKey, bytes.subarray(13));
  return new TextDecoder().decode(plain);
};
const hkdfKey = async (raw, salt, info) => {
  const base = await subtle.importKey('raw', raw, 'HKDF', false, ['deriveBits']);
  const bits = await subtle.deriveBits({ name: 'HKDF', hash: 'SHA-256', salt, info }, base, 256);
  return subtle.importKey('raw', bits, 'AES-GCM', false, ['encrypt', 'decrypt']);
};
const decrypted = async (key, sealed, aad) => {
  const answer = await call('decrypt', key, sealed, b64(aad));
  return answer.reason ?? new TextDecoder().decode(unb64(answer.data));
};
const results = [];
";
    pub(super) const FIRST: &'static str = r"const check = await call('check');
results.push('store:' + check.keyStore);
const keep = check.keyStore === 'os';
results.push('keep:' + (keep ? 'os' : word(await call('makeKeyPair', 'kept-probe', 'ed25519', true), 'key')));
const digest = await call('hash', text('abc'));
results.push(digest.digest === b64(hex('ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad')) ? 'hash' : 'hash-wrong');
const okm = await call('hkdf', b64(new Uint8Array(22).fill(11)), b64(hex('000102030405060708090a0b0c')), b64(hex('f0f1f2f3f4f5f6f7f8f9')), 42);
results.push(okm.bytes === b64(hex('3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865')) ? 'hkdf' : 'hkdf-wrong');
const rfcKey = b64(hex('3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c'));
const rfcSignature = hex('92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00');
results.push('rfc8032:' + (await call('verify', rfcKey, b64(hex('72')), b64(rfcSignature))).valid);
const emptyKey = b64(hex('d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a'));
const emptySignature = b64(hex('e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b'));
results.push('rfc8032-empty:' + (await call('verify', emptyKey, '', emptySignature)).valid);
rfcSignature[0] ^= 1;
results.push('changed:' + (await call('verify', rfcKey, b64(hex('72')), b64(rfcSignature))).valid);
const device = await call('makeKeyPair', 'device', 'ed25519', keep);
localStorage.setItem('device', device.publicKey);
const signature = await call('sign', 'device', text('hello'));
const webSigner = await subtle.importKey('raw', unb64(device.publicKey), { name: 'Ed25519' }, false, ['verify']);
results.push('webkit-verify:' + await subtle.verify({ name: 'Ed25519' }, webSigner, unb64(signature.signature), new TextEncoder().encode('hello')));
const emptySigned = await call('sign', 'device', '');
results.push('empty-round-trip:' + (await call('verify', device.publicKey, '', emptySigned.signature)).valid);
const secret = crypto.getRandomValues(new Uint8Array(32));
results.push('import:' + word(await call('importKey', 'shared-aes', b64(secret), 'encrypt', false), 'key'));
const webAes = await subtle.importKey('raw', secret, 'AES-GCM', false, ['encrypt', 'decrypt']);
const aad = new TextEncoder().encode('chat-1');
results.push('host-opens:' + await decrypted('shared-aes', await sealWith(webAes, 'from webkit', aad), aad));
const sealed = await call('encrypt', 'shared-aes', text('from host'), b64(aad));
results.push('webkit-opens:' + await openWith(webAes, sealed.sealed, aad));
results.push('wrong-aad:' + await decrypted('shared-aes', sealed.sealed, new TextEncoder().encode('chat-2')));
const mine = await call('makeKeyPair', 'agree-key', 'x25519', false);
const theirs = await subtle.generateKey({ name: 'X25519' }, true, ['deriveBits']);
const theirPublic = new Uint8Array(await subtle.exportKey('raw', theirs.publicKey));
const salt = crypto.getRandomValues(new Uint8Array(16));
const info = new TextEncoder().encode('mesail');
results.push('agree:' + word(await call('agree', 'agree-key', b64(theirPublic), b64(salt), b64(info), 'agreed', 'encrypt', false), 'key'));
const myPublic = await subtle.importKey('raw', unb64(mine.publicKey), { name: 'X25519' }, false, []);
const sharedBits = await subtle.deriveBits({ name: 'X25519', public: myPublic }, theirs.privateKey, 256);
results.push('agreed-opens:' + await decrypted('agreed', await sealWith(await hkdfKey(sharedBits, salt, info), 'agreed text', aad), aad));
results.push('small-order:' + word(await call('agree', 'agree-key', b64(new Uint8Array(32)), '', '', 'never', 'encrypt', false), 'key'));
results.push('derive-base:' + word(await call('importKey', 'base', b64(secret), 'derive', false), 'key'));
results.push('derive:' + word(await call('deriveKey', 'base', b64(salt), b64(info), 'child', 'encrypt', false), 'key'));
results.push('child-opens:' + await decrypted('child', await sealWith(await hkdfKey(secret, salt, info), 'child text', aad), aad));
const codeSalt = crypto.getRandomValues(new Uint8Array(16));
results.push('code:' + word(await call('fromCode', 'backup', 'correct horse battery', b64(codeSalt), 'encrypt', keep), 'key'));
const codeBase = await subtle.importKey('raw', new TextEncoder().encode('correct horse battery'), 'PBKDF2', false, ['deriveBits']);
const codeBits = await subtle.deriveBits({ name: 'PBKDF2', hash: 'SHA-256', salt: codeSalt, iterations: 600000 }, codeBase, 256);
const codeKey = await subtle.importKey('raw', codeBits, 'AES-GCM', false, ['encrypt', 'decrypt']);
results.push('code-opens:' + await decrypted('backup', await sealWith(codeKey, 'backup text', aad), aad));
localStorage.setItem('backup', (await call('encrypt', 'backup', text('kept text'), b64(aad))).sealed);
results.push('unknown:' + word(await call('publicKey', 'missing'), 'key'));
results.push('exists:' + word(await call('makeKeyPair', 'device', 'ed25519', false), 'key'));
results.push('wrong-kind:' + word(await call('sign', 'agree-key', text('x')), 'signature'));
results.push('forget-missing:' + word(await call('forget', 'missing'), 'nothing'));
return results.join(',');";
    pub(super) const AGAIN: &'static str = r"const aad = new TextEncoder().encode('chat-1');
results.push('store:' + (await call('check')).keyStore);
results.push('public-kept:' + ((await call('publicKey', 'device')).publicKey === localStorage.getItem('device')));
results.push('backup-opens:' + await decrypted('backup', localStorage.getItem('backup'), aad));
results.push('memory-gone:' + word(await call('publicKey', 'agree-key'), 'key'));
results.push('exists:' + word(await call('makeKeyPair', 'device', 'ed25519', false), 'key'));
results.push('forget-all:' + word(await call('forgetAll'), 'nothing'));
results.push('after:' + word(await call('publicKey', 'device'), 'key'));
return results.join(',');";
    pub(super) const FORGET: &'static str = r"results.push('forget-all:' + word(await call('forgetAll'), 'nothing'));
results.push('after:' + word(await call('publicKey', 'device'), 'key'));
return results.join(',');";
    pub(super) const FRAME: &'static str = concat!(
        include_str!("../template/tac-media-frames.js"),
        r"
const secret = new Uint8Array(32).fill(5);
const frameBits = async (label, bits) => {
  const base = await subtle.importKey('raw', secret, 'HKDF', false, ['deriveBits']);
  const info = new Uint8Array([...new TextEncoder().encode(label), 0, 0, 0, 0, 0, 0, 0, 7, 0, 4]);
  return b64(new Uint8Array(await subtle.deriveBits({ name: 'HKDF', hash: 'SHA-256', salt: new Uint8Array(0), info }, base, bits)));
};
await call('forget', 'frame-secret');
await call('forget', 'frame-encrypt');
results.push('frame-import:' + word(await call('importKey', 'frame-secret', b64(secret), 'derive', false), 'key'));
const pair = await subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, false, ['deriveBits']);
const workerKey = b64(new Uint8Array(await subtle.exportKey('raw', pair.publicKey)));
const wrapped = await call('frameKey', 'frame-secret', 7, workerKey);
results.push('frame-fields:' + ['enc', 'key', 'salt'].every((field) => typeof wrapped[field] === 'string'));
const plainParts = [b64(secret), await frameBits('SFrame 1.0 Secret key ', 128), await frameBits('SFrame 1.0 Secret salt ', 96)];
results.push('frame-opaque:' + !plainParts.some((part) => JSON.stringify(wrapped).includes(part)));
const entry = await TacMediaFrames.unwrapped(pair, 7n, wrapped).catch(() => null);
results.push('frame-unwrapped:' + (entry !== null));
const page = await TacMediaFrames.derive(await subtle.importKey('raw', secret, 'HKDF', false, ['deriveKey', 'deriveBits']), 7n);
const metadata = new Uint8Array([1, 2, 3]);
const plain = new Uint8Array([9, 8, 7, 6]);
results.push('frame-same:' + (b64(await TacMediaFrames.encrypt(entry, metadata, plain)) === b64(await TacMediaFrames.encrypt(page, metadata, plain))));
results.push('frame-extractable:' + entry.key.extractable);
results.push('frame-unknown:' + word(await call('frameKey', 'frame-nobody', 7, workerKey), 'enc'));
await call('importKey', 'frame-encrypt', b64(secret), 'encrypt', false);
results.push('frame-wrong-use:' + word(await call('frameKey', 'frame-encrypt', 7, workerKey), 'enc'));
results.push('frame-bad-worker-key:' + word(await call('frameKey', 'frame-secret', 7, b64(new Uint8Array(65))), 'enc'));
results.push('frame-bad-key-id:' + word(await call('frameKey', 'frame-secret', -1, workerKey), 'enc'));
await call('forget', 'frame-secret');
await call('forget', 'frame-encrypt');
return results.join(',');"
    );
    pub(super) const LINUX_FRAME: &'static str = r"results.push('frame-key:' + word(await call('frameKey', 'device', 7, b64(new Uint8Array([4, ...new Uint8Array(64).fill(1)]))), 'enc'));
return results.join(',');";
    pub(super) const LINUX_FRAME_PASSED: &'static str = "frame-key:unsupported";
    const FRAME_PASSED: &'static str = "frame-import:ok,frame-fields:true,frame-opaque:true,frame-unwrapped:true,frame-same:true,frame-extractable:false,frame-unknown:unknown,frame-wrong-use:invalid,frame-bad-worker-key:invalid,frame-bad-key-id:invalid";
    const FIRST_PASSED: &'static str = "hash,hkdf,rfc8032:true,rfc8032-empty:true,changed:false,webkit-verify:true,empty-round-trip:true,import:ok,host-opens:from webkit,webkit-opens:from host,wrong-aad:rejected,agree:ok,agreed-opens:agreed text,small-order:invalid,derive-base:ok,derive:ok,child-opens:child text,code:ok,code-opens:backup text,unknown:unknown,exists:exists,wrong-kind:invalid,forget-missing:missing";
    const AGAIN_PASSED: &'static str = "public-kept:true,backup-opens:kept text,memory-gone:unknown,exists:exists,forget-all:missing,after:unknown";
    const FORGET_PASSED: &'static str = "forget-all:missing,after:unknown";
    const STOPPED: &'static str = "\n} catch (error) {\n  return results.concat('stopped: ' + error.message + ' after ' + last).join(',');\n}";

    pub(super) fn script(body: &str) -> String {
        [Self::HELPERS, "try {\n", body, Self::STOPPED].concat()
    }

    pub(super) fn first_expected(store: &str) -> String {
        let keep = ["unsupported", "os"][usize::from(store == "os")];
        format!("store:{store},keep:{keep},{}", Self::FIRST_PASSED)
    }

    pub(super) fn frame_expected(_store: &str) -> String {
        String::from(Self::FRAME_PASSED)
    }

    pub(super) fn forget_expected(_store: &str) -> String {
        String::from(Self::FORGET_PASSED)
    }

    pub(super) fn again_expected(store: &str) -> String {
        format!("store:{store},{}", Self::AGAIN_PASSED)
    }
}
