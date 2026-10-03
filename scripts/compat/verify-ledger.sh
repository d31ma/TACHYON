#!/usr/bin/env bash
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TY="${TAC_BIN:-${CARGO_TARGET_DIR:-${REPO}/target}/release/ty}"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/ty-verify-XXXXXX")"

PAGE='<test-view />'
VIEW='<main aria-label="T"><h1>T</h1></main>'
SERVICE="@Service()\nexport default class TestViewService { count = 0 }"
NAMED_VIEW='<main aria-label="T"><output>{testView.count}</output></main>'
YON_CONTROLLER='@RequestRoute("/products")\n@Public\n@NeedsAll()\nclass ProductsController {\nproductsService;\n@GetRoute\nget() { return this.productsService.all(); }\n}'

PASS=0
FAIL=0
declare -a ROWS=()
PROBE_RESULT=""
PROBE_OUTPUT=""
PROBE_PROJECT=""
TS_EXPECTED=""
RUST_WEB_EXPECTED=""

add_local_tools_to_path() {
  if [ -x "${REPO}/node_modules/.bin/tsc" ]; then
    PATH="${REPO}/node_modules/.bin:${PATH}"
    export PATH
  fi
}

write_project_files() {
  local project=$1
  local path content
  while IFS=$'\t' read -r path content; do
    [ -z "${path}" ] && continue
    mkdir -p "${project}/$(dirname "${path}")"
    printf '%b' "${content}" > "${project}/${path}"
  done
}

probe() {
  PROBE_PROJECT="${WORK}/$(echo "$1" | tr -c 'a-zA-Z0-9' '_')"
  rm -rf "${PROBE_PROJECT}"
  mkdir -p "${PROBE_PROJECT}"
  write_project_files "${PROBE_PROJECT}"
  if PROBE_OUTPUT="$("${TY}" build "${PROBE_PROJECT}" 2>&1)"; then
    PROBE_RESULT="ok"
    return
  fi
  PROBE_RESULT="$(printf '%s' "${PROBE_OUTPUT}" | grep -oE 'TY[0-9]{4}' | head -1)"
  PROBE_RESULT="${PROBE_RESULT:-FAILED}"
}

record() {
  local feature="$1" expected="$2" actual="$3"
  if [ "${expected}" = "${actual}" ]; then
    ROWS+=("PASS|${feature}|${expected}|${actual}")
    PASS=$((PASS+1))
    return
  fi
  ROWS+=("FAIL|${feature}|${expected}|${actual}")
  FAIL=$((FAIL+1))
}

check() {
  local feature="$1" expected="$2"
  probe "${feature}"
  record "${feature}" "${expected}" "${PROBE_RESULT}"
}

check_built_file() {
  local feature="$1" expected="$2" built_file="$3"
  probe "${feature}"
  if [ "${PROBE_RESULT}" = "ok" ] && [ ! -f "${PROBE_PROJECT}/dist/${built_file}" ]; then
    PROBE_RESULT="missing ${built_file}"
  fi
  record "${feature}" "${expected}" "${PROBE_RESULT}"
}

check_message() {
  local feature="$1" expected="$2" message="$3"
  probe "${feature}"
  if [[ "${PROBE_OUTPUT}" != *"${message}"* ]]; then
    PROBE_RESULT="${PROBE_RESULT} without the expected message"
  fi
  record "${feature}" "${expected}" "${PROBE_RESULT}"
}

detect_typescript_expectation() {
  local ts_major
  ts_major="$(tsc --version 2>/dev/null | awk '{print $NF}' | cut -d. -f1)"
  if [ "${ts_major:-0}" -ge 6 ] 2>/dev/null; then
    TS_EXPECTED="ok"
    return
  fi
  TS_EXPECTED="TY1009"
  echo "note: TypeScript ${ts_major:-none} on PATH; TypeScript services expected to fail closed"
}

detect_rust_web_expectation() {
  local target_directory
  target_directory="$(rustc --print target-libdir --target wasm32-unknown-unknown 2>/dev/null)"
  if [ -n "${target_directory}" ] && [ -d "${target_directory}" ]; then
    RUST_WEB_EXPECTED="ok"
    return
  fi
  RUST_WEB_EXPECTED="TY1605"
  echo "note: rustc has no wasm32-unknown-unknown target; Rust web services expected to fail closed"
}

