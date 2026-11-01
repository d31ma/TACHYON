# Lifecycle Drill

## Install the CLI

```bash
curl -fsSL https://tachyon.del.ma/install.sh | sh
```

This shows a staged Tachyon progress bar while installing only the standalone `ty` binary.

## Commands

```bash
ty bundle    # build the client + native artifacts
ty preview   # preview the built bundle
ty serve     # run the dev/prod server
```

The bundled output is written to `dist/`. `ty serve` detects whether the app has `client/`, `server/`, or both and serves the matching frontend, backend, or full-stack runtime.
