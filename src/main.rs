#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::usize;

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};

use {defmt_rtt as _, panic_probe as _};

use display_driver::*;
use keyboard_driver::*;


fn fill_with_digits(mut number: u32, repr: &mut [u8; 8]) {
    for i in (0..8).rev() {
        repr[i] = (number % 10) as u8;
        number /= 10
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let p = embassy_stm32::init(Default::default());
    let mut display = LedAndKey::new(p.PA5, p.PA4, p.PA3);

    let out1 = Output::new(p.PA11, Level::Low, Speed::Low).degrade();
    let out2 = Output::new(p.PA12, Level::Low, Speed::Low).degrade();
    let out3 = Output::new(p.PA15, Level::Low, Speed::Low).degrade();
    let out4 = Output::new(p.PB3, Level::Low, Speed::Low).degrade();

    let inp1 = Input::new(p.PB5, Pull::Down).degrade();
    let inp2 = Input::new(p.PB6, Pull::Down).degrade();
    let inp3 = Input::new(p.PB7, Pull::Down).degrade();
    let inp4 = Input::new(p.PB8, Pull::Down).degrade();
    let inp5 = Input::new(p.PB9, Pull::Down).degrade();

    let rows = [inp1, inp2, inp3, inp4, inp5];
    let cols = [out1, out2, out3, out4];

    let mapping = [
        ["F1", "F2", "#", "*"],
        ["1", "2", "3", "ArrowUp"],
        ["4", "5", "6", "ArrowDown"],
        ["7", "8", "9", "Esc"],
        ["<", "0", ">", "Ent"]
    ];

    let mut key: Keyboard<4, 5> = Keyboard::new(cols, rows, mapping);
    let mut n = 0;

    loop {
        let b = key.get_str().await;

        if n <= 9999999 {
            match b {
                "1" => n = n * 10 + 1,
                "2" => n = n * 10 + 2,
                "3" => n = n * 10 + 3,
                "4" => n = n * 10 + 4,
                "5" => n = n * 10 + 5,
                "6" => n = n * 10 + 6,
                "7" => n = n * 10 + 7,
                "8" => n = n * 10 + 8,
                "9" => n = n * 10 + 9,
                "0" => n = n * 10 + 0,
                _ => {},
            }
        }

        match b {
            "F1" =>  n = 0,
            "F2" => {
                if n != 0 {
                    loop {
                        let mut seg = [0.to_segment(false, false); 8];
                        let mut arr = [0; 8];
                        let number = n;
                        let mut cursor = 0;

                        fill_with_digits(number, &mut arr);

                        let mut k = 0;
                        for i in 0..8 {
                            if k == 0 && arr[i] != 0 {
                                seg[i] = arr[i].to_segment(false, true);
                                cursor = i;
                                k += 1;
                            } else {
                                seg[i] = arr[i].to_segment(i != 0 && arr[i - 1] % 2 == 1, false);
                            }
                        }

                        let button = key.get_str().await;
                        match button {
                            "F1" => break,
                            "F2" => {
                                loop {
                                    display.write_auto(&seg);

                                    let b = key.get_str().await;
                                    match b {
                                        "F1" => break,
                                        "F2" => {
                                            let seg_len = seg.len();
                                            if cursor < seg_len {
                                                let digit = arr[cursor];
                                                let segment = &mut seg[cursor];
                                                *segment = if !segment.led_on { digit / 2 } else { (digit + 10) / 2 }
                                                    .to_segment(cursor == seg_len - 1 && digit % 2 == 1, false);
                                                if let Some(last) = seg.get_mut(cursor + 1) {
                                                    last.with_dot = cursor < seg_len;
                                                }
                                                cursor += 1;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            },
            _ => {},
        }
        display.print_number(n);
    }
}