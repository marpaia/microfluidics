//! Writing a chip's masks to disk, for whoever makes them.

use std::{collections::BTreeMap, fs, io, path::Path};

use microfluidics_core::{Check, Finding};

use crate::format::{Dxf, Format, Gds, Svg, read_dxf};
use crate::mask::MaskSet;
use crate::maskspec::Tone;
use crate::process::Process;
use crate::rectangle::area;

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("{path}: {source}")]
    Io { path: String, source: io::Error },
}

/// What an export wrote, and what reading it back found.
#[derive(Debug, Clone, PartialEq)]
pub struct Exported {
    /// Every file written, by name within the directory.
    pub files: Vec<String>,
    /// `gds-readback` and `dxf-readback`.
    pub checks: Vec<Check>,
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), ExportError> {
    let path = dir.join(name);
    fs::write(&path, bytes).map_err(|source| ExportError::Io {
        path: path.display().to_string(),
        source,
    })
}

fn read(dir: &Path, name: &str) -> Result<Vec<u8>, ExportError> {
    let path = dir.join(name);
    fs::read(&path).map_err(|source| ExportError::Io {
        path: path.display().to_string(),
        source,
    })
}

/// Write `masks` into `dir`: `chip.gds` with every layer, a DXF drawing for each layer, a
/// preview of them together, and a README for whoever makes them. Each mask file is read back
/// from disk and compared with what was to be written.
pub fn export(masks: &MaskSet, process: Option<&Process>, dir: &Path) -> Result<Exported, ExportError> {
    fs::create_dir_all(dir).map_err(|source| ExportError::Io {
        path: dir.display().to_string(),
        source,
    })?;
    // What is written is each mask as its process has it made: mirrored where it says so.
    let masks = &masks.as_written();
    let mut files = Vec::new();
    let formats: [&dyn Format; 3] = [&Gds, &Dxf, &Svg];
    for format in formats {
        let layers = if format.per_layer() {
            masks.masks.iter().map(Some).collect()
        } else {
            vec![None]
        };
        for mask in layers {
            let name = format.file_name(mask.map(|mask| &mask.layer));
            let mut bytes = Vec::new();
            format
                .write(masks, mask, &mut bytes)
                .map_err(|source| ExportError::Io {
                    path: name.clone(),
                    source,
                })?;
            write(dir, &name, &bytes)?;
            files.push(name);
        }
    }

    // What a mask shop reads is the file, so the file is what is compared with the plan.
    let gds = crate::gds::readback(masks, &read(dir, &Gds.file_name(None))?);
    let mut findings = Vec::new();
    let mut metrics = BTreeMap::new();
    for mask in &masks.masks {
        let name = Dxf.file_name(Some(&mask.layer));
        let fault = |message: String| Finding {
            rule: "dxf-readback".to_owned(),
            subject: name.clone(),
            measured: None,
            limit: None,
            message,
            waived: None,
        };
        let planned = masks.flattened(mask);
        let planned_um2: f64 = planned.iter().map(|rectangle| rectangle.area()).sum::<f64>() * 1e12;
        match read_dxf(&String::from_utf8_lossy(&read(dir, &name)?)) {
            Ok(outlines) => {
                let area_um2: f64 = outlines.iter().map(|outline| area(outline)).sum();
                metrics.insert(format!("{}.outlines", mask.layer.name), outlines.len() as f64);
                metrics.insert(format!("{}.area_mm2", mask.layer.name), area_um2 * 1e-6);
                if outlines.len() != planned.len() || (area_um2 - planned_um2).abs() > 1e-6 * planned_um2 + 1e-3 {
                    findings.push(fault(format!(
                        "{name} holds {} outlines of {area_um2:.3} um^2; the plan has {} of {planned_um2:.3} um^2",
                        outlines.len(),
                        planned.len()
                    )));
                }
            }
            Err(message) => findings.push(fault(format!("{name}: {message}"))),
        }
    }
    let dxf = Check::from_findings("dxf-readback", findings, metrics, &[]);

    let mut rows = String::new();
    for mask in &masks.masks {
        let (layer, um) = (&mask.layer, mask.layer.height * 1e6);
        let tone = match mask.spec.tone {
            Tone::Clear => "clear",
            Tone::Dark => "dark",
        };
        rows.push_str(&format!(
            "| {} | {} | `{}` | {um:.2} | {} | {} | {tone} | {} | {:+.2} |\n",
            layer.name,
            layer.gds,
            Dxf.file_name(Some(layer)),
            masks.outlines(mask),
            mask.extended,
            if mask.spec.mirror { "yes" } else { "no" },
            mask.spec.bias * 1e6
        ));
    }
    let followed = match process {
        Some(process) => format!(
            "The masks follow the process `{}`: {}\n\nA feature that opens into a deeper one is drawn into it by the distance that process's overlap rules give, so that the two still meet when the layers sit that far apart.",
            process.name, process.description
        ),
        None => "The design names no process, so every feature is drawn exactly to its edge. Layers that sit apart by any distance may leave shallow features closed off from deeper ones.".to_owned(),
    };
    let readme = format!(
        "# Photomasks\n\n\
         `chip.gds` holds every layer, with repeated features as translates. The DXF drawings hold the same outlines, one layer each, flat. Layers are exposed in the order listed.\n\n\
         | Layer | GDSII number | DXF drawing | Height, um | Outlines | Drawn into a deeper layer | Inside the outlines | Mirrored | Bias, um |\n| - | - | - | - | - | - | - | - | - |\n{rows}\n\
         - **Units:** GDSII in nanometres (one user unit is a micrometre); DXF in micrometres. The lower left corner is the origin; the drawing is {:.1} x {:.1} mm.\n\
         - **Outlines:** each closed outline encloses resist that stays on the master and becomes a channel in the cast chip. A clear mask is transparent inside the outlines and opaque elsewhere, for a negative resist; a dark mask is the reverse.\n\
         - **Bias:** the outlines in these files already have every edge moved outward by the bias listed; a negative bias moves them inward.\n\
         - **Height:** the thickness of resist the layer is spun to. In the cast chip it is the depth of that layer's features.\n\
         - **Orientation:** a mask that is not mirrored shows the master from above; a mirrored one is turned over about the drawing's vertical centre line. The cast chip is the mirror image of the master.\n\
         - **Alignment:** each layer after the first is aligned to the marks the first layer leaves.\n\n\
         {followed}\n",
        masks.size[0] * 1e3,
        masks.size[1] * 1e3
    );
    write(dir, "README.md", readme.as_bytes())?;
    files.push("README.md".to_owned());
    Ok(Exported {
        files,
        checks: vec![gds, dxf],
    })
}
