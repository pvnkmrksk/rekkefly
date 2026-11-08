## 0.6.3 — Experiment-owned recording

Rekkefly can be installed and used independently. A controller can optionally launch it with a private local recording mailbox, start a dedicated video take, and stop that exact take while flight tracking continues. The mailbox uses the same protocol version, command fields and ownership receipts as HejjéTrac.

- Use `--control-dir` with `--control-instance` and an explicit configuration to enable the endpoint.
- `--no-camera-fallback` requires the selected live camera when using a camera configuration.
- Normal GUI and manual recording workflows remain available.
- The wing detector and legacy tracking output are unchanged.

[Recording mailbox contract](https://github.com/pvnkmrksk/rekkefly/blob/v0.6.3/book/src/guide/recording.md)
