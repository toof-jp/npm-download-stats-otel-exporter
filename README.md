# npm-download-stats-otel-exporter

## Run

```sh
PACKAGES="react,vue" cargo run
```

`PACKAGES` accepts a comma-separated list of npm package names.

### Exporter config

Optional environment variables:

- `OTEL_EXPORTER_OTLP_ENDPOINT` — OTLP gRPC endpoint (defaults to `http://localhost:4317`).
