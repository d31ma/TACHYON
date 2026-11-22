#!/usr/bin/env node
import assert from 'node:assert/strict'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { afterEach, test } from 'node:test'
import { SharedSimulator } from './shared-simulator.mjs'

class SharedSimulatorTest {
  static PHONE = '11111111-2222-3333-4444-555555555555'
  static TABLET = '66666666-7777-8888-9999-000000000000'
  static PRIVATE_SET = '/tmp/gate/simulator-devices'
  static NAMES = ['TAC_IOS_DEVICE', 'TAC_IOS_TABLET_DEVICE', 'TAC_IOS_DEVICE_SET', 'TAC_IOS_RUNTIME', 'TAC_IOS_FAMILIES']

  /** @param {string} version @returns {{ identifier: string, version: string }} */
  static runtime(version) {
    return { identifier: `com.apple.CoreSimulator.SimRuntime.iOS-${version.replaceAll('.', '-')}`, version }
  }

  static clear() {
    for (const name of SharedSimulatorTest.NAMES) delete process.env[name]
  }
}

afterEach(() => SharedSimulatorTest.clear())

test('without a shared device a gate keeps its own private device set', () => {
  SharedSimulatorTest.clear()
  assert.equal(SharedSimulator.chosen(), false)
  assert.equal(SharedSimulator.deviceSet(SharedSimulatorTest.PRIVATE_SET), SharedSimulatorTest.PRIVATE_SET)
})

test('a shared iPhone moves the gate to the default device set', () => {
  process.env.TAC_IOS_DEVICE = SharedSimulatorTest.PHONE
  assert.equal(SharedSimulator.udid('iPhone'), SharedSimulatorTest.PHONE)
  assert.equal(SharedSimulator.deviceSet(SharedSimulatorTest.PRIVATE_SET), SharedSimulator.DEFAULT_SET)
})

test('a shared iPad alone also moves the gate to the default device set', () => {
  process.env.TAC_IOS_TABLET_DEVICE = SharedSimulatorTest.TABLET
  assert.equal(SharedSimulator.udid('iPhone'), '')
  assert.equal(SharedSimulator.deviceSet(SharedSimulatorTest.PRIVATE_SET), SharedSimulator.DEFAULT_SET)
})

test('TAC_IOS_DEVICE_SET names the device set', () => {
  process.env.TAC_IOS_DEVICE = SharedSimulatorTest.PHONE
  process.env.TAC_IOS_DEVICE_SET = '/tmp/shared-set'
  assert.equal(SharedSimulator.deviceSet(SharedSimulatorTest.PRIVATE_SET), '/tmp/shared-set')
})

test('only the named devices count as shared', () => {
  process.env.TAC_IOS_DEVICE = SharedSimulatorTest.PHONE
  process.env.TAC_IOS_TABLET_DEVICE = SharedSimulatorTest.TABLET
  assert.equal(SharedSimulator.isShared(SharedSimulatorTest.PHONE), true)
  assert.equal(SharedSimulator.isShared(SharedSimulatorTest.TABLET), true)
  assert.equal(SharedSimulator.isShared('AAAAAAAA-0000-0000-0000-000000000000'), false)
})

test('with no shared device an empty id is not shared', () => {
  assert.equal(SharedSimulator.isShared(''), false)
})

test('a TAC_IOS_RUNTIME version matches that version and its point releases only', () => {
  process.env.TAC_IOS_RUNTIME = '27.2'
  assert.equal(SharedSimulator.wantedRuntime(SharedSimulatorTest.runtime('27.2')), true)
  assert.equal(SharedSimulator.wantedRuntime(SharedSimulatorTest.runtime('27.2.1')), true)
  assert.equal(SharedSimulator.wantedRuntime(SharedSimulatorTest.runtime('27.20')), false)
  assert.equal(SharedSimulator.wantedRuntime(SharedSimulatorTest.runtime('26.5')), false)
})

