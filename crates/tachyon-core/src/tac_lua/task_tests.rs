#![allow(clippy::expect_used)]

use super::{LuaRuntimeBuild, LuaWebServices};
use std::path::Path;

struct TaskTests;

impl TaskTests {
    const GREETING: &str = "-- @Service\n---@class GreetingService\nGreetingService = {\n  ---@type string\n  early = \"\",\n  ---@type integer\n  bumps = 0,\n}\n\n---@return nil\n-- @OnStart\nfunction GreetingService:start()\n  self.early = TacTasks.exchange(\"{}\")\nend\n\n---@param path string\n---@param deadline integer\n---@return string\nfunction GreetingService:read(path, deadline)\n  local request = { method = \"GET\", path = path, redirect = \"error\", credentials = \"omit\", deadline = deadline, maxResponseBytes = 4096 }\n  local answer = YonJson.decode(TacTasks.exchange(YonJson.encode(request)))\n  if answer.error ~= nil then\n    error({ behavior_code = answer.error.code, message = \"HTTP failed with \" .. answer.error.code }, 0)\n  end\n  return answer.value.message\nend\n\n---@return integer\nfunction GreetingService:bump()\n  self.bumps = self.bumps + 1\n  return self.bumps\nend\n";

    const WEATHER: &str = "-- @Service\n---@class WeatherService\nWeatherService = {}\n\n---@param path string\n---@param deadline integer\n---@return string\nfunction WeatherService:forecast(path, deadline)\n  local request = { method = \"GET\", path = path, redirect = \"error\", credentials = \"omit\", deadline = deadline, maxResponseBytes = 4096 }\n  return YonJson.decode(TacTasks.exchange(YonJson.encode(request))).value.message\nend\n";

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'\n\
         globalThis.fetch = async (url) => new Response(await readFile(url))\n\
         globalThis.location = new URL('http://localhost/')\n\
         ;(0, eval)(await readFile('./tac-http.js', 'utf8'))\n\
         const { default: GreetingService } = await import('./GreetingService.js')\n\
         const { default: WeatherService } = await import('./WeatherService.js')\n\
         const replies = new Map(), aborted = []\n\
         globalThis.fetch = (url, options) => new Promise((resolve, reject) => {\n\
         \x20 const path = new URL(url).pathname\n\
         \x20 replies.set(path, (body, status) => resolve(new Response(body, { status })))\n\
         \x20 options.signal.addEventListener('abort', () => { aborted.push(path); reject(new Error('aborted')) })\n\
         })\n\
         const reply = (path, value) => replies.get(path)(JSON.stringify(value), 200)\n\
         const outcome = (promise) => Promise.resolve(promise).then((value) => ({ value }), (error) => ({ code: error.code ?? '', message: String(error.message) }))\n\
         const deadline = Date.now() + 30000\n\
         const one = new GreetingService({}), two = new WeatherService({}), three = new GreetingService({})\n\
         const early = one.early\n\
         const first = outcome(one.read('/api/one', deadline))\n\
         const second = outcome(two.forecast('/api/two', deadline))\n\
         const third = outcome(three.read('/api/three', deadline))\n\
         const started = [...replies.keys()]\n\
         const bumped = one.bump()\n\
         three.__tachyonDispose()\n\
         reply('/api/two', { message: 'Rain' })\n\
         const weather = await second\n\
         reply('/api/one', { message: 'Hello' })\n\
         const greeting = await first, removed = await third\n\
         const failing = outcome(one.read('/api/missing', deadline))\n\
         replies.get('/api/missing')('', 404)\n\
         const failed = await failing\n\
         const many = Array.from({ length: 64 }, (_, index) => outcome(one.read(`/api/many/${index}`, deadline)))\n\
         let crowded = ''\n\
         try { one.read('/api/crowded', deadline) } catch (error) { crowded = error.message }\n\
         one.__tachyonDispose()\n\
         const cancelled = (await Promise.all(many)).filter((answer) => answer.code === 'CANCELLED').length\n\
         console.log(JSON.stringify({ early, started, bumped, weather, greeting, removed, failed, crowded, cancelled, aborted }))\n\
         process.exit(0)\n";

