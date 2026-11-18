//! Checked cell geometry. Coordinates are zero-based.

/// A cell position, independent of pixels or Unicode byte offsets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub row: u16,
    pub column: u16,
}

/// A cell extent. Empty sizes are valid for hidden or clipped panes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Size {
    pub rows: u16,
    pub columns: u16,
}

impl Size {
    pub const fn is_empty(self) -> bool {
        self.rows == 0 || self.columns == 0
    }

    pub const fn contains(self, point: Point) -> bool {
        point.row < self.rows && point.column < self.columns
    }

    /// Returns None for an empty extent; otherwise clamps to its last cell.
    pub const fn clamp(self, point: Point) -> Option<Point> {
        if self.is_empty() {
            return None;
        }
        Some(Point {
            row: if point.row < self.rows {
                point.row
            } else {
                self.rows - 1
            },
            column: if point.column < self.columns {
                point.column
            } else {
                self.columns - 1
            },
        })
    }
}

/// A rectangle whose exclusive bottom/right edges fit in u16.
///
/// Construction fails on overflow rather than wrapping a pane into another one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    origin: Point,
    size: Size,
}

impl Rect {
    pub const fn new(origin: Point, size: Size) -> Option<Self> {
        if origin.row.checked_add(size.rows).is_none()
            || origin.column.checked_add(size.columns).is_none()
        {
            return None;
        }
        Some(Self { origin, size })
    }

    pub const fn origin(self) -> Point {
        self.origin
    }
    pub const fn size(self) -> Size {
        self.size
    }
    pub const fn bottom(self) -> u16 {
        self.origin.row + self.size.rows
    }
    pub const fn right(self) -> u16 {
        self.origin.column + self.size.columns
    }

    pub const fn contains(self, point: Point) -> bool {
        point.row >= self.origin.row
            && point.row < self.bottom()
            && point.column >= self.origin.column
            && point.column < self.right()
    }

    /// Intersect with a terminal extent. Empty results have an in-bounds edge.
    pub const fn clipped(self, viewport: Size) -> Self {
        let row = if self.origin.row < viewport.rows {
            self.origin.row
        } else {
            viewport.rows
        };
        let column = if self.origin.column < viewport.columns {
            self.origin.column
        } else {
            viewport.columns
        };
        let bottom = if self.bottom() < viewport.rows {
            self.bottom()
        } else {
            viewport.rows
        };
        let right = if self.right() < viewport.columns {
            self.right()
        } else {
            viewport.columns
        };
        Self {
            origin: Point { row, column },
            size: Size {
                rows: bottom.saturating_sub(row),
                columns: right.saturating_sub(column),
            },
        }
    }

    /// Split into left/right panes with a bounded gap; no cells overlap.
    pub const fn split_columns(self, wanted_left: u16, wanted_gap: u16) -> (Self, Self) {
        let left = if wanted_left < self.size.columns {
            wanted_left
        } else {
            self.size.columns
        };
        let remaining = self.size.columns - left;
        let gap = if wanted_gap < remaining {
            wanted_gap
        } else {
            remaining
        };
        (
            Self {
                origin: self.origin,
                size: Size {
                    rows: self.size.rows,
                    columns: left,
                },
            },
            Self {
                origin: Point {
                    row: self.origin.row,
                    column: self.origin.column + left + gap,
                },
                size: Size {
                    rows: self.size.rows,
                    columns: remaining - gap,
                },
            },
        )
    }

    /// Split into top/bottom panes with a bounded gap.
    pub const fn split_rows(self, wanted_top: u16, wanted_gap: u16) -> (Self, Self) {
        let top = if wanted_top < self.size.rows {
            wanted_top
        } else {
            self.size.rows
        };
        let remaining = self.size.rows - top;
        let gap = if wanted_gap < remaining {
            wanted_gap
        } else {
            remaining
        };
        (
            Self {
                origin: self.origin,
                size: Size {
                    rows: top,
                    columns: self.size.columns,
                },
            },
            Self {
                origin: Point {
                    row: self.origin.row + top + gap,
                    column: self.origin.column,
                },
                size: Size {
                    rows: remaining - gap,
                    columns: self.size.columns,
                },
            },
        )
    }
}
