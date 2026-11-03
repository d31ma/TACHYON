class InputService {
  calls = 0;
  /** @param {ProductInput} input */
  async submit(@Check input) {
    this.calls++;
    return { ...input, validated: true };
  }
}

