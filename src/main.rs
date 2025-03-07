use enigo::{Coordinate, Enigo, Mouse, Settings};

use std::{
    thread::{self},
    time::Duration,
};

fn main() {
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    let wait = Duration::from_secs(300);
    loop {
        thread::sleep(wait);
        enigo.move_mouse(10, 10, Coordinate::Rel).unwrap();
        thread::sleep(Duration::from_millis(50));
        enigo.move_mouse(-10, -10, Coordinate::Rel).unwrap();
    }
}
