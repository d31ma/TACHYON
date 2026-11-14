class Crypto
  API = "crypto"

  def supported
    TacWebHost.call(API, "supported") == true
  end

  def check
    request("check", [])
  end

  def make_key_pair(key, algorithm, keep)
    request("makeKeyPair", [key, algorithm, keep == true])
  end

  def public_key(key)
    request("publicKey", [key])
  end

  def sign(key, data)
    request("sign", [key, data])
  end

  def verify(public_key, data, signature)
    request("verify", [public_key, data, signature])
  end

  def agree(key, peer_public_key, salt, info, new_key, use, keep)
    request("agree", [key, peer_public_key, salt, info, new_key, use, keep == true])
  end

  def derive_key(from_key, salt, info, new_key, use, keep)
    request("deriveKey", [from_key, salt, info, new_key, use, keep == true])
  end

  def import_key(new_key, secret, use, keep)
    request("importKey", [new_key, secret, use, keep == true])
  end

  def from_code(new_key, code, salt, use, keep)
    request("fromCode", [new_key, code, salt, use, keep == true])
  end

  def encrypt(key, data, aad)
    request("encrypt", [key, data, aad])
  end

  def decrypt(key, sealed, aad)
    request("decrypt", [key, sealed, aad])
  end

  def hkdf(input, salt, info, length)
    request("hkdf", [input, salt, info, length])
  end

  def hash_data(data)
    request("hash", [data])
  end

  def forget(key)
    request("forget", [key])
  end

  def forget_all
    request("forgetAll", [])
  end

  private

  def request(call, values)
    TacWebHost.call(API, call, values).to_s
  end
end
