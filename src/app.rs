use clipboard_win::{formats, get_clipboard, set_clipboard};
use eframe::egui::{Button, ComboBox, DragValue, Grid};
use eframe::{App, egui};
use egui::{Color32, Stroke};
use egui_extras::{Column, TableBuilder};
use itertools::{Itertools, izip};
use regex::Regex;
use std::fmt;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use utm;
use walkers::{HttpTiles, Map, MapMemory, Position, Projector, lon_lat, sources::OpenStreetMap};

const DEFAULT_LAT: f64 = 41.651285;
const DEFAULT_LON: f64 = -0.869147;

// https://wiki.openstreetmap.org/wiki/Zoom_levels
const TILE_WIDTHS: [f64; 21] = [
    360.0, 180.0, 90.0, 45.0, 22.5, 11.25, 5.625, 2.813, 1.406, 0.703, 0.352, 0.176, 0.088, 0.044,
    0.022, 0.011, 0.005, 0.003, 0.001, 0.0005, 0.00025,
];

fn parse_number_pairs(input: &str) -> Vec<[f32; 2]> {
    let re = Regex::new(r"([+-]?\d+([.,]\d+)?([eE][+-]?\d+)?)").unwrap();

    let numbers: Vec<f32> = re
        .find_iter(input)
        .filter_map(|m| m.as_str().replace(',', ".").parse::<f32>().ok())
        .collect();

    numbers
        .chunks_exact(2)
        .map(|chunk| [chunk[0], chunk[1]])
        .collect()
}

#[derive(PartialEq, Debug, Clone)]
enum ConversionMode {
    UTMtoLatLon,
    LatLontoUTM,
}

impl fmt::Display for ConversionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionMode::UTMtoLatLon => write!(f, "UTM to Lat/Lon"),
            ConversionMode::LatLontoUTM => write!(f, "Lat/Lon to UTM"),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
enum Hemisphere {
    North,
    South,
}

impl fmt::Display for Hemisphere {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Hemisphere::North => write!(f, "North"),
            Hemisphere::South => write!(f, "South"),
        }
    }
}

struct ConversionError;

pub struct CoordinatesSuite {
    conversion_mode: ConversionMode,
    coords_geo: Vec<[f32; 2]>,
    coords_utm: Vec<[f32; 2]>,
    utm_zone: u8,
    hemisphere: Hemisphere,
    tiles: HttpTiles,
    map_memory: MapMemory,
}

impl CoordinatesSuite {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut coords_suite = CoordinatesSuite {
            conversion_mode: ConversionMode::LatLontoUTM,
            coords_geo: vec![],
            coords_utm: vec![],
            utm_zone: 30,
            hemisphere: Hemisphere::North,
            tiles: HttpTiles::new(OpenStreetMap, cc.egui_ctx.clone()),
            map_memory: MapMemory::default(),
        };

