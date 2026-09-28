package main

import (
	"syscall/js"
	"tachyonservices/tac"
)

func main() {
	registerServices()
	js.Global().Set("__tachyonGo", js.FuncOf(connect))
	select {}
}

func connect(this js.Value, arguments []js.Value) any {
	emit := arguments[0]
	tac.Emit = func(payload string) {
		emit.Invoke(payload)
	}
	if len(arguments) > 1 && arguments[1].Type() == js.TypeFunction {
		storage := arguments[1]
		tac.Storage = func(request string) string {
			return storage.Invoke(request).String()
		}
	}
	if len(arguments) > 2 && arguments[2].Type() == js.TypeFunction {
		web := arguments[2]
		tac.Web.Connect(func(request string) string {
			return web.Invoke(request).String()
		})
	}
	return js.FuncOf(invoke)
}

func invoke(this js.Value, arguments []js.Value) any {
	return tac.Invoke(arguments[0].String())
}
