use std::fmt::Debug;

use macroquad::rand::RandomRange;

#[derive(Debug,Default)]
pub struct RandomNumberGenerator {
}

impl RandomNumberGenerator {
	/// Half-open
	pub fn uniform<T>(&mut self, lower: T, upper: T) -> T
	where
		T: RandomRange,
	{
		macroquad::rand::gen_range(lower, upper)

	}
}
