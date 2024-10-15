pub struct GameState {
	/// Indexing: cell_matrix[y].cells[x] = Some(foo_cell);
	pub cell_matrix: Vec<Row>,
	pub cell_matrix_width: usize,
	/// None during row clears
	pub current_piece: Option<Piece>,
	/// Global coordinates of the center of mass of this piece; may or may not have a Cell.
	pub current_piece_mass_xy: (usize, usize),
	/// Time to fall by one cell-space, expressed in game ticks.
	current_piece_ticks_per_drop_want: u32,
	/// Time already spent falling by one cell-space, expressed in game ticks.
	current_piece_ticks_per_drop_have: u32,
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
			current_piece_ticks_per_drop_want: 10, // HARDCODE see other writes to this field
			current_piece_ticks_per_drop_have: 0,
			rows_cleared: 0,
			is_alive: true,
		};
		gs.queue_new_piece();
		gs
	}
	pub fn try_rotate_current_piece(&mut self, clockwise: bool) -> bool {
		if let Some(p_old) = self.current_piece.as_ref() {
			let p_new = p_old.rotated(clockwise);
			let dst = (self.current_piece_mass_xy.0 as isize, self.current_piece_mass_xy.1 as isize);
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
			let dst = (self.current_piece_mass_xy.0 as isize + direction, self.current_piece_mass_xy.1 as isize);
			if self.can_place(&p, dst) {
				self.current_piece_mass_xy = (dst.0 as usize, dst.1 as usize);
				return true;
			}
		}
		false
	}

	pub fn tick(&mut self) -> () {
		self.current_piece_ticks_per_drop_have += 1;
		if self.current_piece_ticks_per_drop_have < self.current_piece_ticks_per_drop_want {
			return;
		}
		self.current_piece_ticks_per_drop_have = 0;
		if let Some(p) = self.current_piece.as_ref() {
			let dst = (self.current_piece_mass_xy.0 as isize, self.current_piece_mass_xy.1 as isize + 1);
			if self.can_place(&p, dst) {
				self.current_piece_mass_xy = (dst.0 as usize, dst.1 as usize);
			} else {
				self.commit_current_piece();
				self.clear_finished_rows();
			}
		} else {
			self.queue_new_piece();
		}
	}

	fn commit_current_piece(&mut self) -> () {
		if let Some(p) = self.current_piece.take() {
			for c in p.cells.into_iter() {
				let x = (self.current_piece_mass_xy.0 as i32 + c.x) as usize;
				let y = (self.current_piece_mass_xy.1 as i32 + c.y) as usize;
				self.cell_matrix[y].cells[x] = Some(c.cell);
				self.cell_matrix[y].is_empty = false;
			}
		}
	}

	fn clear_finished_rows(&mut self) -> () {
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

	fn queue_new_piece(&mut self) -> () {
		let p = Piece::generate_new();
		let init_xy = (self.cell_matrix_width / 2, 0); // HARDCODE Should this be random?
		if !self.can_place(&p, (init_xy.0 as isize, init_xy.1 as isize)) {
			self.is_alive = false;
			return;
		}
		self.current_piece = Some(p);
		self.current_piece_mass_xy = init_xy;
	}

	fn can_place(&self, p: &Piece, (global_x, global_y): (isize, isize)) -> bool {
		let (height, width) = (self.cell_matrix.len(), self.cell_matrix_width);
		for c in p.cells.iter() {
			let global_cell_x = global_x + (c.x as isize);
			if global_cell_x < 0 || width <= (global_cell_x as usize) {
				return false;
			}
			let global_cell_y = global_y + (c.y as isize);
			if global_cell_y < 0 || height <= (global_cell_y as usize) {
				return false;
			}
			if self.cell_matrix[global_cell_y as usize].cells[global_cell_x as usize].is_some() {
				return false;
			}
		}
		true
	}

	pub fn toggle_drop_rate(&mut self) -> () {
		// HARDCODE Faster/slower falling speeds
		if self.current_piece_ticks_per_drop_want == 1 {
			self.current_piece_ticks_per_drop_want = 10;
		} else {
			self.current_piece_ticks_per_drop_want = 1;
		}
	}
}

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

#[derive(Clone)]
pub struct Piece {
	/// May replace with Vec<_> for penta/hex-tetris.
	pub cells: Vec<CellWithRelativePosition>,
	// Origin for cell positions
	pub center_of_mass_x: i32,
	pub center_of_mass_y: i32,
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
			center_of_mass_y: 1,
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
}

fn rotate_2d(clockwise: bool, (x, y): (i32, i32)) -> (i32, i32) {
	if clockwise {
		(y, -x)
	} else {
		(-y, x)
	}
}

#[derive(Clone)]
pub struct CellWithRelativePosition {
	pub cell: Cell,
	pub x: i32,
	pub y: i32,
}


#[derive(Clone)]
pub struct Cell {
	pub hue: f32,
}

impl Cell {
	pub fn new(hue: f32) -> Cell {
		Self { hue }
	}
}
