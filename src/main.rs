mod tetris;

use macroquad::prelude::*;
use macroquad::color;
use miniquad::window::set_window_size;

const GAME_OVER: &str = "GAME OVER";
fn game_over() {
	// FIXME prompt for another game with a new GameState
	let width = screen_width();
	let font_size = 48;
	let dims = measure_text(GAME_OVER, None, font_size, 1.0);
	draw_text(GAME_OVER, (width - dims.width) / 2.0, dims.offset_y, font_size as f32, RED);
}

#[macroquad::main("Tetris clone in Rust")]
async fn main() {
	// HARDCODE Do a proper config system later
	let width_cells = 8;
	let height_cells = 24;
	let cell_sidelength_px = 32;
	// derived config
	let width_px = width_cells * cell_sidelength_px;
	let height_px = height_cells * cell_sidelength_px;
	let score_font_size = (cell_sidelength_px as u16) * 2;
	let cell_sidelength_px_f32 = cell_sidelength_px as f32;
	// </config>
	let mut game_state = tetris::GameState::new(height_cells, width_cells);
	loop {
		if !game_state.is_alive {
			game_over();
			next_frame().await;
			continue;
		}
		// Input
		// FIXME framerate?
		if is_key_down(KeyCode::Space) {
			game_state.toggle_drop_rate();
		}
		// Only one direction at once, please.
		if is_key_down(KeyCode::Up) {
			game_state.try_rotate_current_piece(false);
		} else if is_key_down(KeyCode::Down) {
			game_state.try_rotate_current_piece(true);
		} else if is_key_down(KeyCode::Left) {
			game_state.try_leftright_current_piece(true);
		} else if is_key_down(KeyCode::Right) {
			game_state.try_leftright_current_piece(false);
		}

		// Logic
		game_state.tick();
		// println!("{:?}", game_state);

		// Draw
		set_window_size(width_px as u32, height_px as u32);
		clear_background(BLACK);

		let score = game_state.rows_cleared.to_string();
		let score_dims = measure_text(&score, None, score_font_size, 1.0);
		draw_text(&score, (width_px as f32 - score_dims.width) / 2.0, (height_px as f32 - score_dims.height) / 2.0, score_font_size as f32, DARKGRAY);

		let (mut x, mut y) = (0.0, 0.0);
		for row in game_state.cell_matrix.iter() {
			for cell in row.cells.iter() {
				if let Some(c) = cell {
					let color = color::hsl_to_rgb(c.hue, 0.5, 0.3); // HARDCODE Maybe less saturated?
					draw_rectangle(x, y, cell_sidelength_px_f32, cell_sidelength_px_f32, color);
				}
				x += cell_sidelength_px_f32;
			}
			x = 0.0;
			y += cell_sidelength_px_f32;
		}

		if let Some(p) = game_state.current_piece.as_ref() {
			for (c, x, y) in p.iter_global_space(game_state.current_piece_mass_xy) {
				let color = color::hsl_to_rgb(c.hue, 1.0, 0.5); // HARDCODE Saturation?
				let (x_px, y_px) = (x as f32 * cell_sidelength_px_f32, y as f32 * cell_sidelength_px_f32);
				draw_rectangle(x_px, y_px, cell_sidelength_px_f32, cell_sidelength_px_f32, color);
			}
		}

		next_frame().await
	}
}
