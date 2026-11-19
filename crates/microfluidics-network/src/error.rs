//! What can go wrong extracting or solving a graph.

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum NetworkError {
    #[error("the chip states no edges: there is nothing for fluid to pass through")]
    Empty,
    #[error("the edge of {0} has no length")]
    NoLength(String),
    #[error("{0} is driven, but no edge reaches it")]
    UnknownDrive(String),
    #[error("{0} is joined to nowhere held at a pressure, so its pressure cannot be found")]
    Floating(String),
    #[error("there is no {0} in the graph")]
    Unknown(String),
    #[error("{0} has several edges; name the port of the one meant")]
    Ambiguous(String),
}