check_page_rules() {
  check "composed page over a static component" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
EOF

  check "plain markup in a page" "TY1309" <<EOF
client/pages/tac.html	${VIEW}
client/components/test/view/tac.html	${VIEW}
EOF

  check "page companion" "TY1309" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
client/pages/tac.js	export default {}
EOF

  check "page stylesheet" "TY1309" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
client/pages/tac.css	h1{color:red}
EOF

  check "page state script" "TY1309" <<EOF
client/pages/tac.html	<script>let count = 0</script>${PAGE}
client/components/test/view/tac.html	${VIEW}
EOF
}

check_component_companions() {
  check "component tac.css" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
client/components/test/view/tac.css	h1{color:red}
EOF

  check "tac.js beside a component view" "TY1310" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
client/components/test/view/tac.js	export default class { count = 0 }
EOF

  check "tac.rs beside a component view" "TY1310" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
client/components/test/view/tac.rs	#[derive(Default)]\nstruct TestViewService { count: i64 }
EOF

  check "retired tachyon-island companion" "TY1310" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
client/components/test/view/tachyon-island.js	export default class {}
EOF
}

check_services() {
  check "browser service named by a view" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${NAMED_VIEW}
client/services/TestViewService.js	${SERVICE}
EOF

  check "TypeScript service" "${TS_EXPECTED}" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${NAMED_VIEW}
client/services/TestViewService.ts	@Service()\nexport default class TestViewService { count: number = 1 }
EOF

  check "service class must match its file name" "TY1310" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${NAMED_VIEW}
client/services/TestViewService.js	@Service()\nexport default class Other { count = 0 }
EOF

  check "service file without the Service suffix" "TY1310" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
client/services/Helper.js	export default class Helper {}
EOF

  check "service in a subdirectory" "TY1310" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
client/services/nested/TestViewService.js	${SERVICE}
EOF

  check "service declaring both js and ts" "TY1310" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${NAMED_VIEW}
client/services/TestViewService.js	${SERVICE}
client/services/TestViewService.ts	@Service()\nexport default class TestViewService { count: number = 1 }
EOF

  check "mount policy on a component naming no service" "TY1405" <<EOF
client/pages/tac.html	<test-view hydrate="load" />
client/components/test/view/tac.html	${VIEW}
EOF
}

check_polyglot_services() {
  check "tac.py polyglot companion" "TY1401" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
client/components/test/view/tac.py	x = 1
EOF

  check_built_file "Rust service builds for web via WebAssembly" "${RUST_WEB_EXPECTED}" ".tachyon/services/tachyon-rust.js" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${NAMED_VIEW}
client/services/TestViewService.rs	#[derive(Default)]\nstruct TestViewService { count: i64 }
EOF

  check "native service has an explicit JS web file" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${NAMED_VIEW}
client/services/TestViewService.rs	#[derive(Default)]\nstruct TestViewService { count: i64 }
client/services/TestViewService.js	${SERVICE}
EOF

  check "retired Dart Tac companion fails closed" "TY1401" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
client/components/test/view/tac.dart	class Companion {}
EOF
}