test('a TAC_IOS_RUNTIME identifier part matches the runtime identifier', () => {
  process.env.TAC_IOS_RUNTIME = '.iOS-18-0'
  assert.equal(SharedSimulator.wantedRuntime(SharedSimulatorTest.runtime('18.0')), true)
  assert.equal(SharedSimulator.wantedRuntime(SharedSimulatorTest.runtime('18.2')), false)
})

test('without TAC_IOS_RUNTIME every runtime is wanted', () => {
  assert.equal(SharedSimulator.wantedRuntime(SharedSimulatorTest.runtime('17.0.1')), true)
})

test('find returns the runtime key and the device with that id', () => {
  const device = { udid: SharedSimulatorTest.PHONE, state: 'Booted', deviceTypeIdentifier: 'phone' }
  const devices = { 'runtime-a': [], 'runtime-b': [device] }
  assert.deepEqual(SharedSimulator.find(devices, SharedSimulatorTest.PHONE), ['runtime-b', device])
  assert.deepEqual(SharedSimulator.find(devices, SharedSimulatorTest.TABLET), [])
})

test('a shared device that is not in the device set fails with its variable name', { skip: process.platform !== 'darwin' }, () => {
  const set = mkdtempSync(path.join(tmpdir(), 'tachyon-shared-simulator-'))
  process.env.TAC_IOS_DEVICE = SharedSimulatorTest.PHONE
  assert.throws(() => SharedSimulator.use(set, 'iPhone'), new RegExp(`TAC_IOS_DEVICE ${SharedSimulatorTest.PHONE} is not in the device set`))
  rmSync(set, { recursive: true, force: true })
})

test('without TAC_IOS_FAMILIES a gate checks the iPhone and then the iPad', () => {
  assert.deepEqual(SharedSimulator.families(), ['iPhone', 'iPad'])
})

test('TAC_IOS_FAMILIES=iPhone checks only the iPhone', () => {
  process.env.TAC_IOS_FAMILIES = 'iPhone'
  assert.deepEqual(SharedSimulator.families(), ['iPhone'])
})

test('TAC_IOS_FAMILIES=iPad checks only the iPad', () => {
  process.env.TAC_IOS_FAMILIES = 'iPad'
  assert.deepEqual(SharedSimulator.families(), ['iPad'])
})

test('TAC_IOS_FAMILIES keeps the iPhone first and ignores spaces', () => {
  process.env.TAC_IOS_FAMILIES = ' iPad , iPhone '
  assert.deepEqual(SharedSimulator.families(), ['iPhone', 'iPad'])
})

test('an unknown family in TAC_IOS_FAMILIES fails with its name', () => {
  process.env.TAC_IOS_FAMILIES = 'iPhone,Watch'
  assert.throws(() => SharedSimulator.families(), /TAC_IOS_FAMILIES names an unknown iOS family: Watch\. Use iPhone, iPad, iPhoneDuo, or a comma list such as iPhone,iPad\./)
})

test('TAC_IOS_FAMILIES=iPhoneDuo checks only the foldable iPhone Duo', () => {
  process.env.TAC_IOS_FAMILIES = 'iPhoneDuo'
  assert.deepEqual(SharedSimulator.families(), ['iPhoneDuo'])
  assert.equal(SharedSimulator.wants('iPhoneDuo'), true)
})

test('without TAC_IOS_FAMILIES the iPhone Duo gate still runs', () => {
  assert.equal(SharedSimulator.wants('iPhoneDuo'), true)
})

test('TAC_IOS_FAMILIES=iPhone leaves the iPhone Duo out', () => {
  process.env.TAC_IOS_FAMILIES = 'iPhone'
  assert.equal(SharedSimulator.wants('iPhoneDuo'), false)
})

test('an empty TAC_IOS_FAMILIES fails instead of checking nothing', () => {
  process.env.TAC_IOS_FAMILIES = ' , '
  assert.throws(() => SharedSimulator.families(), /TAC_IOS_FAMILIES names no iOS family/)
})
