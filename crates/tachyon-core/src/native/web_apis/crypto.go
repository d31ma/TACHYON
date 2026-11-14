package tac

type Crypto struct{}

func (Crypto) Supported() (bool, error) {
	value := false
	err := Web.Call("crypto", "supported", nil, &value)
	return value, err
}

func (Crypto) request(call string, arguments []any) (string, error) {
	request := ""
	err := Web.Call("crypto", call, arguments, &request)
	return request, err
}

func (crypto Crypto) Check() (string, error) {
	return crypto.request("check", nil)
}

func (crypto Crypto) MakeKeyPair(key string, algorithm string, keep bool) (string, error) {
	return crypto.request("makeKeyPair", []any{key, algorithm, keep})
}

func (crypto Crypto) PublicKey(key string) (string, error) {
	return crypto.request("publicKey", []any{key})
}

func (crypto Crypto) Sign(key string, data string) (string, error) {
	return crypto.request("sign", []any{key, data})
}

func (crypto Crypto) Verify(publicKey string, data string, signature string) (string, error) {
	return crypto.request("verify", []any{publicKey, data, signature})
}

func (crypto Crypto) Agree(key string, peerPublicKey string, salt string, info string, newKey string, use string, keep bool) (string, error) {
	return crypto.request("agree", []any{key, peerPublicKey, salt, info, newKey, use, keep})
}

func (crypto Crypto) DeriveKey(fromKey string, salt string, info string, newKey string, use string, keep bool) (string, error) {
	return crypto.request("deriveKey", []any{fromKey, salt, info, newKey, use, keep})
}

func (crypto Crypto) ImportKey(newKey string, secret string, use string, keep bool) (string, error) {
	return crypto.request("importKey", []any{newKey, secret, use, keep})
}

func (crypto Crypto) FromCode(newKey string, code string, salt string, use string, keep bool) (string, error) {
	return crypto.request("fromCode", []any{newKey, code, salt, use, keep})
}

func (crypto Crypto) Encrypt(key string, data string, aad string) (string, error) {
	return crypto.request("encrypt", []any{key, data, aad})
}

func (crypto Crypto) Decrypt(key string, sealed string, aad string) (string, error) {
	return crypto.request("decrypt", []any{key, sealed, aad})
}

func (crypto Crypto) Hkdf(input string, salt string, info string, length int) (string, error) {
	return crypto.request("hkdf", []any{input, salt, info, length})
}

func (crypto Crypto) Hash(data string) (string, error) {
	return crypto.request("hash", []any{data})
}

func (crypto Crypto) Forget(key string) (string, error) {
	return crypto.request("forget", []any{key})
}

func (crypto Crypto) ForgetAll() (string, error) {
	return crypto.request("forgetAll", nil)
}
