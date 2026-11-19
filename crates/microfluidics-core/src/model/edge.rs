//! How fluid passes through a chip: ducts in series between named vertices.

use serde::Serialize;

/// One straight rectangular duct: its height is that of its layer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DuctSpec {
    pub layer: String,
    /// In metres.
    pub width: f64,
    /// In metres.
    pub length: f64,
}

/// A way fluid passes between two vertices: an edge within a region, or a routed channel.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EdgeSpec {
    /// What the edge belongs to: an instance, or a channel.
    pub element: String,
    /// The vertices it joins: ports, or vertices within a region.
    pub from: String,
    pub to: String,
    pub ducts: Vec<DuctSpec>,
}

/// Which way fluid passes a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PortKind {
    Inlet,
    Outlet,
    Both,
}
