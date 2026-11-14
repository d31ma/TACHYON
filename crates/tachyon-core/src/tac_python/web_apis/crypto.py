class Crypto:
    API = 'crypto'

    def supported(self) -> bool:
        return _TacPythonWebHost.call(self.API, 'supported') is True

    def check(self) -> str:
        return self._request('check', ())

    def make_key_pair(self, key: str, algorithm: str, keep: bool) -> str:
        return self._request('makeKeyPair', (key, algorithm, keep))

    def public_key(self, key: str) -> str:
        return self._request('publicKey', (key,))

    def sign(self, key: str, data: str) -> str:
        return self._request('sign', (key, data))

    def verify(self, public_key: str, data: str, signature: str) -> str:
        return self._request('verify', (public_key, data, signature))

    def agree(self, key: str, peer_public_key: str, salt: str, info: str, new_key: str, use: str, keep: bool) -> str:
        return self._request('agree', (key, peer_public_key, salt, info, new_key, use, keep))

    def derive_key(self, from_key: str, salt: str, info: str, new_key: str, use: str, keep: bool) -> str:
        return self._request('deriveKey', (from_key, salt, info, new_key, use, keep))

    def import_key(self, new_key: str, secret: str, use: str, keep: bool) -> str:
        return self._request('importKey', (new_key, secret, use, keep))

    def from_code(self, new_key: str, code: str, salt: str, use: str, keep: bool) -> str:
        return self._request('fromCode', (new_key, code, salt, use, keep))

    def encrypt(self, key: str, data: str, aad: str) -> str:
        return self._request('encrypt', (key, data, aad))

    def decrypt(self, key: str, sealed: str, aad: str) -> str:
        return self._request('decrypt', (key, sealed, aad))

    def hkdf(self, input_bytes: str, salt: str, info: str, length: int) -> str:
        return self._request('hkdf', (input_bytes, salt, info, length))

    def hash(self, data: str) -> str:
        return self._request('hash', (data,))

    def forget(self, key: str) -> str:
        return self._request('forget', (key,))

    def forget_all(self) -> str:
        return self._request('forgetAll', ())

    def _request(self, call: str, values: tuple) -> str:
        return str(_TacPythonWebHost.call(self.API, call, values))
