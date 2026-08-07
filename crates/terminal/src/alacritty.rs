use alacritty_terminal::{
    Grid, Term,
    event::WindowSize,
    grid::{Dimensions, Row},
    index::{Column, Line},
    term::cell::Flags,
};

use crate::{AlacCell, TerminalBounds, TerminalListener};

pub fn window_size_from_terminal_bounds(bounds: TerminalBounds) -> WindowSize {
    WindowSize {
        num_lines: bounds.num_lines() as u16,
        num_cols: bounds.num_columns() as u16,
        cell_width: f32::from(bounds.cell_width()) as u16,
        cell_height: f32::from(bounds.line_height()) as u16,
    }
}

pub(super) fn last_non_empty_lines(
    term: &Term<TerminalListener>,
    line_count: usize,
) -> Vec<String> {
    let grid = term.grid();
    let mut lines = Vec::new();

    let mut current_line = grid.bottommost_line().0;
    let topmost_line = grid.topmost_line().0;

    while current_line >= topmost_line && lines.len() < line_count {
        let (logical_line_start, logical_line) =
            logical_line_for_row(grid, current_line, topmost_line);

        if let Some(line) = process_line(logical_line) {
            lines.push(line);
        }

        current_line = logical_line_start - 1;
    }

    lines.reverse();
    lines
}
fn process_line(line: String) -> Option<String> {
    let trimmed = line.trim_end().to_string();
    if !trimmed.is_empty() {
        Some(trimmed)
    } else {
        None
    }
}

fn logical_line_for_row(grid: &Grid<AlacCell>, current: i32, topmost: i32) -> (i32, String) {
    let start = find_logical_line_start(grid, current, topmost);
    let mut line = String::new();
    for row in start..=current {
        line.push_str(&row_to_string(&grid[Line(row)]));
    }
    (start, line)
}

fn find_logical_line_start(grid: &Grid<AlacCell>, current: i32, topmost: i32) -> i32 {
    let mut line_start = current;
    while line_start > topmost {
        let previous_line = Line(line_start - 1);
        let last_cell = &grid[previous_line][Column(grid.columns() - 1)];
        if !last_cell.flags.contains(Flags::WRAPLINE) {
            break;
        }
        line_start -= 1;
    }
    line_start
}

fn row_to_string(row: &Row<AlacCell>) -> String {
    row[..Column(row.len())]
        .iter()
        .map(|cell| cell.c)
        .collect::<String>()
}
