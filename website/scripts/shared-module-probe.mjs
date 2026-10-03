import path from 'node:path'

export class SharedModuleProbe {
  /** @type {Record<string, string>} */
  static controllers = {
    '.js': 'import { OrderService } from "../shared/orders.js";\n@RequestRoute("/probe")\n@Public\nclass ProbeController {\n  @GetRoute\n  getProbe() { return OrderService.place("book") }\n}\n',
    '.ts': 'import { OrderService } from "../shared/orders.ts";\n@RequestRoute("/probe")\n@Public\nclass ProbeController {\n  @GetRoute\n  getProbe(): string { return OrderService.place("book") }\n}\n',
    '.py': '@request_route("/probe")\n@public\nclass ProbeController:\n    @get_route\n    def get_probe(self) -> str:\n        return OrderService.place("book")\n',
    '.php': '<?php\n#[RequestRoute("/probe")]\n#[Public]\nclass ProbeController {\n    #[GetRoute]\n    public function getProbe(): string {\n        return OrderService::place("book");\n    }\n}\n',
    '.java': '@RequestRoute("/probe")\n@Public\nclass ProbeController {\n    @GetRoute\n    String getProbe() {\n        return OrderService.place("book");\n    }\n}\n',
    '.groovy': '@RequestRoute("/probe")\n@Public\nclass ProbeController {\n    @GetRoute\n    String getProbe() {\n        return OrderService.place("book")\n    }\n}\n',
    '.kt': '@RequestRoute("/probe")\n@Public\nclass ProbeController {\n    @GetRoute\n    fun getProbe(): String = OrderService.place("book")\n}\n',
    '.cs': '[RequestRoute("/probe")]\n[Public]\nclass ProbeController {\n    [GetRoute]\n    public string GetProbe() { return OrderService.Place("book"); }\n}\n',
    '.fs': '[<RequestRoute("/probe")>]\n[<Public>]\ntype ProbeController() =\n    [<GetRoute>]\n    member this.GetProbe() : string = OrderService.Place("book")\n',
    '.rs': '#[RequestRoute("/probe")]\n#[Public]\nstruct ProbeController {}\nimpl ProbeController {\n    fn new() -> Self { Self {} }\n    #[GetRoute]\n    fn get_probe(&self) -> String { OrderService::place("book") }\n}\n',
    '.go': '// @RequestRoute("/probe")\n// @Public\ntype ProbeController struct{}\n\n// @GetRoute\nfunc (controller *ProbeController) GetProbe() string {\n\treturn OrderService{}.Place("book")\n}\n',
    '.dart': '@RequestRoute("/probe")\n@Public\nclass ProbeController {\n  @GetRoute\n  String getProbe() => OrderService.place("book");\n}\n',
    '.rb': '#[RequestRoute("/probe")]\n#[Public]\nclass ProbeController\n  # @return [String]\n  #[GetRoute]\n  def get_probe\n    OrderService.place("book")\n  end\nend\n',
    '.lua': '-- @RequestRoute("/probe")\n-- @Public\n---@class ProbeController\nProbeController = {}\n\n-- @GetRoute\n---@return string\nfunction ProbeController:get_probe()\n  return OrderService.place("book")\nend\n',
  }

  /** @param {{name: string, code: string}} file */
  static controller(file) {
    const extension = path.extname(file.name)
    const code = SharedModuleProbe.controllers[extension]
    if (!code) throw new Error(`No shared module probe for ${extension}`)
    return { name: `server/controllers/ProbeController${extension}`, code }
  }

  /** @param {{name: string}} file */
  static expected(file) {
    return 'book'
  }
}