check_view_and_service_features() {
  check "inline <style> in a component view" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main aria-label="T"><style>h1{color:red}</style><h1>T</h1></main>
EOF

  check "inline <script> in a component view" "TY1306" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main aria-label="T"><script>console.log(1)</script><h1>T</h1></main>
EOF

  check "service state addressed by instance name" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main aria-label="T"><button on:click="testView.count += 1">Add</button><output>{testView.count}</output></main>
client/services/TestViewService.js	${SERVICE}
EOF

  check "handler naming more than one member" "TY1306" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main aria-label="T"><button on:click="testView.nested.add()">Add</button></main>
client/services/TestViewService.js	@Service()\nexport default class TestViewService { nested = { add() {} } }
EOF

  check "prototype-scoped service" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${NAMED_VIEW}
client/services/TestViewService.js	@Service()\n@Lifetime({"value":"prototype"})\nexport default class TestViewService { count = 0 }
EOF

  check "owned typed application events in a service" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${NAMED_VIEW}
client/services/TestViewService.js	/**\n * @typedef {Object} Counter\n * @property {integer} value\n */\n/**\n * @typedef {Object} Invoice\n * @property {integer} value\n */\n@Service()\nexport default class TestViewService {\napplicationEventPublisher;\ncount = 0;\nseen = 0;\nbump() { this.count += 1; /** @type {Counter} */ const counter = { value: this.count }; this.applicationEventPublisher.publishEvent(counter, true); }\n/**\n * @param {Counter} counter\n * @returns {Invoice}\n */\n@OnEvent({"condition":"value.value >= 0"})\n@Order(2)\nchanged(counter) { this.seen = counter.value; return { value: counter.value }; }\n/** @param {Invoice} invoice */\n@OnEvent\ninvoiced(invoice) { this.seen = invoice.value; }\n/** @param {Counter} counter */\n@OnEvent\n@InBackground()\nasync audit(counter) { this.seen = counter.value; }\n}
EOF

  check_message "string event names are refused with a migrate hint" "TY1308" "OnEvent takes no event names" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${NAMED_VIEW}
client/services/TestViewService.js	@Service()\nexport default class TestViewService {\nseen = 0;\n@OnEvent({"event":"counter"})\nchanged(value) { this.seen = value; }\n}
EOF

  check "retired EventPublisher marker in a service" "TY1308" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${NAMED_VIEW}
client/services/TestViewService.js	@Service()\nexport default class TestViewService {\ncount = 0;\n@EventPublisher({"event":"counter"})\nbump() { this.count += 1; return this.count; }\n}
EOF

  check "Tac Component provider bound by Inject" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${NAMED_VIEW}
client/helpers/PricingHelper.js	@Helper()\nclass PricingHelper { price() { return 5 } }
client/services/TestViewService.js	@Service()\n@Needs(PricingHelper)\nexport default class TestViewService {\npricingHelper;\ncount = 0;\nread() { this.count = this.pricingHelper.price(); }\n}
EOF

  check "retired signal decorator fails before emission" "TY1306" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${NAMED_VIEW}
client/services/TestViewService.js	@Service()\nexport default class TestViewService {\n@publish(untrusted)\ncount = 0;\n}
EOF

  check "JS service module-level helpers fail the single-class rule" "TY1310" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${NAMED_VIEW}
client/services/TestViewService.js	const LIMIT = 3;\n@Service()\nexport default class TestViewService { count = LIMIT; }
EOF
}

check_routes_and_components() {
  check "nested route directory" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/pages/about/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
EOF

  check "dynamic route segment _id" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/pages/_id/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
EOF

  check "invalid dynamic segment name" "TY1006" <<EOF
client/pages/tac.html	${PAGE}
client/pages/_1bad/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
EOF

  check "void elements unclosed" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main aria-label="T"><img src="/a.png" alt="A"><hr><br></main>
EOF

  check "component two-level path" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main aria-label="T"><product-card><p>s</p></product-card></main>
client/components/product/card/tac.html	<article aria-label="C"><slot></slot></article>
EOF

  check "component tac.css companion" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main aria-label="T"><product-card><p>s</p></product-card></main>
client/components/product/card/tac.html	<article aria-label="C"><slot></slot></article>
client/components/product/card/tac.css	article{color:red}
EOF

  check "TypeScript service named by a nested component" "${TS_EXPECTED}" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main aria-label="T"><product-card><p>s</p></product-card></main>
client/components/product/card/tac.html	<article aria-label="C">{productCard.x}<slot></slot></article>
client/services/ProductCardService.ts	@Service()\nexport default class ProductCardService { x: number = 1 }
EOF

  check "component one-level path" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main aria-label="T"><clicker>s</clicker></main>
client/components/clicker/tac.html	<article aria-label="C"><slot></slot></article>
EOF

  check "component shadowing an HTML element" "TY1401" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main aria-label="T"><h1>T</h1></main>
client/components/section/tac.html	<article aria-label="C">s</article>
EOF
}

