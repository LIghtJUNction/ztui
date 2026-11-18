//! Small, dependency-free text layout and ANSI frame-diff primitives.
//!
//! The caller owns every string, output buffer, and terminal. This crate neither
//! allocates nor measures Unicode: supply measured display columns for rows.
//! Images, input events, operating-system I/O, and application state stay outside.
#![no_std]
#![forbid(unsafe_code)]

pub mod frame;
pub mod geometry;
pub mod row;

pub use frame::{Cursor, DiffStats, Frame, Modes, write_diff};
pub use geometry::{Point, Rect, Size};
pub use row::{Row, RowError, write_spaces};
