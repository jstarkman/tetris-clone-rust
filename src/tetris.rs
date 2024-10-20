use std::collections::HashSet;

use crate::rng;

#[derive(Debug)]
pub struct GameState {
	rng: Box<rng::RandomNumberGenerator>,
	/// Indexing: cell_matrix[y].cells[x] = Some(foo_cell);
	pub cell_matrix: Vec<Row>,
	pub cell_matrix_width: usize,
	/// None during row clears
	pub current_piece: Option<Piece>,
	/// Global coordinates of the center of mass of this piece; may or may not have a Cell.
	pub current_piece_mass_xy: (i32, i32),
	/// Counter; never decremented.
	pub rows_cleared: u32,
	pub is_alive: bool,
}

impl GameState {
	pub fn new(height: usize, width: usize) -> GameState {
		let mut gs = Self {
			rng: Box::default(),
			cell_matrix: (0 .. height).map(|_| Row::new(width)).collect(),
			cell_matrix_width: width,
			current_piece: None, // generated below
			current_piece_mass_xy: (0, 0), // ibid
			rows_cleared: 0,
			is_alive: true,
		};
		gs.queue_new_piece();
		gs
	}

	pub fn reset(&mut self) {
		self.cell_matrix.iter_mut().for_each(|row| row.reset());
		self.current_piece = None;
		self.current_piece_mass_xy = (0, 0);
		self.rows_cleared = 0;
		self.is_alive = true;
	}

	pub fn try_rotate_current_piece(&mut self, clockwise: bool) -> bool {
		if let Some(p_old) = self.current_piece.as_ref() {
			let p_new = p_old.rotated(clockwise);
			let dst = self.current_piece_mass_xy;
			if self.can_place(&p_new, dst) {
				self.current_piece = Some(p_new);
				return true;
			}
		}
		false
	}

	pub fn try_leftright_current_piece(&mut self, leftwards: bool) -> bool {
		if let Some(p) = self.current_piece.as_ref() {
			let direction = if leftwards { -1 } else { 1 };
			let dst = (self.current_piece_mass_xy.0 + direction, self.current_piece_mass_xy.1);
			if self.can_place(p, dst) {
				self.current_piece_mass_xy = dst;
				return true;
			}
		}
		false
	}

	pub fn try_drop_current_piece(&mut self) -> bool {
		if let Some(p) = self.current_piece.as_ref() {
			let dst = (self.current_piece_mass_xy.0, self.current_piece_mass_xy.1 + 1);
			if self.can_place(p, dst) {
				self.current_piece_mass_xy = dst;
				true
			} else {
				self.commit_current_piece();
				self.clear_finished_rows();
				false
			}
		} else {
			self.queue_new_piece();
			false
		}
	}

	fn commit_current_piece(&mut self) {
		if let Some(p) = self.current_piece.take() {
			for (c, x, y) in p.iter_global_space(self.current_piece_mass_xy) {
				// SAFETY: called .can_place() before this method
				let (x, y) = (x as usize, y as usize);
				self.cell_matrix[y].cells[x] = Some(c.clone());
				self.cell_matrix[y].is_empty = false;
			}
		}
	}

	fn clear_finished_rows(&mut self) {
		// clear and drop rows; is bubble-sort in slow motion
		let mut anything_changed = false;
		for i_row in 0 .. self.cell_matrix.len() {
			{
				let row = &mut self.cell_matrix[i_row];
				if row.is_empty {
					continue;
				}
				if row.cells.iter().all(Option::is_some) {
					row.cells.iter_mut().for_each(|c| { c.take(); });
					self.rows_cleared += 1;
					row.is_empty = true;
					anything_changed = true;
				}
			}
			// drop higher cells
			if self.cell_matrix[i_row].is_empty {
				for i_row in (1 ..= i_row).rev() {
					let i_above = i_row - 1;
					if !self.cell_matrix[i_above].is_empty {
						self.cell_matrix.swap(i_row, i_row-1);
					} else {
						break;
					}
				}
			}
		}
		// FIXME handle multi-clears; maybe take everything above the cleared line and bundle it into a super-Piece?
		// loop until no more clears
		if anything_changed {
			self.clear_finished_rows();
		}
	}

	fn queue_new_piece(&mut self) {
		let p: Piece = Piece::generate_new(&mut self.rng);
		let clearance = p.iter_global_space((0, 0)).map(|(_c, _x, y)| y).min()
			.expect("Should have cells")
			.abs();
		let init_xy = (self.cell_matrix_width as i32 / 2, clearance); // HARDCODE Should this be random?
		if !self.can_place(&p, init_xy) {
			self.is_alive = false;
			return;
		}
		self.current_piece = Some(p);
		self.current_piece_mass_xy = init_xy;
	}

