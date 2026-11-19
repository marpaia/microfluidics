//! The files masks are written as.

use std::io;

use microfluidics_core::model::LayerSpec;

use crate::geometry::Point;
use crate::mask::{Mask, MaskSet};

/// A file format for masks.
pub trait Format {
    /// Whether the format takes a file for each layer, or one for the chip.
    fn per_layer(&self) -> bool;
    /// The name of the file for `layer`, or for the chip when the format takes one file.
    fn file_name(&self, layer: Option<&LayerSpec>) -> String;
    /// Write `mask`, or every mask when the format takes one file.
    fn write(&self, masks: &MaskSet, mask: Option<&Mask>, out: &mut dyn io::Write) -> io::Result<()>;
}

/// GDSII: every layer in one file, with structures and translates.
pub struct Gds;

/// DXF (the R12 dialect): one drawing for each layer, flat, in micrometres.
pub struct Dxf;

/// A drawing of every layer together, for looking over in a browser.
pub struct Svg;

/// The colour of each layer in the SVG drawing, shallowest first.
const COLOURS: [&str; 4] = ["#d1495b", "#30638e", "#00798c", "#edae49"];

impl Format for Gds {
    fn per_layer(&self) -> bool {
        false
    }

    fn file_name(&self, _layer: Option<&LayerSpec>) -> String {
        "chip.gds".to_owned()
    }

    fn write(&self, masks: &MaskSet, _mask: Option<&Mask>, out: &mut dyn io::Write) -> io::Result<()> {
        out.write_all(&crate::gds::write(masks))
    }
}

impl Format for Dxf {
    fn per_layer(&self) -> bool {
        true
    }

    fn file_name(&self, layer: Option<&LayerSpec>) -> String {
        layer.map_or_else(
            || "mask.dxf".to_owned(),
            |layer| format!("mask_{}_{}.dxf", layer.gds, layer.name),
        )
    }

    fn write(&self, masks: &MaskSet, mask: Option<&Mask>, out: &mut dyn io::Write) -> io::Result<()> {
        // A header naming the R12 dialect: without one, layout tools do not take the file for DXF.
        out.write_all(b"0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1009\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n")?;
        for mask in masks
            .masks
            .iter()
            .filter(|held| mask.is_none_or(|mask| mask.layer.name == held.layer.name))
        {
            let layer = mask.layer.name.to_uppercase();
            for rectangle in masks.flattened(mask) {
                write!(out, "0\nPOLYLINE\n8\n{layer}\n66\n1\n70\n1\n")?;
                for point in &rectangle.polygon {
                    write!(
                        out,
                        "0\nVERTEX\n8\n{layer}\n10\n{:.4}\n20\n{:.4}\n",
                        point.x * 1e6,
                        point.y * 1e6
                    )?;
                }
                out.write_all(b"0\nSEQEND\n")?;
            }
        }
        out.write_all(b"0\nENDSEC\n0\nEOF\n")
    }
}

impl Format for Svg {
    fn per_layer(&self) -> bool {
        false
    }

    fn file_name(&self, _layer: Option<&LayerSpec>) -> String {
        "preview.svg".to_owned()
    }

    fn write(&self, masks: &MaskSet, _mask: Option<&Mask>, out: &mut dyn io::Write) -> io::Result<()> {
        // Micrometres, with Y up as on the chip.
        let (width, height) = (masks.size[0] * 1e6, masks.size[1] * 1e6);
        writeln!(
            out,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\">"
        )?;
        writeln!(out, "<g transform=\"translate(0 {height}) scale(1 -1)\">")?;
        writeln!(
            out,
            "<rect width=\"{width}\" height=\"{height}\" fill=\"#f4f4f4\" stroke=\"#555\" stroke-width=\"20\"/>"
        )?;
        // Deepest first, so that shallow features drawn into deep ones show.
        for (index, mask) in masks.masks.iter().enumerate().rev() {
            let colour = COLOURS[index % COLOURS.len()];
            writeln!(
                out,
                "<g fill=\"{colour}\" fill-opacity=\"0.8\"><title>{}, {:.2} um</title>",
                mask.layer.name,
                mask.layer.height * 1e6
            )?;
            for rectangle in masks.flattened(mask) {
                let points: Vec<String> = rectangle
                    .polygon
                    .iter()
                    .map(|point| format!("{:.3},{:.3}", point.x * 1e6, point.y * 1e6))
                    .collect();
                writeln!(out, "<polygon points=\"{}\"/>", points.join(" "))?;
            }
            writeln!(out, "</g>")?;
        }
        writeln!(out, "</g>\n</svg>")
    }
}

/// The closed polylines of a DXF drawing written by [`Dxf`], in micrometres.
pub fn read_dxf(text: &str) -> Result<Vec<Vec<Point>>, String> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    if !lines.len().is_multiple_of(2) {
        return Err("the drawing does not end on a whole code and value pair".to_owned());
    }
    let mut outlines: Vec<Vec<Point>> = Vec::new();
    let (mut vertex, mut x) = (false, None);
    for pair in lines.chunks(2) {
        let number = || {
            pair[1]
                .parse::<f64>()
                .map_err(|_| format!("{:?} is not a coordinate", pair[1]))
        };
        match (pair[0], pair[1]) {
            ("0", "POLYLINE") => outlines.push(Vec::new()),
            ("0", entity) => vertex = entity == "VERTEX",
            ("10", _) if vertex => x = Some(number()?),
            ("20", _) if vertex => {
                let x = x.take().ok_or("a vertex has a Y before its X")?;
                outlines
                    .last_mut()
                    .ok_or("a vertex comes before any polyline")?
                    .push(Point::new(x, number()?));
            }
            _ => {}
        }
    }
    Ok(outlines)
}

#[cfg(test)]
mod tests {
    use crate::format::{Dxf, Format, Svg, read_dxf};
    use crate::mask::prepare;

    #[test]
    fn a_drawing_reads_back_as_the_mask_it_was_written_from() {
        let masks = prepare(&crate::fixtures::arrayed(), None);
        for mask in &masks.masks {
            let mut bytes = Vec::new();
            Dxf.write(&masks, Some(mask), &mut bytes).unwrap();
            let read = read_dxf(&String::from_utf8(bytes).unwrap()).unwrap();
            let drawn = masks.flattened(mask);
            assert_eq!(read.len(), drawn.len());
            for (read, drawn) in read.iter().zip(&drawn) {
                assert!(
                    read.iter()
                        .zip(&drawn.polygon)
                        .all(|(a, b)| (a.x - b.x * 1e6).abs() < 1e-4 && (a.y - b.y * 1e6).abs() < 1e-4)
                );
            }
        }
        assert_eq!(Dxf.file_name(Some(&masks.masks[0].layer)), "mask_1_trench.dxf");
        assert!(read_dxf("0\nVERTEX\n20\n1.0\n").is_err());
    }

    #[test]
    fn the_preview_draws_every_outline_of_every_layer() {
        let masks = prepare(&crate::fixtures::arrayed(), None);
        let mut bytes = Vec::new();
        Svg.write(&masks, None, &mut bytes).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert_eq!(text.matches("<polygon").count(), 101);
        assert!(text.contains("viewBox=\"0 0 2000 2000\""));
    }
}
