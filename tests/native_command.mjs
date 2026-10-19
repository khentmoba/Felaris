const [port, expression] = process.argv.slice(2);
const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
for (const target of targets.filter(target => target.type === 'page')) {
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { socket.onopen = resolve; socket.onerror = reject; });
  let id = 0;
  const pending = new Map();
  socket.onmessage = event => {
    const message = JSON.parse(event.data);
    pending.get(message.id)?.(message);
    pending.delete(message.id);
  };
  const evaluate = expression => new Promise(resolve => {
    const request = ++id;
    pending.set(request, resolve);
    socket.send(JSON.stringify({ id: request, method: 'Runtime.evaluate', params: { expression, returnByValue: true, awaitPromise: true, userGesture: true } }));
  });
  const probe = await evaluate("Boolean(document.getElementById('url-input'))");
  if (probe.result?.result?.value) {
    const response = await evaluate(expression);
    socket.close();
    if (response.error || response.result?.exceptionDetails) throw Error(JSON.stringify(response));
    console.log(JSON.stringify(response.result.result.value ?? null));
    process.exit(0);
  }
  socket.close();
}
throw Error('Native sidebar debugging target not found');