	fn can_place(&self, p: &Piece, (global_x, global_y): (i32, i32)) -> bool {
		p.iter_global_space((global_x, global_y))
			.all(|(_c, x, y)| {
				if x < 0 || y < 0 {
					return false;
				}
				let Some(row) = self.cell_matrix.get(y as usize)
					else { return false; };
				let Some(cell) = row.cells.get(x as usize)
					else { return false; };
				cell.is_none()
			})
	}
}

#[derive(Debug)]
pub struct Row {
	pub cells: Vec<Option<Cell>>,
	is_empty: bool,
}

impl Row {
	fn new(width: usize) -> Row {
		Self {
			cells: vec![None; width],
			is_empty: true,
		}
	}

	fn reset(&mut self) {
		self.cells.iter_mut().for_each(|c| *c = None);
		self.is_empty = true;
	}
}

#[derive(Clone,Debug)]
pub struct Piece {
	/// May replace with Vec<_> for penta/hex-tetris.
	pub cells: Vec<CellWithRelativePosition>,
	// Origin for cell positions
	pub center_of_mass_x: i32,
	pub center_of_mass_y: i32,
}

pub struct PieceGlobalSpaceIter<'a> {
	piece: &'a Piece,
	iter_cells: std::slice::Iter<'a, CellWithRelativePosition>,
	global_xy: (i32, i32),
}

impl <'a> Iterator for PieceGlobalSpaceIter<'a> {
	type Item = (&'a Cell, i32, i32);
	fn next(&mut self) -> Option<Self::Item> {
		self.iter_cells
			.next()
			.map(|c| {
				let x = self.global_xy.0 + c.x - self.piece.center_of_mass_x;
				let y = self.global_xy.1 + c.y - self.piece.center_of_mass_y;
				(&c.cell, x, y)
			})
	}
}

impl Piece {
	const OFFSETS: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];
	fn generate_new(rng: &mut rng::RandomNumberGenerator) -> Piece {
		// Idea: randomly attach each new cell to an empty site on the existing piece's perimeter.
		let hue: f32 = rng.uniform(0.0, 1.0);
		// Why limit ourselves to just *tetr*-is?
		let size = rng.uniform(3, 6);
		// This is biased towards T- and L-shaped pieces; is that a good thing?
		let mut cells = vec![CellWithRelativePosition { cell: Cell::new(hue), x: 0, y: 0, }];
		let mut sites = HashSet::from(Self::OFFSETS);
		for _ in 1 .. size {
			let idx = rng.uniform(0, sites.len());
			let &(x, y) = sites.iter().nth(idx)
				.expect("Should have generated a valid index");
			cells.push(CellWithRelativePosition { cell: Cell::new(hue), x, y, });
			sites.remove(&(x, y));
			for (xx, yy) in Self::OFFSETS.iter().map(|(dx, dy)| (x+dx, y+dy)) {
				let is_blocked = cells.iter().any(|c| c.x == xx && c.y == yy);
				if !is_blocked {
					let _already_had = sites.insert((xx, yy));
				}
			}
		}
		let (center_of_mass_x, center_of_mass_y) = {
			let (x, y) = cells.iter()
				.fold((0, 0), |(acc_x, acc_y), c| (acc_x + c.x, acc_y + c.y));
			let m = cells.len() as f32;
			((x as f32 / m).round() as i32, (y as f32 / m).round() as i32)
		};
		Self { cells, center_of_mass_x, center_of_mass_y }
	}

	fn rotated(&self, clockwise: bool) -> Piece {
		let cells = self.cells.iter()
			.map(|p| {
				let v = (p.x - self.center_of_mass_x, p.y - self.center_of_mass_y);
				let v = rotate_2d(clockwise, v);
				let v = (v.0 + self.center_of_mass_x, v.1 + self.center_of_mass_y);
				CellWithRelativePosition {
					cell: p.cell.clone(),
					x: v.0,
					y: v.1,
				}
			})
			.collect();
		Self { cells, ..*self }
	}

	pub fn iter_global_space(&self, xy: (i32, i32)) -> PieceGlobalSpaceIter {
		PieceGlobalSpaceIter {
			piece: self,
			iter_cells: self.cells.iter(),
			global_xy: xy,
		}
	}
}

fn rotate_2d(clockwise: bool, (x, y): (i32, i32)) -> (i32, i32) {
	if clockwise {
		(y, -x)
	} else {
		(-y, x)
	}
}

#[derive(Clone,Debug)]
pub struct CellWithRelativePosition {
	pub cell: Cell,
	pub x: i32,
	pub y: i32,
}


#[derive(Clone,Debug)]
pub struct Cell {
	pub hue: f32,
}

impl Cell {
	pub fn new(hue: f32) -> Cell {
		Self { hue }
	}
}
