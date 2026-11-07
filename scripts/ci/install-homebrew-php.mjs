import { execFileSync } from 'node:child_process';
import { appendFileSync, copyFileSync } from 'node:fs';
import { join } from 'node:path';

class HomebrewPhp {
  static usage = 'Usage: node scripts/ci/install-homebrew-php.mjs <major.minor>';
  static versionPattern = /^[0-9]+\.[0-9]+$/;
  static attempts = 3;
  static attemptMilliseconds = 6 * 60 * 1000;
  static pauseSeconds = '10';
  static quietHomebrew = {
    HOMEBREW_NO_AUTO_UPDATE: '1',
    HOMEBREW_NO_INSTALL_CLEANUP: '1',
    HOMEBREW_NO_INSTALLED_DEPENDENTS_CHECK: '1',
    HOMEBREW_NO_ENV_HINTS: '1',
  };

  /** @param {string[]} argumentsList */
  static run(argumentsList) {
    const version = HomebrewPhp.#version(argumentsList);
    const formula = `php@${version}`;
    HomebrewPhp.#requireInstalled(HomebrewPhp.#installWithRetries(formula), formula);
    const prefix = HomebrewPhp.#brew(['--prefix', formula]).trim();
    HomebrewPhp.#useProductionSettings(version);
    HomebrewPhp.#addToPath(join(prefix, 'bin'));
    HomebrewPhp.#addToPath(join(prefix, 'sbin'));
    HomebrewPhp.#requireVersion(join(prefix, 'bin', 'php'), version);
    HomebrewPhp.#requireAnswer(join(prefix, 'bin', 'php-cgi'));
  }

  /**
   * @param {string[]} argumentsList
   * @returns {string}
   */
  static #version(argumentsList) {
    const [version] = argumentsList;
    if (argumentsList.length !== 1 || !HomebrewPhp.versionPattern.test(version)) throw new Error(HomebrewPhp.usage);
    return version;
  }

  /**
   * @param {string} formula
   * @returns {boolean}
   */
  static #installWithRetries(formula) {
    const attempts = Array.from({ length: HomebrewPhp.attempts }, (_, index) => index + 1);
    return attempts.some((attempt) => HomebrewPhp.#installOnce(formula, attempt));
  }

  /**
   * @param {boolean} installed
   * @param {string} formula
   */
  static #requireInstalled(installed, formula) {
    if (!installed) throw new Error(`brew install ${formula} failed ${HomebrewPhp.attempts} times; PHP is not installed`);
  }

  /**
   * @param {string} formula
   * @param {number} attempt
   * @returns {boolean}
   */
  static #installOnce(formula, attempt) {
    console.log(`brew install ${formula}, attempt ${attempt} of ${HomebrewPhp.attempts}, at most ${HomebrewPhp.attemptMilliseconds / 60000} minutes`);
    try {
      execFileSync('brew', ['install', formula], {
        stdio: 'inherit',
        timeout: HomebrewPhp.attemptMilliseconds,
        killSignal: 'SIGKILL',
        env: { ...process.env, ...HomebrewPhp.quietHomebrew },
      });
      return true;
    } catch (error) {
      console.log(`brew install ${formula} attempt ${attempt} failed: ${HomebrewPhp.#message(error)}`);
      execFileSync('sleep', [HomebrewPhp.pauseSeconds]);
      return false;
    }
  }

  /**
   * @param {unknown} error
   * @returns {string}
   */
  static #message(error) {
    return error instanceof Error ? error.message : String(error);
  }

  /** @param {string} version */
  static #useProductionSettings(version) {
    const settings = join(HomebrewPhp.#brew(['--prefix']).trim(), 'etc', 'php', version);
    copyFileSync(join(settings, 'php.ini-production'), join(settings, 'php.ini'));
    console.log(`${join(settings, 'php.ini')} <- php.ini-production`);
  }

  /** @param {string} folder */
  static #addToPath(folder) {
    if (process.env.GITHUB_PATH) appendFileSync(process.env.GITHUB_PATH, `${folder}\n`);
    console.log(`PATH += ${folder}`);
  }

  /**
   * @param {string} php
   * @param {string} version
   */
  static #requireVersion(php, version) {
    const found = execFileSync(php, ['-r', 'echo PHP_MAJOR_VERSION . "." . PHP_MINOR_VERSION;'], { encoding: 'utf8' });
    if (found !== version) throw new Error(`${php} is PHP ${found}, not ${version}`);
    console.log(execFileSync(php, ['--version'], { encoding: 'utf8' }).split('\n')[0]);
  }

  /** @param {string} program */
  static #requireAnswer(program) {
    console.log(execFileSync(program, ['--version'], { encoding: 'utf8' }).split('\n')[0]);
  }

  /**
   * @param {string[]} argumentsList
   * @returns {string}
   */
  static #brew(argumentsList) {
    return execFileSync('brew', argumentsList, { encoding: 'utf8', env: { ...process.env, ...HomebrewPhp.quietHomebrew } });
  }
}

HomebrewPhp.run(process.argv.slice(2));
