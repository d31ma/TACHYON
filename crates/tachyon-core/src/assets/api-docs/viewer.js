class ApiPage {
  static operations = document.getElementById('operations')
}
const node = (tag, properties = {}, children = []) => {
  const element = Object.assign(document.createElement(tag), properties)
  element.append(...children)
  return element
}
const label = (name, input) => node('label', { textContent: name }, [input])
const schema = (title, value) => [node('h3', { textContent: title }), node('pre', { textContent: JSON.stringify(value, null, 2) })]

async function boundedText(response, limit) {
  if (!response.body) return ''
  const reader = response.body.getReader()
  const decoder = new TextDecoder()
  let text = '', size = 0
  try {
    while (true) {
      const { value, done } = await reader.read()
      if (done) return text + decoder.decode()
      size += value.byteLength
      if (size > limit) throw new Error('Response exceeds the viewer size limit.')
      text += decoder.decode(value, { stream: true })
    }
  } finally { await reader.cancel() }
}

function pathInputs(route, body) {
  const parameters = new Map()
  for (const name of route.parameters ?? []) {
    const input = node('input', { type: 'text', autocomplete: 'off' })
    parameters.set(name, input)
    body.append(label(`${name} (path parameter)`, input))
  }
  return parameters
}

function namedInputs(schemaFields, body, place) {
  const inputs = new Map()
  for (const field of Object.keys(schemaFields)) {
    const name = field.endsWith('?') ? field.slice(0, -1) : field
    const input = node('input', { type: /authorization|token|key/i.test(name) ? 'password' : 'text', autocomplete: 'off' })
    inputs.set(name, input)
    body.append(label(`${name} (${place})`, input))
  }
  return inputs
}

function requestPath(route, parameters) {
  const segments = route.route.split('/').map(segment => {
    const input = segment.startsWith('_') && parameters.get(segment.slice(1))
    if (!input) return segment
    if (!input.value || input.value === '.' || input.value === '..') throw new Error('Enter every path parameter; dot segments are not allowed.')
    return encodeURIComponent(input.value)
  })
  return segments.join('/')
}

async function sendRequest(route, verb, parameters, headers, payload, query) {
  const url = new URL(requestPath(route, parameters), location.origin)
  if (url.origin !== location.origin) throw new Error('Requests must stay on this origin.')
  for (const [name, input] of query) if (input.value) url.searchParams.append(name, input.value)
  const offered = {}
  for (const [name, input] of headers) if (input.value) offered[name] = input.value
  const hasBody = payload && !['GET', 'HEAD'].includes(verb)
  if (hasBody) { JSON.parse(payload.value); offered['content-type'] = 'application/json' }
  const response = await fetch(url, { method: verb, headers: offered, ...(hasBody ? { body: payload.value } : {}), redirect: 'error', signal: AbortSignal.timeout(15000) })
  const text = await boundedText(response, 1024 * 1024)
  return `${response.status} ${response.statusText}\n\n${text}\n\nResponse schemas are documentation only.`
}

function operation(route, verb, contract) {
  const body = node('div', { className: 'body' })
  if (contract.summary) body.append(node('p', { textContent: contract.summary }))
  const request = contract.request ?? {}
  const parameters = pathInputs(route, body)
  if (request.parameters) body.append(...schema('Path parameter schema', request.parameters))
  let query = new Map()
  if (request.query) {
    body.append(...schema('Query value schema', request.query))
    query = namedInputs(request.query, body, 'query')
  }
  let headers = new Map()
  if (request.headers) {
    body.append(...schema('Request header schema', request.headers))
    headers = namedInputs(request.headers, body, 'header')
  }
  let payload
  if (request.body) {
    body.append(...schema('Request body schema', request.body))
    payload = node('textarea', { rows: 6, value: '{}' })
    body.append(label('Request body (JSON)', payload))
  }
  for (const [field, title] of [['ok', 'Expected 2xx response'], ['clientError', 'Expected 4xx response'], ['serverError', 'Expected 5xx response']]) {
    if (contract[field]) body.append(...schema(`${title} (documentation only)`, contract[field]))
  }
  const send = node('button', { type: 'button', textContent: 'Send request' })
  const output = node('pre', { hidden: true, ariaLive: 'polite' })
  send.addEventListener('click', async () => {
    send.disabled = true
    output.hidden = false
    output.textContent = 'Sending request...'
    try { output.textContent = await sendRequest(route, verb, parameters, headers, payload, query) }
    catch (error) { output.textContent = `Request failed: ${error.message}` }
    finally { send.disabled = false }
  })
  body.append(send, output)
  const summary = node('summary', {}, [node('span', { className: 'verb', textContent: verb }), node('code', { textContent: route.route })])
  return node('details', { className: 'operation' }, [summary, body])
}

try {
  const response = await fetch('../api.json', { signal: AbortSignal.timeout(15000) })
  if (!response.ok) throw new Error(`HTTP ${response.status}`)
  const document = JSON.parse(await boundedText(response, 16 * 1024 * 1024))
  if (document.contract_version !== 1 || !Array.isArray(document.routes)) throw new Error('Unsupported API document.')
  ApiPage.operations.replaceChildren()
  ApiPage.operations.removeAttribute('role')
  for (const route of document.routes) {
    const section = node('section', {}, [node('h2', { textContent: route.route })])
    if (route.summary) section.append(node('p', { textContent: route.summary }))
    for (const [verb, contract] of Object.entries(route.methods ?? {})) section.append(operation(route, verb, contract))
    ApiPage.operations.append(section)
  }
  if (!ApiPage.operations.childElementCount) ApiPage.operations.textContent = 'No routes declare an API contract.'
} catch (error) { ApiPage.operations.textContent = `Cannot load the API document: ${error.message}. Check api.json and rebuild the application.` }
