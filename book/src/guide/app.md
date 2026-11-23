# The window

![The Rekkefly window tracking the Bogong moth, with the debug traces open](../images/main-window.png)

The preview shows the camera image, draggable markers and live angle readouts.
The title shows the rig's label, source and ZMQ port. The footer shows capture
speed, output and recording status. Hover over a warning to read its details.

## Toolbar

| Control | What it does |
| --- | --- |
| **Start / Stop** | Starts or stops acquisition |
| **Record** | Starts and stops video recording ([formats](recording.md)) |
| **Save** | Saves this rig's configuration |
| **Settings** | Opens tracking, camera, output and recording controls |
| **Diagnostics** | Shows the angle traces and the selected region's angular profile |
| **New window** | Opens another Rekkefly window for the next rig, with its own camera and ports |

The toolbar wraps in narrow windows. **Settings** sits beside the preview in a
wide window and uses the workspace in a compact window. Click **Settings**
again to return to the preview and any open diagnostics.

## Editing markers

Drag the dots; see [Placing the markers](../getting-started/markers.md). The
wings mirror each other until **Asymmetric wings** is on. Edits apply to the very
next frame.

## Settings

![Settings docked beside the camera preview](../images/settings.png)

Use the **Track and draw** checkboxes to enable tracking and overlays by body
part. **Asymmetric wings** allows independent wing edits. For a brightly lit
insect against a dark background, enable **Bright insect on dark background**.
**Ignore legs and antennae** selects the robust wing detector for both wings.

| Section | Contents |
| --- | --- |
| **Detector and markers** | Select **Left**, **Right**, **Head** or **Abdomen** to edit its detector, edge, threshold and geometry and see the live edge strength |
| **Background** | **Save BG**, **Load saved background**, subtraction and difference preview. Capture an empty scene for the reference image |
| **Camera / input** | The source: a camera (with a list of cameras), a video file, a network stream or the simulated fly. **Back to saved source** and **Save as this config's source** for a video opened for one run |
| **Instance / output** | The instance label, the ZMQ publisher and its bind address, the UDP output's sending and receiving addresses, and the JSONL file |
| **Video / saved state** | **Automatically save configuration**, the recording folder, format, MJPEG quality, write queue and file size |
| **Sampling** | Angular bins, radial samples and the preview frame rate |

At the top, **Restart acquisition** applies source, label and output
changes, and **Reload config** discards unsaved edits by reading the file
again. Greyed fields are set for this run by the command line or the rig
launcher, and cannot be changed here.

## Debug traces

Click **Diagnostics**, or start with `--show plots`, to see two plots below the image.

- **Wing angles · last 8 s**: left (red), right (green) and **L − R** (white),
  from −90° to +90°. Spikes in L − R that the insect did not make are a sign of
  misplaced sectors or something in the sector that is not wing.
- **Angular profile** of the selected wing across its sector: thin is the raw
  intensity; bold is what the robust detector ranks edges on, with legs and
  antennae removed. Vertical lines mark the edges found; the labelled one is
  reported, with its angle.

## Saving

Edits are written 300 ms after you stop dragging, and when you close the window.
Writes are atomic (a new file replaces the old one in a single step), so a crash
cannot leave a half-written config, and the file's comments are kept. Turn
**Automatically save configuration** off to experiment without saving; edits then
last until the window closes.

Only one window may use a config file at a time. A second window started without
`--config` takes the next rig's config instead (`camera-b.toml`, then
`camera-c.toml`, which it creates).

## When something is wrong

Problems that stop tracking are shown in the window instead of closing it:

| Banner | Meaning |
| --- | --- |
| Amber: the example is playing | No camera could be opened; the banner says why. Click **Retry camera** |
| **Outputs off: Bind output port …** | Another program or rig uses that port. Tracking continues in the window; choose another port in **Instance / output** |
| **Recording stopped** | The disk could not keep up, or is full. Tracking continues |

Headless runs and rigs started from a manifest stop with an error instead, so an
experiment never runs silently without its outputs.
