#!/usr/bin/env node
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

class NativeEventsTest {
  static root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  static emitted = process.env.TAC_NATIVE_EVENT_FIXTURES;
}

assert.ok(NativeEventsTest.emitted, 'Set TAC_NATIVE_EVENT_FIXTURES to the emitted native owner fixture directory.');

class Owners {
  static first = '00000000-0000-4000-8000-000000000001';
  static second = '00000000-0000-4000-8000-000000000002';
  static third = '00000000-0000-4000-8000-000000000003';
  static fourth = '00000000-0000-4000-8000-000000000004';
}

function request(owner, op, extra = {}) {
  return { route: '/', service: 'ownerProbe', owner, op, ...extra };
}

class ProbeSequence {
  static requests = [
    request(Owners.first, 'init'), request(Owners.first, 'get', {name:'count'}), request(Owners.first,'get',{name:'deaths'}),
    request(Owners.first,'init'), request(Owners.first,'call',{name:'send',args:[]}), request(Owners.first,'get',{name:'count'}),
    request(Owners.first,'call',{name:'missing',args:[]}), request(Owners.first,'get',{name:'count'}),
    request(Owners.first,'call',{name:'cycle',args:[]}), request(Owners.first,'get',{name:'count'}),
    request(Owners.first,'dispose'), request(Owners.first,'dispose'), request(Owners.second,'init'), request(Owners.second,'get',{name:'deaths'}),
    request(Owners.first,'get',{name:'count'}), request(Owners.first,'dispose'), request(Owners.second,'get',{name:'count'}),
    {op:'host-dispose'}, request(Owners.third,'init'), request(Owners.third,'get',{name:'deaths'}),
    request(Owners.fourth,'init'), request(Owners.fourth,'get',{name:'deaths'}), request(Owners.fourth,'get',{name:'count'}),
    request(Owners.fourth,'call',{name:'isolated',args:[]}), request(Owners.fourth,'get',{name:'count'}),
    request(Owners.fourth,'call',{name:'publishOrdered',args:[]}), request(Owners.fourth,'get',{name:'log'}), request(Owners.fourth,'get',{name:'count'}),
    request(Owners.fourth,'call',{name:'publishOther',args:[]}), request(Owners.fourth,'get',{name:'log'}), request(Owners.fourth,'get',{name:'count'}),
  ];
}

function run(scratch, program, args, options = {}) {
  const result = spawnSync(program, args, {cwd:scratch, encoding:'utf8', timeout:180000, maxBuffer:4*1024*1024, ...options});
  assert.equal(result.error, undefined, result.error?.message);
  assert.equal(result.status, 0, `${program} failed:\n${result.stdout}\n${result.stderr}`);
  return result;
}

