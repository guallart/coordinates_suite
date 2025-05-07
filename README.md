# CoordinatesSuite

A desktop application for converting and visualizing coordinates between UTM and Latitude/Longitude formats with clipboard integration, CSV/KML export, and OpenStreetMap background.

---

## Features

- **Clipboard Integration:** Paste coordinates directly from your clipboard.
- **Automatic Format Detection:** Detects if your input is UTM or Lat/Lon and converts accordingly.
- **Bidirectional Conversion:** Instantly convert between UTM and Lat/Lon.
- **Map Visualization:** See your coordinates on an interactive OpenStreetMap view.
- **Zone & Hemisphere Selection:** Adjust UTM zone and hemisphere as needed.
- **Export Options:** Export to CSV (UTM or Lat/Lon) or KML.
- **Copy Results:** Copy converted coordinates back to your clipboard.

---

## Usage

1. **Paste Coordinates:**  
   Copy a list of coordinates (either UTM or Lat/Lon) to your clipboard.  
   Click **"Read from clipboard"** in the app.

2. **Conversion:**  
   The app will auto-detect the format and convert.  
   You can manually switch conversion direction if needed.

3. **Visualization:**  
   The map will center and zoom to your points.

4. **Export:**  
   Use the export buttons to save coordinates as CSV or KML.

5. **Copy:**  
   Copy converted coordinates to clipboard for use elsewhere.

---

## Supported Formats

- **Latitude/Longitude:**  
  - Decimal degrees, e.g., `41.651285, -0.869147`  
  - Tab, comma, or space separated (any separator is supported, actually)

- **UTM:**  
  - Easting/Northing, e.g., `676000, 4610000`  
  - Tab, comma, or space separated

---

## Building & Running

**Prerequisites:**  
- [Rust](https://www.rust-lang.org/tools/install)
- [cargo](https://doc.rust-lang.org/cargo/)

**Build:**
```bash
cargo build --release
```

**Run:**
```bash
cargo run --release
```

---

## Credits

- OpenStreetMap for map tiles.
- Rust and the open source crates community.

---

## Contributing

Pull requests and issues welcome!
