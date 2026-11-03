ty bundle
ls dist/web/.tachyon/tac-client.js*
ty start --port 8080
curl -sI -H 'Accept-Encoding: br' http://127.0.0.1:8080/.tachyon/tac-client.js
