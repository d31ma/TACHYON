module JSON
  def self.pretty_generate(value) = generate(value, "  ")
end

class Object
  def to_json(*) = JSON.generate(self)
end

module Kernel
  private

  def print(*values)
    values.each { |value| TachyonWeb.host_write(value.to_s) }
    nil
  end

  def puts(*values)
    TachyonWeb.host_write("\n") if values.empty?
    values.each do |value|
      if value.is_a?(Array)
        value.empty? ? TachyonWeb.host_write("\n") : value.flatten.each { |item| puts(item) }
      else
        text = value.nil? ? "" : value.to_s
        TachyonWeb.host_write(text.end_with?("\n") ? text : "#{text}\n")
      end
    end
    nil
  end

  def p(*values)
    values.each { |value| TachyonWeb.host_write("#{value.inspect}\n") }
    values.size <= 1 ? values.first : values
  end
end