    fn missing_tool() -> Option<String> {
        let node = std::process::Command::new("node")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success());
        assert!(
            node || std::env::var_os("CI").is_none(),
            "CI must provide node"
        );
        (!node)
            .then(|| String::from("node is unavailable"))
            .or_else(LuaRuntimeBuild::unavailable_reason)
    }

    fn save(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
        std::fs::write(path, bytes).expect("file");
    }

    async fn build_page(root: &Path) {
        let services = [
            ("greeting", "GreetingService", Self::GREETING),
            ("weather", "WeatherService", Self::WEATHER),
        ];
        let chosen: Vec<_> = services
            .iter()
            .map(|(instance, class, text)| {
                let source = root.join(format!("client/services/{class}.lua"));
                Self::save(&source, text.as_bytes());
                let language = crate::tac_web::WebLanguage::Lua;
                (
                    String::from(*instance),
                    String::from(*class),
                    language,
                    source,
                )
            })
            .collect();
        let jobs = crate::tac_web::BuildJobs::new(1);
        let built = crate::tac_web::build_all(root, &chosen, &jobs, crate::BuildProfile::Release)
            .await
            .expect("page");
        built
            .files
            .iter()
            .for_each(|(name, bytes)| Self::save(&root.join(name), bytes));
        let http = include_str!("../template/tac-http.js");
        Self::save(&root.join("tac-http.js"), http.as_bytes());
        Self::save(&root.join("tasks.mjs"), Self::DRIVER.as_bytes());
    }

    fn report(root: &Path) -> serde_json::Value {
        let finished = std::process::Command::new("node")
            .arg("tasks.mjs")
            .current_dir(root)
            .output()
            .expect("node");
        let printed = String::from_utf8_lossy(&finished.stdout);
        let problems = String::from_utf8_lossy(&finished.stderr);
        assert!(finished.status.success(), "{printed}{problems}");
        let line = printed.lines().last().unwrap_or_default();
        serde_json::from_str(line).expect("report")
    }
}

#[test]
fn the_lua_engine_routes_task_messages_like_the_ruby_engine() {
    let root = tempfile::tempdir().expect("project");
    let engine = LuaWebServices::engine(root.path(), &[]).expect("engine");
    assert!(engine.contains("class TacTaskAnswers"), "{engine}");
    assert!(engine.contains("class TacTicketHttp"), "{engine}");
    assert!(engine.contains("if (request.http) return luaTasks(request)"));
    assert!(engine.contains("lua.invoke(`tachyon-http:settle ${ticket}\\n${answer}`)"));
    assert!(LuaWebServices::PRELUDE.contains("function TacTasks.exchange(request)"));
}

#[tokio::test]
async fn a_lua_call_parks_on_browser_http_until_the_answer_arrives() {
    if let Some(reason) = TaskTests::missing_tool() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    TaskTests::build_page(root.path()).await;
    let report = TaskTests::report(root.path());
    assert_eq!(
        report["early"], "{\"error\":{\"code\":\"HTTP_REQUEST\"}}",
        "{report}"
    );
    assert_eq!(
        report["started"],
        serde_json::json!(["/api/one", "/api/two", "/api/three"]),
        "{report}"
    );
    assert_eq!(report["bumped"], 1, "{report}");
    assert_eq!(report["weather"]["value"], "Rain", "{report}");
    assert_eq!(report["greeting"]["value"], "Hello", "{report}");
    assert_eq!(report["removed"]["code"], "CANCELLED", "{report}");
    assert_eq!(report["failed"]["code"], "HTTP_STATUS", "{report}");
    assert!(
        report["crowded"]
            .as_str()
            .is_some_and(|message| message.contains("HTTP_CAPACITY")),
        "{report}"
    );
    assert_eq!(report["cancelled"], 64, "{report}");
    let aborted = report["aborted"].as_array().expect("aborted");
    assert_eq!(aborted.len(), 65, "{report}");
    assert_eq!(aborted[0], "/api/three", "{report}");
}
