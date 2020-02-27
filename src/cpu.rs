use rand::random;
use std::mem;

const MEM_SIZE: usize = 0x1000;
const FONT_START: usize = 0x0050;
const ROM_START: usize = 0x0200;
const MAX_ROM_SIZE: usize = MEM_SIZE - ROM_START;
pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;
pub const SCREEN_SIZE: usize = SCREEN_WIDTH * SCREEN_HEIGHT;
const KEY_COUNT: usize = 16;
const STACK_SIZE: usize = 16;

pub struct Cpu {
    memory: [u8; MEM_SIZE],
    v: [u8; 16],
    i: u16,
    pc: u16,
    screen: [u8; SCREEN_SIZE],
    delay_timer: u8,
    sound_timer: u8,
    stack: [u16; STACK_SIZE],
    sp: u16,
    key: [bool; KEY_COUNT],
    should_draw: bool,
}

impl Cpu {
    pub fn new() -> Cpu {
        let mut memory = [0; MEM_SIZE];
        memory[FONT_START..FONT_START + FONT_DATA.len()].copy_from_slice(&FONT_DATA);

        Cpu {
            memory,
            v: [0; 16],
            i: 0,
            pc: ROM_START as u16,
            screen: [0; SCREEN_SIZE],
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; STACK_SIZE],
            sp: 0,
            key: [false; KEY_COUNT],
            should_draw: false,
        }
    }

    pub fn load(&mut self, rom: &[u8]) {
        assert!(rom.len() < MAX_ROM_SIZE, "game larger than {} bytes", MAX_ROM_SIZE);
        self.memory[ROM_START..ROM_START + rom.len()].copy_from_slice(rom);
    }

    pub fn step(&mut self) {
        assert!(self.pc as usize + 1 < MEM_SIZE, "program counter overflow");
        let op_high = self.memory[self.pc as usize];
        let op_low = self.memory[(self.pc + 1) as usize];
        let op: u16 = ((op_high as u16) << 8) | op_low as u16;
        let vx = ((op & 0x0f00) >> 8) as usize;
        let vy = ((op & 0x00f0) >> 4) as usize;
        let n = (op & 0x000f) as u8;
        let nn = (op & 0x00ff) as u8;
        let nnn = op & 0x0fff;

        self.pc += 2;

        match ((op & 0xf000) >> 12, (op & 0x0f00) >> 8, (op & 0x00f0) >> 4, op & 0x000f) {
            (0x0, 0x0, 0xe, 0x0) => {
                self.screen.copy_from_slice(&[0; SCREEN_SIZE]);
                self.should_draw = true;
            },
            (0x0, 0x0, 0xe, 0xe) => {
                assert!(self.sp > 0, "stack pointer underflow");
                self.sp -= 1;
                self.pc = self.stack[self.sp as usize] + 2;
            },
            (0x1, _, _, _) => self.pc = nnn,
            (0x2, _, _, _) => {
                assert!((self.sp as usize) < STACK_SIZE, "stack pointer overflow");
                self.stack[self.sp as usize] = self.pc - 2;
                self.sp += 1;
                self.pc = nnn;
            },
            (0x3, _, _, _) => self.pc += if self.v[vx] == nn { 2 } else { 0 },
            (0x4, _, _, _) => self.pc += if self.v[vx] != nn { 2 } else { 0 },
            (0x5, _, _, _) => self.pc += if self.v[vx] == self.v[vy] { 2 } else { 0 },
            (0x6, _, _, _) => self.v[vx] = nn,
            (0x7, _, _, _) => self.v[vx] = self.v[vx].wrapping_add(nn),
            (0x8, _, _, 0x0) => self.v[vx] = self.v[vy],
            (0x8, _, _, 0x1) => self.v[vx] |= self.v[vy],
            (0x8, _, _, 0x2) => self.v[vx] &= self.v[vy],
            (0x8, _, _, 0x3) => self.v[vx] ^= self.v[vy],
            (0x8, _, _, 0x4) => {
                let (v, of) = self.v[vx].overflowing_add(self.v[vy]);
                self.v[0xf] = if of { 1 } else { 0 };
                self.v[vx] = v;
            },
            (0x8, _, _, 0x5) => {
                let (v, of) = self.v[vx].overflowing_sub(self.v[vy]);
                self.v[0xf] = if of { 0 } else { 1 };
                self.v[vx] = v;
            },
            (0x8, _, _, 0x6) => {
                self.v[0xf] = self.v[vx] & 0x01;
                self.v[vx] >>= 1;
            },
            (0x8, _, _, 0x7) => {
                let (v, of) = self.v[vy].overflowing_sub(self.v[vx]);
                self.v[0xf] = if of { 0 } else { 1 };
                self.v[vx] = v;
            },
            (0x8, _, _, 0xe) => {
                self.v[0xf] = self.v[vx] & 0x80;
                self.v[vx] <<= 1;
            },
            (0x9, _, _, _) => self.pc += if self.v[vx] != self.v[vy] { 2 } else { 0 },
            (0xa, _, _, _) => self.i = nnn,
            (0xb, _, _, _) => self.pc = nnn + self.v[0x0] as u16,
            (0xc, _, _, _) => self.v[vx] = random::<u8>() & nn,
            (0xd, _, _, _) => {
                let x = self.v[vx] as usize;
                let y = self.v[vy] as usize;

                self.v[0xf] = 0;

                for line in 0..(n as u16) {
                    let data = self.memory[(self.i + line) as usize];
                    let y_off = ((y as u16 + line) * SCREEN_WIDTH as u16) as u16;
                    for b in 0..8 {
                        if (data & (0x80 >> b)) != 0 {
                            let mut pixel = x as u16 + b as u16 + y_off;
                            // Height wrapping behavior:
                            pixel %= SCREEN_SIZE as u16;

                            if self.screen[pixel as usize] == 1 {
                                self.v[0xf] = 1;
                            }

                            self.screen[pixel as usize] ^= 1;
                        }
                    }
                }

                self.should_draw = true;
            },
            (0xe, _, 0x9, 0xe) => self.pc += if self.key[self.v[vx] as usize] { 2 } else { 0 },
            (0xe, _, 0xa, 0x1) => self.pc += if !self.key[self.v[vx] as usize] { 2 } else { 0 },
            (0xf, _, 0x0, 0x7) => self.v[vx] = self.delay_timer,
            (0xf, _, 0x0, 0xa) => {
                if let Some((index, _)) = self.key.iter().enumerate().filter(|(_, &k)| k).next() {
                    self.v[vx] = index as u8;
                } else {
                    self.pc -= 2;
                }
            },
            (0xf, _, 0x1, 0x5) => self.delay_timer = self.v[vx],
            (0xf, _, 0x1, 0x8) => self.sound_timer = self.v[vx],
            (0xf, _, 0x1, 0xe) => {
                self.sound_timer = self.v[vx];
                self.i += self.v[vx] as u16;
                if self.i >= MEM_SIZE as u16 {
                    self.i %= MEM_SIZE as u16;
                    self.v[0xf] = 1;
                } else {
                    self.v[0xf] = 0;
                }
            },
            (0xf, _, 0x2, 0x9) => self.i = FONT_START as u16 + (self.v[vx] as u16 & 0xf) * 5,
            (0xf, _, 0x3, 0x3) => {
                self.memory[self.i as usize] = self.v[vx] / 100;
                self.memory[self.i as usize + 1] = (self.v[vx] / 10) % 10;
                self.memory[self.i as usize + 2] = (self.v[vx] % 100) % 10;
            },
            (0xf, _, 0x5, 0x5) => {
                for index in 0..vx + 1 {
                    self.memory[self.i as usize + index] = self.v[index];
                }
            },
            (0xf, _, 0x6, 0x5) => {
                for index in 0..vx + 1 {
                    self.v[index] = self.memory[self.i as usize + index];
                }
            },
            _ => panic!("unsupported opcode 0x{:04x}", op),
        }

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            // TODO: Somehow play sound.
            self.sound_timer -= 1;
        }
    }

    pub fn set_key(&mut self, index: usize, on: bool) {
        assert!(index < KEY_COUNT, "key index out of range");
        self.key[index] = on;
    }

    pub fn new_frame(&mut self) -> Option<&[u8; SCREEN_SIZE]> {
        match mem::replace(&mut self.should_draw, false) {
            true => Some(&self.screen),
            false => None,
        }
    }
}

const FONT_DATA: [u8; 16 * 5] = [
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
