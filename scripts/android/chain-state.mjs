#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { loadavg } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

class AndroidChainState {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static sdk = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT ?? '';
  static adb = path.join(AndroidChainState.sdk, 'platform-tools', process.platform === 'win32' ? 'adb.exe' : 'adb');
  static root = process.env.TAC_ANDROID_CHAIN_STATE ?? path.join(AndroidChainState.repo, 'target/android-chain-state');
  static pointer = path.join(AndroidChainState.root, 'current');
  static settles = process.env.TAC_ANDROID_CHAIN_SETTLE !== '0';
  static gatePackages = /^(dev\.tachyon\.|com\.example\.catalog$)/;
  static strayTools = /uiautomator|AccessibilityProbe|app_process \/system\/bin/;
  static errorWindows = /Application Not Responding|Application Error|isn't responding|keeps stopping|has stopped/i;
  static keptSettings = [['global', 'stay_on_while_plugged_in'], ['system', 'screen_off_timeout'], ['system', 'accelerometer_rotation'], ['system', 'user_rotation']];
  static awakeMilliseconds = '1800000';
  static settleAttempts = 40;
  static pauseMilliseconds = 500;
  static routineActions = ['wake the screen', 'dismiss the lock screen', 'close system dialogs', 'go home'];

  static adbOutput(args, encoding = 'utf8') {
    const result = spawnSync(AndroidChainState.adb, args, { encoding, timeout: 60000, maxBuffer: 64 * 1024 * 1024 });
    return result.status === 0 ? result.stdout : (encoding === 'utf8' ? '' : Buffer.alloc(0));
  }

  static adbText(...args) {
    return AndroidChainState.adbOutput(args).trim();
  }

  static shell(...args) {
    return AndroidChainState.adbText('shell', ...args);
  }

  static pause() {
    return new Promise(resolve => setTimeout(resolve, AndroidChainState.pauseMilliseconds));
  }

  static run() {
    return existsSync(AndroidChainState.pointer) ? readFileSync(AndroidChainState.pointer, 'utf8').trim() : AndroidChainState.root;
  }

  static setting(namespace, name) {
    return AndroidChainState.shell('settings', 'get', namespace, name);
  }