function verify(result, language) {
  if (process.env.TAC_NATIVE_EVENT_DEBUG) console.error(result.stdout + result.stderr);
  const records = result.stdout.trim().split('\n');
  const answers = records.filter(line=>line.startsWith('R')).map(line=>JSON.parse(line.slice(1)));
  const events = records.filter(line=>line.startsWith('E')).map(line=>JSON.parse(line.slice(1)));
  assert.equal(answers.length, ProbeSequence.requests.length, result.stdout);
  assert.equal(answers[1].value,9); assert.equal(answers[2].value,0);
  assert.deepEqual(answers[0],answers[3]);
  assert.equal(answers[4].value,5); assert.equal(answers[5].value,17);
  assert.ok(answers[6].error); assert.equal(answers[7].value,17);
  assert.equal(answers[9].value,145); assert.ok(answers[11].error);
  assert.equal(answers[13].value,1); assert.ok(answers[14].error); assert.ok(answers[15].error);
  assert.equal(answers[16].value,9); assert.equal(answers[19].value,2);
  assert.equal(answers[21].value,2); assert.equal(answers[22].value,9);
  assert.deepEqual(answers[23].value,{value:1}); assert.equal(answers[24].value,10);
  assert.equal(answers[26].value,'123AE'); assert.equal(answers[27].value,17);
  assert.equal(answers[29].value,'123AE1E'); assert.equal(answers[30].value,24);
  assert.ok(!answers[0].value.methods.some(name => /start|stop|receive|ordinal|second|third|skipped|failing|deferred|onEcho/i.test(name)));
  assert.ok(!answers[0].value.methods.some(name => /mutation|original/i.test(name)));
  assert.equal(events.filter(event=>event.name==='Cycle').length,128);
  assert.deepEqual(events.slice(0,2).map(event=>event.name),['Pulse','Relay']);
  assert.deepEqual(events.slice(0,2).map(event=>event.value),[{amount:5},{amount:3}]);
  assert.deepEqual(events.slice(-4).map(event=>event.name),['Ordered','Echo','Other','Echo']);
  assert.deepEqual(events.slice(-3)[0].value,{amount:7});
  assert.ok(events.every(event=>event.route==='/' && event.service==='ownerProbe' && [Owners.first,Owners.second,Owners.third,Owners.fourth].includes(event.owner)));
  assert.equal(events[0].retain,true); assert.equal(events[1].retain,false);
  assert.match(result.stderr,/native event delivery failed/);
  console.log(`PASS ${language}: activation, hook ordering, no duplicate init, typed model publisher, listener reentrancy, bounded cycles, Order/condition/Async/returned model, exact disposal and stale-owner rejection.`);
}

async function buildRust(scratch, unit, prelude) {
  const main = `\nconst TAC_APPLICATION_ID: &str = "events-fixture";\nextern "C" fn receive(raw: *const core::ffi::c_char) { let text = unsafe {std::ffi::CStr::from_ptr(raw)}.to_string_lossy(); assert!(tac_invoke_owned("{\\"op\\":\\"host-dispose\\"}").contains("busy")); println!("E{}",text); }\nfn main() { use std::io::BufRead; unsafe { tac_native_set_emit(Some(receive)); } for line in std::io::stdin().lock().lines() { println!("R{}",tac_invoke_owned(&line.unwrap())); } }\n`;
  await writeFile(join(scratch,'main.rs'),unit+prelude+main);
  run(scratch, 'rustc',['--edition','2024',join(scratch,'main.rs'),'-o',join(scratch,'rust-owner')]);
  return { program: join(scratch,'rust-owner'), args: [] };
}

async function buildSwift(scratch, unit, prelude) {
  const json = await readFile(join(NativeEventsTest.root,'crates/tachyon-core/src/native/apple_json.swift'),'utf8');
  const main = `\nTacBridge.emit = { payload in precondition(tacNativeInvoke("{\\"op\\":\\"host-dispose\\"}").contains("busy")); print("E" + payload) }\nwhile let line = readLine() { print("R" + tacNativeInvoke(line)) }\n`;
  await writeFile(join(scratch,'main.swift'),'import Foundation\n'+json+unit+prelude+main);
  run(scratch, 'swiftc',[join(scratch,'main.swift'),'-o',join(scratch,'swift-owner')]);
  return { program: join(scratch,'swift-owner'), args: [] };
}

