package main

import (
	"syscall/js"
	"tachyonservices/tac"
)

var httpStarted = map[int]js.Value{}

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
	if len(arguments) > 3 && arguments[3].Type() == js.TypeFunction {
		done := arguments[3]
		tac.TaskDone = func(id int, answer string) {
			done.Invoke(id, answer)
		}
		tac.HTTPStart = startHTTP
		tac.HTTPCancel = cancelHTTP
	}
	return js.FuncOf(invoke)
}

func invoke(this js.Value, arguments []js.Value) any {
	return tac.InvokeTask(arguments[0].String())
}

func startHTTP(ticket int, request string) {
	symbol := js.Global().Get("Symbol").Call("for", "tachyon.tac.http")
	endpoint := js.Global().Get("Reflect").Call("get", js.Global(), symbol)
	if endpoint.Type() != js.TypeObject {
		go tac.SettleHTTP(ticket, `{"error":{"code":"HTTP_MISSING"}}`)
		return
	}
	started := endpoint.Call("start", request)
	httpStarted[ticket] = started
	var settle js.Func
	settle = js.FuncOf(func(this js.Value, arguments []js.Value) any {
		settle.Release()
		delete(httpStarted, ticket)
		tac.SettleHTTP(ticket, arguments[0].String())
		return nil
	})
	started.Get("answer").Call("then", settle)
}

func cancelHTTP(ticket int) {
	started, present := httpStarted[ticket]
	delete(httpStarted, ticket)
	if present {
		started.Call("cancel")
	}
}
