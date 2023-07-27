#![no_std]
#![allow(dead_code)]

mod raw;

use embassy_stm32::{self, gpio::{Level, Output, Speed}, into_ref, Peripheral};
use embassy_stm32::gpio::{Flex, Pin, Pull};
use crate::raw::SpecialSegments;

pub struct State {
    pub segments_states: [Segment; 8],
}

impl Default for State {
    fn default() -> Self {
        Self { segments_states: [Segment::default(); 8] }
    }
}

const DIGITS_REPR: &[u8] = &[0x3F, 0x06, 0x5B, 0x4F, 0x66, 0x6D, 0x7D, 0x07, 0x7F, 0x6F];

pub struct LedAndKey<'d, STB: Pin, CLK: Pin, DIO: Pin> {
    stb: Output<'d, STB>,
    clk: Output<'d, CLK>,
    dio: Flex<'d, DIO>,

    pub state: State,
}

impl<'d, STB: Pin, CLK: Pin, DIO: Pin> LedAndKey<'d, STB, CLK, DIO> {
    pub fn new(
        stb: impl Peripheral<P=STB> + 'static,
        clk: impl Peripheral<P=CLK> + 'static,
        dio: impl Peripheral<P=DIO> + 'static,
    ) -> LedAndKey<'d, STB, CLK, DIO> {
        into_ref!(stb, clk, dio);

        let mut clk = Output::new(clk, Level::Low, Speed::Low);
        let mut dio = Flex::new(dio);
        let mut stb = Output::new(stb, Level::Low, Speed::Low);

        stb.set_low();
        clk.set_low();
        dio.set_as_input_output(Speed::Low, Pull::Up);

        let mut display = Self { stb, dio, clk, state: Default::default() };

        display.activate();
        display.reset();
        display
    }

    fn activate(&mut self) {
        self.shift_out(0x8F);
    }

    fn reset(&mut self) {
        self.send_command(0x40);
        self.write_auto(&[0u8.to_segment(false, false); 8]);
        // self.write_auto(&[Segment::default(); 8]);
    }

    pub fn print_number(&mut self, number: u32) {
        let mut current_segments = self.state.segments_states;
        let mut n = number;
        for i in (0..8).rev() {
            current_segments[i].char = DIGITS_REPR[(n % 10) as usize];
            n /= 10;
        }
        self.write_auto(&current_segments)
    }

    pub fn write_auto(&mut self, cmd: &[Segment]) {
        self.send_command(0x40);

        self.stb.set_low();
        {
            self.shift_out(0xC0);
            for c in cmd {
                self.shift_out(c.char | if c.with_dot { SpecialSegments::DOT as u8 } else { 0 });
                self.shift_out(if c.led_on { 1 } else { 0 });
            }
        }
        self.stb.set_high();
    }

    pub fn write_at(&mut self, pos: usize, c: Segment) {
        self.send_command(0x44);

        self.stb.set_low();
        {
            self.shift_out((0xC0 | pos * 2) as u8);
            self.shift_out(c.char | if c.with_dot { SpecialSegments::DOT as u8 } else { 0 });
        }
        self.stb.set_high();

        self.send_command(0x44);

        self.stb.set_low();
        {
            self.shift_out((0xC0 | (pos * 2 + 1)) as u8);
            self.shift_out(if c.led_on { 1 } else { 0 });
        }
        self.stb.set_high();
    }

    pub fn light_up_led(&mut self, index: usize) {
        let mut state = self.state.segments_states;
        state[index].led_on = true;
        self.write_auto(&state);
    }

    pub fn read_keys(&mut self) -> u8 {
        self.stb.set_low();

        self.shift_out(0x42);

        self.dio.set_high();

        let mut keys = 0;
        for p in 0..4 {
            keys |= self.shift_in() << p;
        }

        self.stb.set_high();

        keys
    }

    pub fn send_command(&mut self, cmd: u8) {
        self.stb.set_low();

        self.shift_out(cmd);

        self.stb.set_high();
    }


    fn shift_out(&mut self, cmd: u8) {
        for i in 0..8 {
            if (cmd & (1 << i)) == 0 { self.dio.set_low() } else { self.dio.set_high() }
            self.clk.set_high();
            self.clk.set_low();
        }
    }

    fn shift_in(&mut self) -> u8 {
        let mut cmd: u8 = 0;
        for i in 0..8 {
            self.clk.set_high();
            if self.dio.is_high() {
                cmd |= 1 << i;
            }
            self.clk.set_low();
        }
        cmd
    }
}

#[derive(Copy, Clone)]
pub struct Segment {
    char: u8,
    digit: Option<u8>,
    pub led_on: bool,
    pub with_dot: bool,
}

impl Default for Segment {
    fn default() -> Self {
        Self { char: 0, digit: None, led_on: false, with_dot: false }
    }
}

pub trait Displayable {
    fn to_segment(self, with_led: bool, with_dot: bool) -> Segment;
}

impl Displayable for u8 {
    fn to_segment(self, led: bool, dot: bool) -> Segment {
        Segment {
            char: DIGITS_REPR[self as usize],
            digit: Some(self),
            led_on: led,
            with_dot: dot,
        }
    }
}