  static windowName(block) {
    return block.match(/Window\{\S+ \S+ ([^}]+)\}/)?.[1] ?? '';
  }

  static visibleWindows() {
    const blocks = AndroidChainState.shell('dumpsys', 'window', 'windows').split(/\n(?=\s*Window #)/).slice(1);
    return blocks.filter(block => /isVisible=true|isOnScreen=true/.test(block)).map(AndroidChainState.windowName).filter(Boolean);
  }

  static focusedWindow() {
    return AndroidChainState.windowName(AndroidChainState.shell('dumpsys', 'window').match(/mCurrentFocus=[^\n]*/)?.[0] ?? '');
  }

  static screenshot(file) {
    const image = AndroidChainState.adbOutput(['exec-out', 'screencap', '-p'], 'buffer');
    if (image.length < 24) return { width: 0, height: 0 };
    if (file) writeFileSync(file, image);
    return { width: image.readUInt32BE(16), height: image.readUInt32BE(20) };
  }

  static gateProcesses() {
    return AndroidChainState.shell('ps', '-A', '-o', 'PID,NAME').split('\n').slice(1).map(line => line.trim().split(/\s+/))
      .filter(([, name]) => AndroidChainState.gatePackages.test(name ?? '')).map(([pid, name]) => ({ pid, name }));
  }

  static strayToolProcesses() {
    return AndroidChainState.shell('ps', '-A', '-o', 'PID,ARGS').split('\n').slice(1).map(line => line.trim())
      .filter(line => AndroidChainState.strayTools.test(line)).map(line => ({ pid: line.split(/\s+/)[0], args: line.slice(line.indexOf(' ') + 1) }));
  }

  static settings() {
    return Object.fromEntries(AndroidChainState.keptSettings.map(([namespace, name]) => [name, AndroidChainState.setting(namespace, name)]));
  }

  static snapshot(label) {
    const state = AndroidChainState;
    const folder = state.run();
    const windows = state.visibleWindows();
    const activities = state.shell('dumpsys', 'activity', 'activities');
    return {
      label,
      time: new Date().toISOString(),
      hostLoad: loadavg().map(value => Number(value.toFixed(2))),
      deviceLoad: state.shell('cat', '/proc/loadavg'),
      wakefulness: state.shell('dumpsys', 'power').match(/mWakefulness=(\w+)/)?.[1] ?? 'unknown',
      keyguard: activities.match(/keyguardShowing=(\w+)/i)?.[1] ?? 'unknown',
      focus: state.focusedWindow(),
      resumed: activities.split('\n').map(line => line.trim()).filter(line => line.startsWith('topResumedActivity=')),
      windows,
      keyboard: windows.some(name => name === 'InputMethod'),
      screen: state.screenshot(path.join(folder, `${label}.png`)),
      settings: state.settings(),
      airplane: state.setting('global', 'airplane_mode_on'),
      mockLocation: state.shell('appops', 'get', 'com.android.shell', 'android:mock_location'),
      forwards: state.adbText('forward', '--list').split('\n').filter(line => line.includes('webview_devtools_remote')),
      reverses: state.adbText('reverse', '--list').split('\n').filter(Boolean),
      gateProcesses: state.gateProcesses(),
      strayTools: state.strayToolProcesses(),
      notifications: state.shell('dumpsys', 'notification').split('\n').filter(line => /NotificationRecord\(/.test(line) && /pkg=(dev\.tachyon\.|com\.example\.catalog)/.test(line)).length,
      crashes: state.adbText('logcat', '-d', '-b', 'crash', '-t', '40').split('\n').filter(Boolean).slice(-40),
      notResponding: state.adbText('logcat', '-d', '-s', 'ActivityManager:E', '-t', '400').split('\n').filter(line => /ANR in/.test(line)).slice(-10),
    };
  }

  static leftovers(state, baseline) {
    const found = [];
    const add = (condition, text) => { if (condition) found.push(text); };
    add(state.wakefulness !== 'Awake', `the screen is ${state.wakefulness}`);
    add(state.keyguard === 'true', 'the lock screen shows');
    add(baseline.home && state.focus !== baseline.home, `focus is on ${state.focus || 'nothing'}, not ${baseline.home}`);
    add(state.keyboard, 'the soft keyboard is open');
    add(state.windows.some(name => AndroidChainState.errorWindows.test(name)), `an error dialog shows: ${state.windows.filter(name => AndroidChainState.errorWindows.test(name)).join(', ')}`);
    add(state.screen.width > state.screen.height, `the screen is landscape (${state.screen.width}x${state.screen.height})`);
    for (const name of ['accelerometer_rotation', 'user_rotation']) add(state.settings[name] !== baseline.settings[name], `${name} is ${state.settings[name]}, was ${baseline.settings[name]}`);
    add(state.airplane === '1', 'airplane mode is on');
    add(/allow/.test(state.mockLocation), `the shell may mock locations: ${state.mockLocation}`);
    add(state.forwards.length > 0, `WebView DevTools forwards remain: ${state.forwards.join(' | ')}`);
    add(state.reverses.length > 0, `adb reverse routes remain: ${state.reverses.join(' | ')}`);
    add(state.gateProcesses.length > 0, `gate apps still run: ${state.gateProcesses.map(entry => entry.name).join(', ')}`);
    add(state.strayTools.length > 0, `UI tools still run: ${state.strayTools.map(entry => entry.args).join(' | ')}`);
    add(state.notifications > 0, `${state.notifications} gate notification(s) remain`);
    return found;
  }

  static async settle(state, baseline) {
    const chain = AndroidChainState;
    const actions = [];
    const act = (text, ...args) => { actions.push(text); chain.shell(...args); };
    act('wake the screen', 'input', 'keyevent', 'KEYCODE_WAKEUP');
    act('dismiss the lock screen', 'wm', 'dismiss-keyguard');
    for (const tool of state.strayTools) act(`stop ${tool.args}`, 'kill', tool.pid);
    for (const entry of state.gateProcesses) act(`force-stop ${entry.name}`, 'am', 'force-stop', entry.name);
    for (const line of state.forwards) {
      const local = line.split(/\s+/)[1];
      actions.push(`remove forward ${local}`);
      chain.adbText('forward', '--remove', local);
    }
    if (state.reverses.length) {
      actions.push('remove every adb reverse route');
      chain.adbText('reverse', '--remove-all');
    }
    if (state.airplane === '1') act('turn airplane mode off', 'cmd', 'connectivity', 'airplane-mode', 'disable');
    if (/allow/.test(state.mockLocation)) act('reset the mock location permission', 'appops', 'set', 'com.android.shell', 'android:mock_location', 'default');
    for (const name of ['accelerometer_rotation', 'user_rotation']) {
      if (state.settings[name] !== baseline.settings[name]) act(`put ${name} back to ${baseline.settings[name]}`, 'settings', 'put', 'system', name, baseline.settings[name]);
    }
    act('close system dialogs', 'am', 'broadcast', '-a', 'android.intent.action.CLOSE_SYSTEM_DIALOGS');
    act('go home', 'input', 'keyevent', 'KEYCODE_HOME');
    let settled = {};
    for (let attempt = 0; attempt < chain.settleAttempts; attempt++) {
      settled = { focus: chain.focusedWindow(), screen: chain.screenshot(), keyboard: chain.visibleWindows().includes('InputMethod') };
      if (settled.screen.height > settled.screen.width && (!baseline.home || settled.focus === baseline.home) && !settled.keyboard) break;
      await chain.pause();
    }
    return { actions, settled };
  }

  static async prepare() {
    const chain = AndroidChainState;
    const folder = path.join(chain.root, new Date().toISOString().replaceAll(':', '-'));
    mkdirSync(folder, { recursive: true });
    writeFileSync(chain.pointer, folder);
    chain.adbText('wait-for-device');
    const settings = chain.settings();
    writeFileSync(path.join(folder, 'kept-settings.json'), JSON.stringify(settings, null, 2) + '\n');
    chain.shell('svc', 'power', 'stayon', 'true');
    chain.shell('settings', 'put', 'system', 'screen_off_timeout', chain.awakeMilliseconds);
    const before = chain.snapshot('00-before-chain');
    const { actions, settled } = await chain.settle(before, { settings });
    const baseline = { settings, home: settled.focus };
    writeFileSync(path.join(folder, 'baseline.json'), JSON.stringify(baseline, null, 2) + '\n');
    writeFileSync(path.join(folder, '00-before-chain.json'), JSON.stringify({ ...before, settle: actions, settled }, null, 2) + '\n');
    console.log(`Android chain state: ${folder}; home is ${baseline.home || 'unknown'}; screen kept awake for the chain`);
  }

  static async between(step, status, seconds) {
    const chain = AndroidChainState;
    const folder = chain.run();
    const baseline = JSON.parse(readFileSync(path.join(folder, 'baseline.json'), 'utf8'));
    const index = readFileSync(path.join(folder, 'steps.jsonl'), { encoding: 'utf8', flag: 'a+' }).split('\n').filter(Boolean).length + 1;
    const label = `${String(index).padStart(2, '0')}-${path.basename(step, '.mjs')}`;
    const state = chain.snapshot(label);
    const leftovers = chain.leftovers(state, baseline);
    const settled = chain.settles ? await chain.settle(state, baseline) : { actions: [], settled: null };
    writeFileSync(path.join(folder, `${label}.json`), JSON.stringify({ ...state, step, status: Number(status), seconds: Number(seconds), leftovers, settle: settled.actions, settled: settled.settled }, null, 2) + '\n');
    appendFileSync(path.join(folder, 'steps.jsonl'), JSON.stringify({ step, status: Number(status), seconds: Number(seconds), leftovers, hostLoad: state.hostLoad, deviceLoad: state.deviceLoad }) + '\n');
    const repairs = settled.actions.filter(action => !chain.routineActions.includes(action));
    console.log(`Android chain state after ${step} (exit ${status}, ${seconds} s, host load ${state.hostLoad.join(' ')}): ${leftovers.length ? `left ${leftovers.join('; ')}` : 'nothing left behind'}${repairs.length ? `; settled: ${repairs.join(', ')}` : ''}`);
  }

  static restore() {
    const chain = AndroidChainState;
    const folder = chain.run();
    const kept = path.join(folder, 'kept-settings.json');
    if (existsSync(kept)) {
      const settings = JSON.parse(readFileSync(kept, 'utf8'));
      for (const [namespace, name] of chain.keptSettings) {
        if (/^\d+$/.test(settings[name] ?? '')) chain.shell('settings', 'put', namespace, name, settings[name]);
      }
    }
    const steps = path.join(folder, 'steps.jsonl');
    const rows = existsSync(steps) ? readFileSync(steps, 'utf8').split('\n').filter(Boolean).map(line => JSON.parse(line)) : [];
    const table = ['| Step | Exit | Seconds | Left behind |', '| --- | --- | --- | --- |', ...rows.map(row => `| ${row.step} | ${row.status} | ${row.seconds} | ${row.leftovers.join('; ') || 'nothing'} |`)];
    writeFileSync(path.join(folder, 'SUMMARY.md'), `# Android chain ${path.basename(folder)}\n\n${table.join('\n')}\n`);
    console.log(`Android chain summary: ${path.join(folder, 'SUMMARY.md')}\n${table.join('\n')}`);
  }

  static async main(command, ...args) {
    const chain = AndroidChainState;
    try {
      if (command === 'prepare') await chain.prepare();
      else if (command === 'between') await chain.between(...args);
      else if (command === 'restore') chain.restore();
      else throw new Error(`usage: chain-state.mjs prepare | between <step> <exit> <seconds> | restore (got ${command})`);
    } catch (error) {
      console.error(`Android chain state ${command} could not finish: ${error.stack ?? error}`);
      process.exitCode = command === 'prepare' ? 1 : 0;
    }
  }
}

await AndroidChainState.main(...process.argv.slice(2));
