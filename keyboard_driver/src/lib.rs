#![no_std]

use embassy_stm32::gpio::{AnyPin, Input, Output};
use embassy_time::{Duration, Timer};


pub struct Keyboard<'d, const C: usize, const R: usize> {
    cols: [Output<'d, AnyPin>; C],
    rows: [Input<'d, AnyPin>; R],
    mapping: [[&'d str; C]; R],
}

impl<'d, const C: usize, const R: usize> Keyboard<'d, C, R> {
    pub fn new(
        cols: [Output<'d, AnyPin>; C],
        rows: [Input<'d, AnyPin>; R],
        mapping: [[&'d str; C]; R],
    ) -> Self {
        let cols = cols;
        let rows = rows;

        let mut keyboard = Self { cols, rows, mapping };

        keyboard.set_outputs_low();
        keyboard.set_outputs_high();

        keyboard
    }

    fn set_outputs_high(&mut self) {
        for c in &mut self.cols {
            c.set_high();
        }
    }

    fn set_outputs_low(&mut self) {
        for c in &mut self.cols {
            c.set_low();
        }
    }


    async fn read_rows(&self, arr: &mut [bool]) {
        for (rn, row) in self.rows.iter().enumerate() {
            if row.is_high() {
                arr[rn] = true;
            }
            Timer::after(Duration::from_micros(200)).await;
        }
    }

    async fn read_keys(&mut self) -> [[bool; C]; R] {
        let mut key = [[false; C]; R];

        let mut rows = [false; R];

        self.read_rows(&mut rows).await;

        for (rn, r) in rows.iter().enumerate() {
            if *r {
                self.set_outputs_low();
                for (cn, c) in self.cols.iter_mut().enumerate() {
                    c.set_high();
                    if self.rows[rn].is_high() {
                        key[rn][cn] = true;
                    }
                    c.set_low();
                }
            }
        }
        self.set_outputs_high();
        key
    }

    pub async fn get_on_released(&mut self) -> (usize, usize) {
        let mut key = self.read_keys().await;

        let mut addr_button = (R, C);

        for r in 0..R {
            for c in 0..C {
                if key[r][c] {
                    while key[r][c] {
                        key = self.read_keys().await;
                    }
                    addr_button = (R - r - 1, c);
                }
            }
        }

        addr_button
    }

    pub async fn get_str(&mut self) -> &str {
        loop {
            let (r, c) = self.get_on_released().await;
            if (r, c) != (R, C) {
                return self.mapping[r][c];
            }
        }
    }

    pub async fn get_digit(&mut self) -> Option<u8> {
        loop {
            let (r, c) = self.get_on_released().await;
            if (r, c) != (R, C) {
                if let Some(digit) = char::to_digit(
                    self.mapping[r][c].chars().nth(0).unwrap(), 10) {
                    return Some(digit as u8);
                }
                return None;
            }
        }
    }
}