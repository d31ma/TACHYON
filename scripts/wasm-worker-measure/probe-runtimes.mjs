export class ProbeRuntimes {
  static all = [
    {
      key: 'js',
      label: 'JavaScript',
      className: 'JsProbeService',
      service: 'jsProbe',
      file: 'client/services/JsProbeService.js',
      source: `@Service
export default class JsProbeService {
  /** @type {number} */
  count = 0;

  /** @returns {number} */
  increment() {
    this.count += 1;
    return this.count;
  }

  /**
   * @param {number} rounds
   * @returns {number}
   */
  spin(rounds) {
    return this._mix(rounds);
  }

  /**
   * @param {number} rounds
   * @returns {number}
   */
  _mix(rounds) {
    let total = 0;
    for (let index = 0; index < rounds; index += 1) {
      total = (total * 31 + index % 1000) % 1000003;
    }
    return total;
  }
}
`,
    },
    {
      key: 'rust',
      label: 'Rust',
      className: 'RustProbeService',
      service: 'rustProbe',
      file: 'client/services/RustProbeService.rs',
      source: `#[Service]
#[derive(Default)]
struct RustProbeService {
    count: i64,
}

impl RustProbeService {
    pub fn increment(&mut self) -> i64 {
        self.count += 1;
        self.count
    }

    pub fn spin(&self, rounds: i64) -> i64 {
        Self::mix(rounds)
    }

    fn mix(rounds: i64) -> i64 {
        let mut total: i64 = 0;
        for index in 0..rounds {
            total = (total * 31 + index % 1000) % 1_000_003;
        }
        total
    }
}
`,
    },
    {
      key: 'go',
      label: 'Go (TinyGo)',
      className: 'GoProbeService',
      service: 'goProbe',
      file: 'client/services/GoProbeService.go',
      source: `// @Service
type GoProbeService struct {
\tCount int
}

func (probe *GoProbeService) Increment() int {
\tprobe.Count++
\treturn probe.Count
}

func (probe *GoProbeService) Spin(rounds int) int {
\treturn probe.mix(rounds)
}

func (probe *GoProbeService) mix(rounds int) int {
\ttotal := 0
\tfor index := 0; index < rounds; index++ {
\t\ttotal = (total*31 + index%1000) % 1000003
\t}
\treturn total
}
`,
    },
    {
      key: 'csharp',
      label: 'C# (.NET)',
      className: 'CsProbeService',
      service: 'csProbe',
      file: 'client/services/CsProbeService.cs',
      source: `[Service]
public sealed class CsProbeService {
    public int count = 0;

    public int increment() {
        count++;
        return count;
    }

    public int spin(int rounds) {
        return Mix(rounds);
    }

    private int Mix(int rounds) {
        int total = 0;
        for (int index = 0; index < rounds; index++) {
            total = (total * 31 + index % 1000) % 1000003;
        }
        return total;
    }
}
`,
    },
    {
      key: 'fsharp',
      label: 'F# (.NET)',
      className: 'FsProbeService',
      service: 'fsProbe',
      file: 'client/services/FsProbeService.fs',
      source: `[<Service>]
type FsProbeService() =
    member val Count = 0 with get, set

    member this.Increment() : int =
        this.Count <- this.Count + 1
        this.Count

    member this.Spin(rounds: int) : int =
        this.Mix(rounds)

    member private this.Mix(rounds: int) : int =
        let mutable total = 0
        for index in 0 .. rounds - 1 do
            total <- (total * 31 + index % 1000) % 1000003
        total
`,
    },
    {
      key: 'dart',
      label: 'Dart',
      className: 'DartProbeService',
      service: 'dartProbe',
      file: 'client/services/DartProbeService.dart',
      source: `@Service
class DartProbeService {
  int count = 0;

  int increment() {
    count += 1;
    return count;
  }

  int spin(int rounds) => _mix(rounds);

  int _mix(int rounds) {
    var total = 0;
    for (var index = 0; index < rounds; index++) {
      total = (total * 31 + index % 1000) % 1000003;
    }
    return total;
  }
}
`,
    },
    {
      key: 'ruby',
      label: 'Ruby (mruby)',
      className: 'RubyProbeService',
      service: 'rubyProbe',
      file: 'client/services/RubyProbeService.rb',
      source: `#[Service]
class RubyProbeService
  # @return [Integer]
  attr_reader :count

  def initialize
    @count = 0
  end

  # @return [Integer]
  def increment
    @count += 1
  end

  # @param rounds [Integer]
  # @return [Integer]
  def spin(rounds)
    mix(rounds)
  end

  private

  # @param rounds [Integer]
  # @return [Integer]
  def mix(rounds)
    total = 0
    index = 0
    while index < rounds
      total = (total * 31 + index % 1000) % 1_000_003
      index += 1
    end
    total
  end
end
`,
    },
    {
      key: 'kotlin',
      label: 'Kotlin',
      className: 'KtProbeService',
      service: 'ktProbe',
      file: 'client/services/KtProbeService.kt',
      source: `@Service
class KtProbeService {
    var count: Int = 0

    fun increment(): Int {
        count += 1
        return count
    }

    fun spin(rounds: Int): Int = mix(rounds)

    private fun mix(rounds: Int): Int {
        var total = 0
        for (index in 0 until rounds) {
            total = (total * 31 + index % 1000) % 1000003
        }
        return total
    }
}
`,
    },
    {
      key: 'swift',
      label: 'Swift',
      className: 'SwiftProbeService',
      service: 'swiftProbe',
      file: 'client/services/SwiftProbeService.swift',
      source: `@Service
final class SwiftProbeService {
    var count: Int = 0

    func increment() -> Int {
        count += 1
        return count
    }

    func spin(_ rounds: Int) -> Int {
        return mix(rounds)
    }

    private func mix(_ rounds: Int) -> Int {
        var total = 0
        var index = 0
        while index < rounds {
            total = (total * 31 + index % 1000) % 1000003
            index += 1
        }
        return total
    }
}
`,
    },
  ];

  static engines = [
    { label: 'Rust', pattern: /tachyon-rust\.js/ },
    { label: 'Go', pattern: /tachyon-go\.js|services\/go\// },
    { label: '.NET', pattern: /tachyon-csharp\.js|services\/dotnet\// },
    { label: 'Dart', pattern: /tachyon-dart/ },
    { label: 'Ruby', pattern: /tachyon-ruby|services\/ruby\// },
    { label: 'Kotlin', pattern: /tachyon-kotlin\.js|services\/kotlin\// },
    { label: 'Swift', pattern: /tachyon-swift\.js|services\/swift\// },
  ];

  static otherEngine = 'page JavaScript';

  /**
   * @param {string[]} keys
   * @returns {typeof ProbeRuntimes.all}
   */
  static chosen(keys) {
    return ProbeRuntimes.all.filter((runtime) => keys.includes(runtime.key));
  }

  /**
   * @param {string} url
   * @returns {string}
   */
  static engineOf(url) {
    return ProbeRuntimes._labelOf(ProbeRuntimes.engines.find((candidate) => candidate.pattern.test(url)));
  }

  /**
   * @param {{label: string} | undefined} engine
   * @returns {string}
   */
  static _labelOf(engine) {
    return engine?.label ?? ProbeRuntimes.otherEngine;
  }
}