check_yon_handlers() {
  check "yon.html is rejected" "TY1008" <<EOF
server/yon.html	<main>not a Yon endpoint</main>
EOF

  check "JavaScript controller discovery" "ok" <<EOF
server/controllers/HomeController.js	@RequestRoute("/")\n@Public\nexport class HomeController {\n@GetRoute\nget() { return { ok: true } }\n}
EOF

  check "Python controller discovery" "ok" <<EOF
server/controllers/HomeController.py	@request_route("/")\n@public\nclass HomeController:\n    @get_route\n    def get(self):\n        return {"ok": True}
EOF

  check "TypeScript controller discovery" "ok" <<EOF
server/controllers/HomeController.ts	@RequestRoute("/")\n@Public\nexport class HomeController {\n@GetRoute\nget() { return {} }\n}
EOF

  check "Rust controller discovery" "ok" <<EOF
server/controllers/HomeController.rs	#[RequestRoute("/")]\n#[Public]\nstruct HomeController {}\nimpl HomeController {\n    fn new() -> Self { Self {} }\n    #[GetRoute]\n    fn get(&self) -> Json { Json::Null }\n}
EOF

  check_message "project cannot register a Ruby interpreter" "TY1502" "The 'interpreters' field was removed" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
.tachyonrc	{"interpreters":{"rb":["ruby"]}}
EOF
}

check_template_syntax() {
  check "conditional <logic :if>/<logic else>" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main><logic :if="on"><p>y</p></logic><logic else><p>n</p></logic></main>
EOF

  check "iteration <loop :for>" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main><loop :for="i of items"><p>{i}</p></loop></main>
EOF

  check_message "removed <if> element" "TY1367" "Write <logic :if" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main><if :when="on"><p>y</p></if></main>
EOF

  check_message "removed <for> element" "TY1367" "Write <loop :for" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main><for :each="i in items"><p>{i}</p></for></main>
EOF

  check "counted loop ascending" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main><loop :for="let i = 0; i < 3; i++"><p>{i}</p></loop></main>
EOF

  check "counted loop descending" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main><loop :for="let i = 4; i >= 0; i -= 2"><p>{i}</p></loop></main>
EOF

  check "counted loop wrong direction fails closed" "TY1302" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main><loop :for="let i = 0; i < 3; i--"><p>{i}</p></loop></main>
EOF

  check "dynamic attribute :attr" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	<main :aria-label="label"><h1>x</h1></main>
EOF
}

check_project_files() {
  check "middleware.js present" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
middleware.js	export class Middleware { static before() { return { status: 204 } } }
EOF

  check "server/workers present" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
server/workers/job.js	export class Handler { static GET() { return { status: 204 } } }
EOF

  check ".tachyonrc present" "ok" <<EOF
client/pages/tac.html	${PAGE}
client/components/test/view/tac.html	${VIEW}
.tachyonrc	{"interpreters":{}}
EOF

  check "controller importing a service module is not executed at build" "ok" <<EOF
server/controllers/HomeController.js	import { ValueService } from '../services/ValueService.js'\n@RequestRoute("/")\n@Public\nexport class HomeController {\n@GetRoute\nget() { return { v: ValueService.v } }\n}
server/services/ValueService.js	export class ValueService { static v = 'from-service' }
EOF
}

