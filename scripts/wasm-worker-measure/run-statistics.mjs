export class RunStatistics {
  static half = 0.5;
  static warmUpCalls = 20;

  /**
   * @param {number[]} values
   * @param {number} share
   * @returns {number | null}
   */
  static percentile(values, share) {
    const sorted = RunStatistics._numbers(values).sort((left, right) => left - right);
    return RunStatistics._at(sorted, Math.ceil(share * sorted.length) - 1);
  }

  /**
   * @param {Array<number | null>} values
   * @returns {number | null}
   */
  static median(values) {
    return RunStatistics.percentile(RunStatistics._numbers(values), RunStatistics.half);
  }

  /**
   * @param {number[]} values
   * @returns {number | null}
   */
  static mean(values) {
    const numbers = RunStatistics._numbers(values);
    return RunStatistics._average(numbers);
  }

  /**
   * @param {number[]} durations
   * @returns {number[]}
   */
  static withoutWarmUp(durations) {
    return durations.slice(RunStatistics.warmUpCalls);
  }

  /**
   * @param {Array<number | null>} values
   * @returns {number[]}
   */
  static _numbers(values) {
    return /** @type {number[]} */ (values.filter((value) => typeof value === 'number' && Number.isFinite(value)));
  }

  /**
   * @param {number[]} numbers
   * @returns {number | null}
   */
  static _average(numbers) {
    if (!numbers.length) return null;
    return numbers.reduce((total, value) => total + value, 0) / numbers.length;
  }

  /**
   * @param {number[]} sorted
   * @param {number} index
   * @returns {number | null}
   */
  static _at(sorted, index) {
    if (!sorted.length) return null;
    return sorted[Math.max(0, index)];
  }
}
