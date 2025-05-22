# Gerber Viewer Lens Demo

An interactive demonstration application showcasing the integration of `egui_lens` reactive event logging with the `gerber_viewer` library. This demo provides a comprehensive example of how to implement real-time event logging, property manipulation, and PCB design rule checking in an egui-based application.

![Demo Screenshot](../assets/egui_lens_component/egui_lens_demo.gif)

## Features

There are numerous features in the design such as the ability to zooming and panning
with the mouse, along with setting various offsets and mirroring throught the GUI. These are all part of the native `gerber_viewer` as part of the `MakerPnP` project. 

The reactive logger is an instance of `egui_lens` and is setup to log all GUI events, where one can customize the colors
of each event through the `Log Colors` button and have that chosen color persist when exiting and restarting the 
application. 

One can use the `Filters` button to filter on INFO, WARNING, ERROR, or a CUSTOM event, or enter an expression/regex and filter on that. 

Additionally the `Save Logs` button will bring up an RFD panel and allow
saving to a specified file. 

The `System Info` button allows one to see the operating system details of the 
machine that one is on, along with a banner display. 


## Dependencies

The demo requires the following crates:
- `egui` & `eframe` - UI framework
- `gerber_viewer` - Core gerber parsing and rendering
- `egui_lens` - Reactive event logging component
- `egui_mobius` & `egui_mobius_reactive` - Reactive state management
- Additional utilities for system info, networking, and file operations

## Usage

### Running the Demo

```bash
cd demo_lens
cargo run
```


## Configuration

### Color Persistence
The demo automatically saves custom log colors to:
- Linux/macOS: `~/.config/gerber_viewer/log_colors.json`
- Windows: `%APPDATA%\gerber_viewer\log_colors.json`

Colors are loaded on startup and saved whenever they're modified.

### Constants
Key configuration constants in `main.rs`:
```rust
const ENABLE_UNIQUE_SHAPE_COLORS: bool = true;
const ENABLE_POLYGON_NUMBERING: bool = false;
const ZOOM_FACTOR: f32 = 0.50;
const ROTATION_SPEED_DEG_PER_SEC: f32 = 45.0;
const INITIAL_ROTATION: f32 = 45.0_f32.to_radians();
const MIRRORING: [bool; 2] = [false, false];
const CENTER_OFFSET: Vector = Vector::new(15.0, 20.0);
const DESIGN_OFFSET: Vector = Vector::new(-5.0, -10.0);
const MARKER_RADIUS: f32 = 2.5;
```


## License

This demo is part of the gerber_viewer project. See the parent directory for license information.