        coords_suite.parse_coordinates();
        coords_suite.move_map_to_points();
        coords_suite
    }

    fn compute_geo_coords(&mut self) -> Result<(), ConversionError> {
        if self.coords_utm.is_empty() {
            return Err(ConversionError);
        }

        let zone_letter = match self.hemisphere {
            Hemisphere::North => 'N',
            Hemisphere::South => 'J', // the utm function only checks if letter >= 'N'
        };

        self.coords_geo = self
            .coords_utm
            .iter()
            .map(|&[x, y]| {
                match utm::wsg84_utm_to_lat_lon(x.into(), y.into(), self.utm_zone, zone_letter) {
                    Ok((lat, lon)) => Ok([lon as f32, lat as f32]),
                    Err(_) => Err(ConversionError),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(())
    }

    fn compute_utm_coords(&mut self) -> Result<(), ConversionError> {
        if self.coords_geo.is_empty() {
            return Err(ConversionError);
        }

        let [lon, lat] = self.coords_geo[0];
        self.utm_zone = utm::lat_lon_to_zone_number(lat.into(), lon.into());
        self.hemisphere = if lat >= 0.0 {
            Hemisphere::North
        } else {
            Hemisphere::South
        };

        self.coords_utm = self
            .coords_geo
            .iter()
            .map(|&[lon, lat]| utm::to_utm_wgs84_no_zone(lat.into(), lon.into()))
            .map(|(y, x, _mc)| [x as f32, y as f32])
            .collect();

        Ok(())
    }

    fn parse_coordinates(&mut self) {
        let clipboard_content = match get_clipboard(formats::Unicode) {
            Ok(content) => content,
            Err(e) => {
                println!("{:?}", e);
                "".to_string()
            }
        };

        let coords = parse_number_pairs(&clipboard_content);

        if coords.is_empty() {
            return;
        }

        self.conversion_mode = if coords[0][1] > 1000.0 {
            ConversionMode::UTMtoLatLon
        } else {
            ConversionMode::LatLontoUTM
        };

        match self.conversion_mode {
            ConversionMode::UTMtoLatLon => {
                self.coords_utm = coords;
                match self.compute_geo_coords() {
                    Ok(()) => println!("Conversion succesful"),
                    Err(_) => println!("Conversion failed"),
                };
            }
            ConversionMode::LatLontoUTM => {
                self.coords_geo = coords.iter().map(|&[lon, lat]| [lat, lon]).collect();
                match self.compute_utm_coords() {
                    Ok(()) => println!("Conversion succesful"),
                    Err(_) => println!("Conversion failed"),
                };
            }
        }
    }

    fn calculate_zoom_level(&self) -> f64 {
        if self.coords_geo.len() == 1 {
            return 15.0;
        }

        let mut min_lat = f32::MAX;
        let mut max_lat = f32::MIN;
        let mut min_lon = f32::MAX;
        let mut max_lon = f32::MIN;

        for &[lon, lat] in &self.coords_geo {
            min_lat = min_lat.min(lat);
            max_lat = max_lat.max(lat);
            min_lon = min_lon.min(lon);
            max_lon = max_lon.max(lon);
        }

        let lat_range = max_lat - min_lat;
        let lon_range = max_lon - min_lon;
        let range = 1.3 * lat_range.max(lon_range);

        TILE_WIDTHS
            .into_iter()
            .enumerate()
            .find(|(_i, tw)| (range as f64 - tw) > 0.0)
            .map_or(0, |(i, _)| i) as f64
    }

    fn move_map_to_points(&mut self) {
        if self.coords_geo.is_empty() {
            return;
        }

        let n_points = self.coords_geo.len() as f32;
        let (center_lon, center_lat) = if n_points > 0.0 {
            let lat = self.coords_geo.iter().map(|[_lon, lat]| *lat).sum::<f32>() / n_points;
            let lon = self.coords_geo.iter().map(|[lon, _lat]| *lon).sum::<f32>() / n_points;
            (lon as f64, lat as f64)
        } else {
            (DEFAULT_LAT, DEFAULT_LON)
        };

        self.map_memory
            .center_at(Position::new(center_lon, center_lat));

        let zoom_level = self.calculate_zoom_level();
        let _ = self.map_memory.set_zoom(zoom_level);
    }

    fn copy_coords_geo_to_clipboard(&self) {
        let data = self
            .coords_geo
            .iter()
            .map(|&[lon, lat]| format!("{}\t{}", lat, lon))
            .join("\n");

        match set_clipboard(formats::Unicode, data) {
            Ok(()) => println!("Copied to clipboard"),
            Err(e) => println!("Failed to copy to clipboard: {}", e),
        };
    }

    fn copy_coords_utm_to_clipboard(&self) {
        let data = self
            .coords_utm
            .iter()
            .map(|&[x, y]| format!("{}\t{}", x, y))
            .join("\n");

        match set_clipboard(formats::Unicode, data) {
            Ok(()) => println!("Copied to clipboard"),
            Err(e) => println!("Failed to copy to clipboard: {}", e),
        };
    }

    fn export_csv_utm(&self, outfile: &PathBuf) -> Result<(), std::io::Error> {
        let mut file = File::create(outfile)?;
        writeln!(file, "Easting\tNorthing")?;
        for &[x, y] in &self.coords_utm {
            writeln!(file, "{}\t{}", x, y)?;
        }
        println!("UTM coordinates exported to {:?}", outfile);
        Ok(())
    }

    fn export_csv_latlon(&self, outfile: &PathBuf) -> Result<(), std::io::Error> {
        let mut file = File::create(outfile)?;
        writeln!(file, "Latitude\tLongitude")?;
        for &[lon, lat] in &self.coords_geo {
            writeln!(file, "{}\t{}", lat, lon)?;
        }
        println!("UTM coordinates exported to {:?}", outfile);
        Ok(())
    }

    fn export_kmz(&self, outfile: &PathBuf) -> Result<(), std::io::Error> {
        let kml_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <kml xmlns="http://www.opengis.net/kml/2.2">
                <Document>
                    <name>Coordinates</name>
                    {}
                </Document>
            </kml>"#,
            self.coords_geo
                .iter()
                .map(|&[lon, lat]| format!(
                    r#"<Placemark>
                        <Point>
                            <coordinates>{},{},0</coordinates>
                        </Point>
                    </Placemark>"#,
                    lon, lat
                ))
                .join("\n")
        );

        let mut file = File::create(outfile)?;
        file.write_all(kml_content.as_bytes())?;

        println!("KML file exported to {:?}", outfile);
        Ok(())
    }
}

impl App for CoordinatesSuite {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        eframe::egui::SidePanel::left("left_panel")
            .show_separator_line(true)
            .exact_width(420.0)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                Grid::new("utm_inputs")
                    .striped(false)
                    .num_columns(3)
                    .min_col_width(20.0)
                    .spacing([20.0, 7.0])
                    .show(ui, |ui| {
                        ui.label("Conversion mode");
                        ComboBox::new("conversion_mode", "")
                            .width(130.0)
                            .selected_text(format!("{}", self.conversion_mode))
                            .show_ui(ui, |ui| {
                                for mode in
                                    [ConversionMode::LatLontoUTM, ConversionMode::UTMtoLatLon]
                                {
                                    if ui
                                        .selectable_value(
                                            &mut self.conversion_mode,
                                            mode.clone(),
                                            format!("{}", mode),
                                        )
                                        .clicked()
                                    {
                                        self.parse_coordinates();
                                        self.move_map_to_points();
                                    }
                                }
                            });

                        let read_clip_button =
                            ui.add_sized([130., 20.], Button::new("Read from clipboard"));
                        if read_clip_button.clicked() {
                            self.parse_coordinates();
                            self.move_map_to_points();
                        }
                        ui.end_row();

                        ui.label("UTM Zone");
                        let previous_utm_zone = self.utm_zone;
                        ui.add_enabled_ui(
                            matches!(self.conversion_mode, ConversionMode::UTMtoLatLon),
                            |ui| {
                                ui.add_sized(
                                    [130., 20.],
                                    DragValue::new(&mut self.utm_zone).range(1..=60),
                                );
                            },
                        );

                        if self.utm_zone != previous_utm_zone {
                            self.parse_coordinates();
                            self.move_map_to_points();
                        }

                        let move_button =
                            ui.add_sized([130., 20.], Button::new("Move map to points"));
                        if move_button.clicked() {
                            self.move_map_to_points();
                            self.move_map_to_points();
                        }

                        ui.end_row();

                        ui.label("Hemisphere");
                        ui.add_enabled_ui(
                            matches!(self.conversion_mode, ConversionMode::UTMtoLatLon),
                            |ui| {
                                ComboBox::new("hemisphere", "")
                                    .width(130.0)
                                    .selected_text(format!("{}", self.hemisphere))
                                    .show_ui(ui, |ui| {
                                        for hemisphere in [Hemisphere::North, Hemisphere::South] {
                                            if ui
                                                .selectable_value(
                                                    &mut self.hemisphere,
                                                    hemisphere.clone(),
                                                    format!("{}", hemisphere),
                                                )
                                                .clicked()
                                            {
                                                self.parse_coordinates();
                                                self.move_map_to_points();
                                            }
                                        }
                                    });
                            },
                        );

                        let kmz_button = ui.add_sized([130., 20.], Button::new("Export to kmz"));
                        if kmz_button.clicked() {
                            if let Some(outfile) = rfd::FileDialog::new()
                                .add_filter("KML files", &["kml"])
                                .set_file_name("coordinates.kml")
                                .save_file()
                            {
                                match self.export_kmz(&outfile) {
                                    Ok(()) => println!("File exported"),
                                    Err(_) => println!("Failed to export file"),
                                };
                            } else {
                                println!("No file selected.");
                            }
                        }

                        ui.end_row();

                        ui.label(""); //dummy
                        ui.label(""); //dummy
                        let csv_utm_button =
                            ui.add_sized([130., 20.], Button::new("Export UTM to csv"));
                        if csv_utm_button.clicked() {
                            if let Some(outfile) = rfd::FileDialog::new()
                                .add_filter("CSV files", &["csv"])
                                .set_file_name("coordinates.csv")
                                .save_file()
                            {
                                match self.export_csv_utm(&outfile) {
                                    Ok(()) => println!("File exported"),
                                    Err(_) => println!("Failed to export file"),
                                };
                            } else {
                                println!("No file selected.");
                            }
                        }
                        ui.end_row();

                        ui.label(""); //dummy
                        ui.label(""); //dummy
                        let csv_latlon_button =
                            ui.add_sized([130., 20.], Button::new("Export Lat/Lon to csv"));
                        if csv_latlon_button.clicked() {
                            if let Some(outfile) = rfd::FileDialog::new()
                                .add_filter("CSV files", &["csv"])
                                .set_file_name("coordinates.csv")
                                .save_file()
                            {
                                match self.export_csv_latlon(&outfile) {
                                    Ok(()) => println!("File exported"),
                                    Err(_) => println!("Failed to export file"),
                                };
                            } else {
                                println!("No file selected.");
                            }
                        }
                        ui.end_row();
                    });

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui.button("Copy").clicked() {
                        self.copy_coords_geo_to_clipboard();
                    }
                    ui.add_space(160.0);
                    if ui.button("Copy").clicked() {
                        self.copy_coords_utm_to_clipboard();
                    }
                });
                ui.add_space(5.0);
                TableBuilder::new(ui)
                    .striped(true)
                    .column(Column::exact(75.0))
                    .column(Column::exact(75.0))
                    .column(Column::exact(30.0))
                    .column(Column::exact(75.0))
                    .column(Column::exact(75.0))
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.label("Latitude");
                        });
                        header.col(|ui| {
                            ui.label("Longitude");
                        });
                        header.col(|ui| {
                            ui.label("");
                        }); // dummy
                        header.col(|ui| {
                            ui.label("Easting");
                        });
                        header.col(|ui| {
                            ui.label("Northing");
                        });
                    })
                    .body(|mut body| {
                        for (geoc, utmc) in izip!(&self.coords_geo, &self.coords_utm) {
                            body.row(20.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(format!("{:.5}", geoc[1]));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{:.5}", geoc[0]));
                                });
                                row.col(|ui| {
                                    ui.label("");
                                }); // dummy
                                row.col(|ui| {
                                    ui.label(format!("{}", utmc[0] as u64));
                                });
                                row.col(|ui| {
                                    ui.label(format!("{}", utmc[1] as u64));
                                });
                            });
                        }
                    });
            });

        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let map_response = ui.add(Map::new(
                Some(&mut self.tiles),
                &mut self.map_memory,
                lon_lat(DEFAULT_LON, DEFAULT_LAT),
            ));

            if !self.coords_geo.is_empty() {
                let projector = Projector::new(
                    map_response.rect,
                    &self.map_memory,
                    Position::new(self.coords_geo[0][0] as f64, self.coords_geo[0][1] as f64),
                );

                let painter = ui.painter_at(map_response.rect);
                for &[lon, lat] in &self.coords_geo {
                    let pos = Position::new(lon as f64, lat as f64);
                    let pos_proj = projector.project(pos);
                    painter.circle(
                        pos_proj.to_pos2(),
                        5.0,
                        Color32::RED,
                        Stroke::new(1.0, Color32::BLACK),
                    );
                }
            }
        });
    }
}
