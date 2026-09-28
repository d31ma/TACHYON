#[Helper]
class MemoryCacheHelper
  def initialize
    @entries = {}
  end

  def get(key, execution)
    @entries.fetch(key, { "hit" => false })
  end

  def set(key, value, execution)
    @entries[key] = { "hit" => true, "value" => value, "expiresAt" => execution.expires_at }
  end

  def delete(key, execution)
    @entries.delete(key)
  end

  def self.allow(call, execution)
    true
  end
end
