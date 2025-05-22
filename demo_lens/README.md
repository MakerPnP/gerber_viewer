# Gerber Viewer Lens Demo

An interactive demonstration application showcasing the integration of `egui_lens` reactive event logging with the `gerber_viewer` library. This demo provides a comprehensive example of how to implement real-time event logging, property manipulation, and PCB design rule checking in an egui-based application.

![Demo Screenshot](../assets/egui_lens_component/egui_lens_demo.gif)

## Features

### Core Functionality
- **Interactive Gerber Visualization**: Real-time rendering of Gerber files with customizable display options
- **Reactive Event Logging**: Live logging of all user interactions and state changes using `egui_lens`
- **Property Controls**: Intuitive UI controls for manipulating gerber display properties
- **Design Rule Checking (DRC)**: Simulated PCB manufacturer rule validation

### Visual Features
- **Animated Rotation**: Configurable rotation speed for dynamic visualization
- **Zoom Controls**: Adjustable zoom factor with instant preview
- **Mirroring Options**: X and Y axis mirroring toggles
- **Offset Controls**: Both center and design offset adjustments
- **Color Customization**: Unique colors for different shapes and polygon numbering
- **Visual Overlays**: Bounding boxes, crosshairs, and markers for reference

### Event Logging Categories
The demo uses custom log types with distinct colors:
- `rotation` - Orange (#E67E22) - Rotation speed changes
- `zoom` - Blue (#2980B9) - Zoom factor adjustments
- `center_offset` - Purple (#8E44AD) - Center offset modifications
- `design_offset` - Green (#27AE60) - Design offset changes
- `mirror` - Red (#C0392B) - Mirroring state changes
- `display` - Yellow (#F1C40F) - Display option toggles
- `drc` - Light Purple (#9B59B6) - Design rule check operations

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

### Controls

**Left Panel - Properties:**
- **Rotation Speed**: Slider to control rotation animation (0-180 deg/s)
- **Zoom Factor**: Slider to adjust zoom level (0.1-2.0x)
- **Apply Zoom**: Button to reset view with new zoom factor
- **Enable Unique Colors**: Checkbox for shape-specific coloring
- **Enable Polygon Numbering**: Checkbox to display polygon indices
- **Mirroring**: X and Y axis mirror toggles
- **Center Offset**: X/Y offset controls for rotation center
- **Design Offset**: X/Y offset controls for design positioning

**Design Rule Check Section:**
- **PCB Manufacturer Rules**: Collapsible section with manufacturer presets
  - JLC PCB Rules
  - PCB WAY Rules
  - Advanced Circuits Rules
- **Run DRC**: Execute design rule validation
- **Clear Ruleset**: Remove currently loaded ruleset

**Central Panel - Viewer:**
- Drag to pan the view
- Visual indicators:
  - Blue crosshair: Origin position
  - Gray crosshair: Center position
  - Red outline: Transformed bounding box
  - Green outline: Rotated gerber outline
  - Orange marker: Design offset position
  - Purple marker: Design origin position

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

## Architecture

### State Management
The demo uses `egui_mobius_reactive` for reactive state management:
- `Dynamic<ReactiveEventLoggerState>` - Logger state container
- `Dynamic<LogColors>` - Color configuration with change detection
- Automatic persistence of color preferences

### Event Flow
1. User interacts with UI controls
2. Change handlers log events with appropriate categories
3. Logger displays events with custom colors
4. State changes trigger reactive updates
5. Gerber view updates in real-time

### Platform Integration
The demo includes platform-specific modules:
- `banner` - Application banner and version info
- `details` - System information display
- `parameters` - Configuration constants

## Example Use Cases

1. **PCB Design Validation**: Load gerber files and run DRC checks against manufacturer specifications
2. **Interactive Visualization**: Rotate and examine PCB designs from different angles
3. **Event Monitoring**: Track all user interactions for debugging or usage analytics
4. **UI Development**: Reference implementation for integrating `egui_lens` in applications

## Development

### Adding New Log Types
1. Define a new constant in `DemoLensApp`:
   ```rust
   const LOG_TYPE_CUSTOM: &'static str = "custom";
   ```

2. Configure color in `configure_custom_log_colors_if_missing`:
   ```rust
   colors_value.set_custom_color(Self::LOG_TYPE_CUSTOM, egui::Color32::from_rgb(R, G, B));
   ```

3. Use in event handlers:
   ```rust
   logger.log_custom(Self::LOG_TYPE_CUSTOM, "Custom event occurred");
   ```

### Extending DRC Functionality
The current DRC implementation is simulated. To add real validation:
1. Implement rule definitions in a separate module
2. Add gerber analysis logic
3. Replace simulated messages with actual validation results

## License

This demo is part of the gerber_viewer project. See the parent directory for license information.