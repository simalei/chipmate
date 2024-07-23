use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use macroquad::input::{is_key_down, is_key_pressed};
use macroquad::prelude::{get_keys_down, KeyCode::*};
use rand::random;
use thiserror::Error;
use crate::screen::{Screen, SCREEN_HEIGHT, SCREEN_WIDTH};

const START_ADDRESS: usize = 0x200;
const FONTSET_START_ADDRESS: usize = 0x50;
const FONTSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

pub(crate) struct Chip8 {
    pub(crate) registers: [u8; 16],
    pub(crate) memory: [u8; 4096],
    index: u16,
    pc: u16,
    stack: [u16; 16],
    stack_ptr: u8,
    delay_timer: u8,
    sound_timer: u8,
    keypad: [bool; 16],
    pub(crate) screen: Screen,
    pub(crate) opcode: u16,
    pub(crate) cycle_advance: bool,
    pub(crate) block_cycle: bool,
    pub(crate) shift_quirk: bool
}

#[derive(Error, Debug)]
pub(crate) enum Chip8Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unimplemented opcode {0:X}")]
    UnimplementedOpcode(u16)
}

type Chip8Result<T> = Result<T, Chip8Error>;

impl Default for Chip8 {
    fn default() -> Self {
        let mut memory: [u8; 4096] = [0; 4096];
        memory[FONTSET_START_ADDRESS..(FONTSET.len() + FONTSET_START_ADDRESS)].copy_from_slice(&FONTSET[..]);


        Self {
            registers: [0; 16],
            memory,
            index: 0,
            pc: 0x200,
            stack: [0; 16],
            stack_ptr: 0,
            delay_timer: 0,
            sound_timer: 0,
            keypad: [false; 16],
            screen: Screen::default(),
            opcode: 0,
            cycle_advance: false,
            block_cycle: true,
            shift_quirk: false,
        }
    }
}

impl Chip8 {
    pub(crate) fn reset(&mut self) {
        let mut memory: [u8; 4096] = [0; 4096];

        memory[FONTSET_START_ADDRESS..(FONTSET.len() + FONTSET_START_ADDRESS)].copy_from_slice(&FONTSET[..]);

        self.memory = memory;
        self.registers = [0; 16];
        self.pc = 0x200;
        self.index = 0;
        self.stack = [0; 16];
        self.stack_ptr = 0;
        self.delay_timer = 0;
        self.sound_timer = 0;
        self.keypad = [false; 16];
        self.opcode = 0;

        self.screen.reset();
    }

    pub(crate) fn load_rom(&mut self, path: &PathBuf) -> Chip8Result<()> {
        let mut file = File::open(path.clone())?;
        let mut buffer: [u8; 4096] = [0; 4096];
        let size = file.metadata()?.len() as usize;

        #[allow(clippy::unused_io_amount)] // It's fine if we have some buffer space left
        file.read(&mut buffer)?;


        self.memory[START_ADDRESS..(size + START_ADDRESS)].copy_from_slice(&buffer[..size]);

        log::info!("Successfully loaded {}", path.to_str().unwrap());
        Ok(())
    }

    pub(crate) fn process_input(&mut self) {
        let pressed_keys = get_keys_down();
        self.keypad[1] = pressed_keys.contains(&Key1);
        self.keypad[2] = pressed_keys.contains(&Key2);
        self.keypad[3] = pressed_keys.contains(&Key3);
        self.keypad[12] = pressed_keys.contains(&Key4);
        self.keypad[4] = pressed_keys.contains(&Q);
        self.keypad[5] = pressed_keys.contains(&W);
        self.keypad[6] = pressed_keys.contains(&E);
        self.keypad[13] = pressed_keys.contains(&R);
        self.keypad[7] = pressed_keys.contains(&A);
        self.keypad[8] = pressed_keys.contains(&S);
        self.keypad[9] = pressed_keys.contains(&D);
        self.keypad[14] = pressed_keys.contains(&F);
        self.keypad[10] = pressed_keys.contains(&Z);
        self.keypad[0] = pressed_keys.contains(&X);
        self.keypad[11] = pressed_keys.contains(&C);
        self.keypad[15] = pressed_keys.contains(&V);

        if is_key_pressed(L) && self.cycle_advance {
            log::info!("Advancing one cycle forward...");
            self.block_cycle = false;
        }
        if is_key_down(K) && self.cycle_advance {
            self.block_cycle = false;
        }

    }