check_yon_roles() {
  check "shared server role folder behind a routed controller" "ok" <<EOF
server/controllers/ProductsController.js	${YON_CONTROLLER}
server/services/ProductsService.js	@Service()\nclass ProductsService {\nall() { return [] }\n}
EOF

  check "Yon role file beside its route" "TY1310" <<EOF
server/controllers/ProductsController.js	${YON_CONTROLLER}
server/routes/products/services/ProductsService.js	@Service()\nclass ProductsService {\nall() { return [] }\n}
EOF

  check "Yon controller outside its controllers folder" "TY1310" <<EOF
server/ProductsController.js	${YON_CONTROLLER}
server/services/ProductsService.js	@Service()\nclass ProductsService {\nall() { return [] }\n}
EOF

  check "Yon autowired datasource, Redis and event publisher" "ok" <<EOF
server/controllers/ProductsController.js	${YON_CONTROLLER}
server/services/ProductsService.js	@Service\nclass ProductsService {\n@Needs\ndataSource;\n@Needs\nredisTemplate;\n@Needs\napplicationEventPublisher;\nasync all() { /** @type {Listed} */ const listed = { count: 1 }; this.applicationEventPublisher.publishEvent(listed); await this.redisTemplate.get('k'); return this.dataSource.query('SELECT 1', []); }\n}
EOF

  check_message "retired Yon Resource marker fails closed" "TY1308" "Resource is retired (ADR 0038): write @Needs on a field named dataSource" <<EOF
server/controllers/ProductsController.js	${YON_CONTROLLER}
server/services/ProductsService.js	@Service\nclass ProductsService {\n@Resource({"name":"dataSource"})\ndataSource;\nall() { return []; }\n}
EOF

  check "Yon typed listeners, condition, Order and Async" "ok" <<EOF
server/controllers/ProductsController.js	${YON_CONTROLLER}
server/services/ProductsService.js	/**\n * @typedef {Object} OrderPlaced\n * @property {integer} total\n */\n/**\n * @typedef {Object} Invoice\n * @property {integer} total\n */\n@Service()\n@NeedsAll()\nclass ProductsService {\napplicationEventPublisher;\nseen = 0;\nall() { /** @type {OrderPlaced} */ const order = { total: 12 }; this.applicationEventPublisher.publishEvent(order, true); return this.seen; }\n/**\n * @param {OrderPlaced} order\n * @returns {Invoice}\n */\n@OnEvent({"condition":"value.total >= 10"})\n@Order(2)\nrecord(order) { this.seen = order.total; return { total: 0 }; }\n/** @param {Invoice} invoice */\n@OnEvent\ninvoiced(invoice) { this.seen = invoice.total; }\n/** @param {OrderPlaced} order */\n@OnEvent\n@InBackground()\nasync audit(order) { this.seen = order.total; }\n}
EOF

  check "retired EventPublisher marker in a Yon service" "TY1308" <<EOF
server/controllers/ProductsController.js	${YON_CONTROLLER}
server/services/ProductsService.js	@Service()\nclass ProductsService {\n@EventPublisher({"event":"order.placed"})\nplace() { return 1 }\n}
EOF

  check "Yon Gateway to a program under server/helpers" "ok" <<EOF
server/controllers/ProductsController.js	${YON_CONTROLLER}
server/services/ProductsService.js	@Service()\nclass ProductsService {\nall() { return this.summarize() }\n@Gateway({"command":["python3","server/helpers/report.py"]})\nasync summarize() {}\n}
server/helpers/report.py	print("[]")
EOF

  check "Yon Gateway command file outside server/helpers" "TY1312" <<EOF
server/controllers/ProductsController.js	${YON_CONTROLLER}
server/services/ProductsService.js	@Service()\nclass ProductsService {\nall() { return this.summarize() }\n@Gateway({"command":["python3","server/report.py"]})\nasync summarize() {}\n}
server/report.py	print("[]")
EOF

  check "authored file in reserved server/gateways" "TY1310" <<EOF
server/controllers/ProductsController.js	${YON_CONTROLLER}
server/services/ProductsService.js	@Service()\nclass ProductsService {\nall() { return [] }\n}
server/gateways/manual/yon.js	export class Handler { static GET() { return {} } }
EOF
}

print_report() {
  echo
  printf '%-6s %-42s %-10s %s\n' "VERDICT" "FEATURE" "EXPECTED" "ACTUAL"
  echo "--------------------------------------------------------------------------------------"
  local row verdict feature expected actual
  for row in "${ROWS[@]}"; do
    IFS='|' read -r verdict feature expected actual <<< "${row}"
    printf '%-6s %-42s %-10s %s\n' "${verdict}" "${feature}" "${expected}" "${actual}"
  done
  echo
  echo "${PASS} claims confirmed, ${FAIL} claims wrong"
}

main() {
  add_local_tools_to_path
  trap 'rm -rf "${WORK}"' EXIT
  echo "verifying against: ${TY}"
  "${TY}" --version
  echo
  check_page_rules
  check_component_companions
  detect_typescript_expectation
  detect_rust_web_expectation
  check_services
  check_polyglot_services
  check_view_and_service_features
  check_routes_and_components
  check_yon_handlers
  check_template_syntax
  check_project_files
  check_yon_roles
  print_report
  [ "${FAIL}" -eq 0 ]
}

main "$@"
