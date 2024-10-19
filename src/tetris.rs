#[derive(Debug)]
pub struct GameState {
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

	pub fn try_drop_current_piece(&mut self) {
		if let Some(p) = self.current_piece.as_ref() {
			let dst = (self.current_piece_mass_xy.0, self.current_piece_mass_xy.1 + 1);
			if self.can_place(p, dst) {
				self.current_piece_mass_xy = dst;
			} else {
				self.commit_current_piece();
				self.clear_finished_rows();
			}
		} else {
			self.queue_new_piece();
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
		let p = Piece::generate_new();
		let init_xy = (self.cell_matrix_width as i32 / 2, 0); // HARDCODE Should this be random?
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
	i_cells: usize,
	global_xy: (i32, i32),
}

impl <'a> Iterator for PieceGlobalSpaceIter<'a> {
	type Item = (&'a Cell, i32, i32);
	fn next(&mut self) -> Option<Self::Item> {
		let retval = self.piece.cells
			.get(self.i_cells)
			.map(|c| {
				let x = self.global_xy.0 + c.x - self.piece.center_of_mass_x;
				let y = self.global_xy.1 + c.y - self.piece.center_of_mass_y;
				(&c.cell, x, y)
			});
		self.i_cells += 1;
		retval
	}
}

impl Piece {
	fn generate_new() -> Piece {
		let hue: f32 = rand::random();
		Self {
			// FIXME placeholder
			cells: vec![
				CellWithRelativePosition { cell: Cell::new(hue), x: 0, y: 0, },
				CellWithRelativePosition { cell: Cell::new(hue), x: 1, y: 0, },
				CellWithRelativePosition { cell: Cell::new(hue), x: 2, y: 0, },
				CellWithRelativePosition { cell: Cell::new(hue), x: 3, y: 0, }
			],
			center_of_mass_x: 1,
			center_of_mass_y: 0,
		}
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
			piece: &self,
			i_cells: 0,
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