    pub(crate) fn execute(&mut self) -> Chip8Result<()> {

        let digit1 = (self.opcode & 0xF000) >> 12;
        let digit2 = (self.opcode & 0x0F00) >> 8;
        let digit3 = (self.opcode & 0x00F0) >> 4;
        let digit4 = self.opcode & 0x000F;

        match (digit1, digit2, digit3, digit4) {
            (0, 0, 0, 0) => {
                // noop
            }
            (0, 0, 0xE, 0) => {
                self.screen.state = [[false; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize]
            }
            (0, 0, 0xE, 0xE) => {
                self.stack_ptr -= 1;
                self.pc = self.stack[self.stack_ptr as usize];
            }
            (1, _, _, _) => {
                let dest = self.opcode & 0xFFF;
                self.pc = dest;
            }
            (2, _, _, _) => {
                let dest = self.opcode & 0xFFF;
                self.stack[self.stack_ptr as usize] = self.pc;
                self.stack_ptr += 1;
                self.pc = dest;
            }
            (3, _, _, _) => {
                let vx: usize = digit2 as usize;
                let byte = self.opcode & 0x00FF;

                if self.registers[vx] == byte as u8 {
                    self.pc += 2;
                }
            }
            (4, _, _, _) => {
                let vx: usize = digit2 as usize;
                let byte = self.opcode & 0x00FF;

                if self.registers[vx] != byte as u8 {
                    self.pc += 2;
                }
            }
            (5, _, _, 0) => {
                let vx: usize = digit2 as usize;
                let vy: usize = digit3 as usize;

                if self.registers[vx] == self.registers[vy] {
                    self.pc += 2;
                }
            }
            (6, _, _, _) => {
                let vx: usize = digit2 as usize;
                let nn = self.opcode & 0x00FF;

                self.registers[vx] = nn as u8;
            }
            (7, _, _, _) => {
                let vx: usize = digit2 as usize;
                let nn = self.opcode & 0x00FF;

                self.registers[vx] = (self.registers[vx] as u16 + nn) as u8;
            }
            (8, _, _, 0) => {
                let vx: usize = digit2 as usize;
                let vy: usize = digit3 as usize;

                self.registers[vx] = self.registers[vy];
            }
            (8, _, _, 1) => {
                let vx: usize = digit2 as usize;
                let vy: usize = digit3 as usize;

                self.registers[vx] |= self.registers[vy];
            }
            (8, _, _, 2) => {
                let vx: usize = digit2 as usize;
                let vy: usize = digit3 as usize;

                self.registers[vx] &= self.registers[vy];
            }
            (8, _, _, 3) => { // 8XY3
                let vx: usize = digit2 as usize;
                let vy: usize = digit3 as usize;

                self.registers[vx] ^= self.registers[vy];
            }
            (8, _, _, 4) => { // 8XY4
                let vx: usize = digit2 as usize;
                let vy: usize = digit3 as usize;

                let (result, overflow) = self.registers[vx].overflowing_add(self.registers[vy]);

                self.registers[vx] = result;
                self.registers[0xF] = if overflow { 1 } else { 0 };
            }
            (8, _, _, 5) => { // 8XY5
                let vx: usize = digit2 as usize;
                let vy: usize = digit3 as usize;

                let (result, borrow) = self.registers[vx].overflowing_sub(self.registers[vy]);

                self.registers[vx] = result;
                self.registers[0xF] = if !borrow { 1 } else { 0 };
            }
            (8, _, _, 6) => {
                let vx: usize = digit2 as usize;
                let vy: usize = digit3 as usize;

                if self.shift_quirk {
                    self.registers[vy] = self.registers[vx];
                }
                let lsb = self.registers[vx] & 1;
                self.registers[vx] >>= 1;
                self.registers[0xF] = lsb;
            }
            (8, _, _, 7) => { // 8XY7
                let vx: usize = digit2 as usize;
                let vy: usize = digit3 as usize;

                let (result, borrow) = self.registers[vy].overflowing_sub(self.registers[vx]);

                self.registers[vx] = result;
                self.registers[0xF] = if !borrow { 1 } else { 0 };
            }
            (8, _, _, 0xE) => {
                let vx: usize = digit2 as usize;
                let vy: usize = digit3 as usize;

                if self.shift_quirk {
                    self.registers[vy] = self.registers[vx];
                }
                let msb = (self.registers[vx] >> 7) & 1;
                self.registers[vx] <<= 1;
                self.registers[0xF] = msb;

            }
            (9, _, _, 0) => {
                let vx: usize = digit2 as usize;
                let vy: usize = digit3 as usize;


                if self.registers[vx] != self.registers[vy] {
                    self.pc += 2;
                }
            }
            (0xA, _, _, _) => {
                let nnn = self.opcode & 0xFFF;

                self.index = nnn;
            }
            (0xB, _, _, _) => {
                let nnn = self.opcode & 0xFFF;

                self.pc = self.registers[0] as u16 + nnn;
            }
            (0xC, _, _, _) => {
                let vx: usize = digit2 as usize;
                let byte = self.opcode & 0x00FF;

                self.registers[vx] = random::<u8>() & byte as u8;
            }
            (0xD, _, _, _) => {
                let x = self.registers[digit2 as usize] as u16;
                let y = self.registers[digit3 as usize] as u16;
                let n = digit4;

                let mut flipped = false;

                for row in 0..n {
                    let spr_byte = self.memory[self.index as usize + row as usize];

                    for col in 0..8 {
                        if (spr_byte & (0b1000_0000 >> col)) != 0 {
                            let x = (x + col) as usize % SCREEN_WIDTH as usize;
                            let y = (y + row) as usize % SCREEN_HEIGHT as usize;

                            if self.screen.state[y][x] {
                                flipped = true;
                            }
                            self.screen.state[y][x] ^= true;
                        }
                    }
                }

                if flipped {
                    self.registers[0xF] = 1;
                } else {
                    self.registers[0xF] = 0;
                }
            }
            (0xE, _, 9, 0xE) => {
                let vx: usize = digit2 as usize;
                let key = self.registers[vx] as usize;

                if self.keypad[key] {
                    self.pc += 2;
                }
            }
            (0xE, _, 0xA, 1) => {
                let vx: usize = digit2 as usize;
                let key = self.registers[vx] as usize;

                if !self.keypad[key] {
                    self.pc += 2;
                }
            }
            (0xF, _, 0, 7) => {
                let vx: usize = digit2 as usize;

                self.registers[vx] = self.delay_timer;
            }
            (0xF, _, 0, 0xA) => {
                let vx: usize = digit2 as usize;

                if self.keypad[0] {
                    self.registers[vx] = 0;
                } else if self.keypad[1] {
                    self.registers[vx] = 1;
                } else if self.keypad[2] {
                    self.registers[vx] = 2;
                } else if self.keypad[3] {
                    self.registers[vx] = 3;
                } else if self.keypad[4] {
                    self.registers[vx] = 4;
                } else if self.keypad[5] {
                    self.registers[vx] = 5;
                } else if self.keypad[6] {
                    self.registers[vx] = 6;
                } else if self.keypad[7] {
                    self.registers[vx] = 7;
                } else if self.keypad[8] {
                    self.registers[vx] = 8;
                } else if self.keypad[9] {
                    self.registers[vx] = 9;
                } else if self.keypad[10] {
                    self.registers[vx] = 10;
                } else if self.keypad[11] {
                    self.registers[vx] = 11;
                } else if self.keypad[12] {
                    self.registers[vx] = 12;
                } else if self.keypad[13] {
                    self.registers[vx] = 13;
                } else if self.keypad[14] {
                    self.registers[vx] = 14;
                } else if self.keypad[15] {
                    self.registers[vx] = 15;
                } else {
                    self.pc -= 2;
                }
            }
            (0xF, _, 1, 5) => {
                let vx: usize = digit2 as usize;

                self.delay_timer = self.registers[vx];
            }
            (0xF, _, 1, 8) => {
                let vx: usize = digit2 as usize;

                self.sound_timer = self.registers[vx];
            }
            (0xF, _, 1, 0xE) => {
                let vx: usize = digit2 as usize;

                self.index += self.registers[vx] as u16;
            }
            (0xF, _, 2, 9) => {
                let vx: usize = digit2 as usize;
                let digit = self.registers[vx];

                self.index = FONTSET_START_ADDRESS as u16 + (5 * digit as u16);
            }
            (0xF, _, 3, 3) => {
                let vx = digit2 as usize;
                let value = self.registers[vx] as f32;
                let hundreds = (value / 100.0).floor() as u8;
                let tens = ((value / 10.0) % 10.0).floor() as u8;
                let ones = (value % 10.0) as u8;
                self.memory[self.index as usize] = hundreds;
                self.memory[(self.index + 1) as usize] = tens;
                self.memory[(self.index + 2) as usize] = ones;
            }
            (0xF, _, 5, 5) => {
                let x = digit2 as usize; let i = self.index as usize; for idx in 0..=x { self.memory[i + idx] = self.registers[idx]; }
            }
            (0xF, _, 6, 5) => {
                let x = digit2 as usize; let i = self.index as usize; for idx in 0..=x { self.registers[idx] = self.memory[i + idx]; }
            }
            (_, _, _, _) => {
                return Err(Chip8Error::UnimplementedOpcode(self.opcode))
            }
        }


        Ok(())
    }

    pub(crate) fn cycle(&mut self) -> Chip8Result<()> {
        let hi_byte = self.memory[self.pc as usize] as u16;
        let lo_byte = self.memory[(self.pc + 1) as usize] as u16;
        self.opcode = (hi_byte << 8) | lo_byte;

        self.pc += 2;

        self.execute()?;

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_8xy4() {
        let mut chip8 = Chip8::default();
        chip8.opcode = 0x81D4;
        chip8.registers[1] = 25;
        chip8.registers[0xD] = 30;

        chip8.execute().unwrap();
        assert_eq!(chip8.registers[1], 55);
    }
    #[test]
    fn op_8xy4_overflow() {
        let mut chip8 = Chip8::default();
        chip8.opcode = 0x81D4;
        chip8.registers[1] = 254;
        chip8.registers[0xD] = 30;

        chip8.execute().unwrap();
        assert_eq!(chip8.registers[1], 28);
        assert_eq!(chip8.registers[0xF], 1);
    }
    #[test]
    fn op_8xy4_vf_as_x() {
        let mut chip8 = Chip8::default();
        chip8.opcode = 0x8FD4;
        chip8.registers[0xF] = 25;
        chip8.registers[0xD] = 25;

        chip8.execute().unwrap();
        assert_eq!(chip8.registers[0xF], 50);
    }

    #[test]
    fn op_8xy4_vf_as_y() {
        let mut chip8 = Chip8::default();
        chip8.opcode = 0x8DF4;
        chip8.registers[0xF] = 25;
        chip8.registers[0xD] = 25;

        chip8.execute().unwrap();
        assert_eq!(chip8.registers[0xD], 50);
    }
}
