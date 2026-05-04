use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlphaShapeError {
    NotEnoughPoints(usize),
    InvalidAlpha,
    InvalidAreaRatio,
    InvalidPoint,
    TriangulationFailed(String),
    EmptyShape,
}

pub type IsoHullError = AlphaShapeError;

impl fmt::Display for AlphaShapeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotEnoughPoints(count) => {
                write!(
                    formatter,
                    "not enough unique points: expected 3, got {count}"
                )
            }
            Self::InvalidAlpha => write!(formatter, "alpha radius must be finite and positive"),
            Self::InvalidAreaRatio => {
                write!(formatter, "area ratio must be finite and between 0 and 1")
            }
            Self::InvalidPoint => write!(formatter, "point coordinates must be finite"),
            Self::TriangulationFailed(error) => write!(formatter, "triangulation failed: {error}"),
            Self::EmptyShape => write!(formatter, "alpha shape is empty"),
        }
    }
}

impl Error for AlphaShapeError {}
