## 0.6.5 — Direct macOS camera capture

Rekkefly now reads a macOS camera's available modes before opening it. If
AVFoundation rejects a listed mode, Rekkefly tries the next ranked mode.

On Apple silicon, a physical FaceTime HD camera opened directly at 640×480
and 30 fps. MATREX VR started the experiment, archived all 90 frames from
its own take on Stop, and the same tracker kept sending measurements afterward.

[Camera guide](https://pvnkmrksk.github.io/rekkefly/docs/guide/cameras.html) ·
[Recording guide](https://pvnkmrksk.github.io/rekkefly/docs/guide/recording.html#experiment-owned-recording)
