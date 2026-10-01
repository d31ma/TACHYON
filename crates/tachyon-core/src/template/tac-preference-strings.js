class TacPreferenceStrings {
  static read(key) {
    return TacPreferenceStrings.#stored(key)
  }

  static write(key, value) {
    return TacPreferenceStrings.#saved(key, value)
  }

  static #stored(key) {
    try {
      TacWebApis.allowed('preferenceStrings', false)
      TacPreferenceBounds.key(key)
      const value = localStorage.getItem(key)
      TacPreferenceBounds.value(value)
      return value
    } catch {
      return null
    }
  }

  static #saved(key, value) {
    try {
      TacWebApis.allowed('preferenceStrings', false)
      TacPreferenceBounds.key(key)
      TacPreferenceBounds.value(value)
      localStorage.setItem(key, value)
      return true
    } catch {
      return false
    }
  }
}
