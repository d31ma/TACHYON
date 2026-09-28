#[Helper]
class MemoryCacheHelper
  def initialize
    @entries = {}
  end

  # @param key [String]
  # @param execution [Object]
  # @return [Hash]
  def get(key, execution)
    @entries.fetch(key, { "hit" => false })
  end

  # @param key [String]
  # @param value [Object]
  # @param execution [Object]
  # @return [Hash]
  def set(key, value, execution)
    @entries[key] = { "hit" => true, "value" => value, "expiresAt" => execution.expires_at }
  end

  # @param key [String]
  # @param execution [Object]
  # @return [Hash, nil]
  def delete(key, execution)
    @entries.delete(key)
  end

  # @param call [Object]
  # @param execution [Object]
  # @return [Boolean]
  def self.allow(call, execution)
    true
  end
end