async function buildKotlin(scratch, unit, prelude) {
  const response = await fetch('https://repo.maven.apache.org/maven2/org/json/json/20260814/json-20260814.jar', {signal:AbortSignal.timeout(30000)});
  assert.equal(response.status,200);
  const jar = Buffer.from(await response.arrayBuffer());
  assert.equal(createHash('sha256').update(jar).digest('hex'),'7dd2de1533e766e2b6c07d8194da85145ad313a167a4760050f5c9b6839604fe');
  await writeFile(join(scratch,'json.jar'),jar);
  await writeFile(join(scratch,'SharedPreferences.kt'),`package android.content\ninterface SharedPreferences { val all: Map<String, *>; fun edit(): Editor; interface Editor { fun putInt(key:String,value:Int):Editor; fun putLong(key:String,value:Long):Editor; fun putBoolean(key:String,value:Boolean):Editor; fun putFloat(key:String,value:Float):Editor; fun putString(key:String,value:String?):Editor; fun apply() } }\n`);
  const main = `\nfun main() { TacBridge.emit = TacEmit { payload -> check(tacNativeInvoke("{\\"op\\":\\"host-dispose\\"}").contains("busy")); println("E" + payload) }; generateSequence(::readLine).forEach { println("R" + tacNativeInvoke(it)) } }\n`;
  await writeFile(join(scratch,'Owner.kt'),'import kotlin.coroutines.startCoroutine\n'+unit+prelude+main);
  await writeFile(join(scratch,'Models.kt'),await readFile(join(NativeEventsTest.emitted,'owner-models.kt'),'utf8'));
  run(scratch, 'kotlinc',['-classpath',join(scratch,'json.jar'),join(scratch,'Owner.kt'),join(scratch,'Models.kt'),join(scratch,'SharedPreferences.kt'),'-include-runtime','-d',join(scratch,'owner.jar')]);
  return { program: 'java', args: ['-cp',join(scratch,'owner.jar')+':'+join(scratch,'json.jar'),'OwnerKt'] };
}

async function buildCSharp(scratch, unit, prelude) {
  const major=Number(run(scratch, 'dotnet',['--version']).stdout.trim().split('.')[0]);
  await writeFile(join(scratch,'Owner.csproj'),`<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net${major}.0</TargetFramework><AllowUnsafeBlocks>true</AllowUnsafeBlocks><Nullable>disable</Nullable><ImplicitUsings>enable</ImplicitUsings></PropertyGroup></Project>`);
  const main = `\nstatic class TacApplication { internal const string Id = "events-fixture"; }\nstatic class Program { [UnmanagedCallersOnly] static void Receive(IntPtr raw) { if (!Tac.Invoke("{\\"op\\":\\"host-dispose\\"}").Contains("busy")) throw new Exception("reentrancy"); Console.WriteLine("E" + Marshal.PtrToStringUTF8(raw)); } public static unsafe void Main() { Tac.SetEmit((IntPtr)(delegate* unmanaged<IntPtr, void>)&Receive); string line; while ((line=Console.ReadLine()) != null) Console.WriteLine("R"+Tac.Invoke(line)); } }\n`;
  await writeFile(join(scratch,'Owner.cs'),'using System; using System.Collections; using System.Collections.Generic; using System.Linq; using System.Globalization; using System.Text; using System.Text.Json.Nodes; using System.Runtime.InteropServices;\n'+unit+prelude+main);
  run(scratch, 'dotnet',['build',join(scratch,'Owner.csproj'),'--nologo','--disable-build-servers']);
  return { program: 'dotnet', args: [join(scratch,'bin/Debug',`net${major}.0`,'Owner.dll')] };
}

async function buildOwner(scratch, language, unit, prelude) {
  if (language==='rs') return buildRust(scratch, unit, prelude);
  if (language==='swift') return buildSwift(scratch, unit, prelude);
  if (language==='kt') return buildKotlin(scratch, unit, prelude);
  if (language==='cs') return buildCSharp(scratch, unit, prelude);
  throw new Error(`Unknown language: ${language}`);
}

async function main() {
  const scratch = await mkdtemp(join(process.env.TAC_NATIVE_EVENT_TEST_ROOT || tmpdir(), 'tachyon-native-events-'));
  const languages = process.argv.slice(2);
  const selected = languages.length ? languages : ['rs', 'swift', 'kt', 'cs'];
  try {
    for (const language of selected) {
      const unit = await readFile(join(NativeEventsTest.emitted, `owner.${language}`),'utf8');
      const prelude = await readFile(join(NativeEventsTest.emitted, `prelude.${language}`),'utf8');
      const { program, args } = await buildOwner(scratch, language, unit, prelude);
      verify(run(scratch,program,args,{input:ProbeSequence.requests.map(value=>JSON.stringify(value)).join('\n')+'\n'}),language);
    }
  } finally { await rm(scratch,{recursive:true,force:true}); }
}

await main();
