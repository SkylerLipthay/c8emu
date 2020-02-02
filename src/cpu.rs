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
        let opcode_high = self.memory[self.pc as usize];
        let opcode_low = self.memory[(self.pc + 1) as usize];
        let opcode: u16 = ((opcode_high as u16) << 8) | opcode_low as u16;
        let vx = ((opcode & 0x0f00) >> 8) as usize;
        let vy = ((opcode & 0x00f0) >> 4) as usize;
        let nn = (opcode & 0x00ff) as u8;
        let nnn = opcode & 0x0fff;

        match opcode & 0xf000 {
            0x0000 if opcode == 0x00e0 => {
                self.screen.copy_from_slice(&[0; SCREEN_SIZE]);
                self.should_draw = true;
                self.pc += 2;
            },
            0x0000 if opcode == 0x00ee => {
                assert!(self.sp > 0, "stack pointer underflow");
                self.sp -= 1;
                self.pc = self.stack[self.sp as usize] + 2;
            },
            0x1000 => {
                self.pc = nnn;
            },
            0x2000 => {
                assert!((self.sp as usize) < STACK_SIZE, "stack pointer overflow");
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = nnn;
            },
            0x3000 => {
                self.pc += if self.v[vx] == nn { 4 } else { 2 };
            },
            0x4000 => {
                self.pc += if self.v[vx] != nn { 4 } else { 2 };
            },
            0x5000 => {
                self.pc += if self.v[vx] == self.v[vy] { 4 } else { 2 };
            },
            0x6000 => {
                self.v[vx] = nn;
                self.pc += 2;
            },
            0x7000 => {
                self.v[vx] = self.v[vx].wrapping_add(nn);
                self.pc += 2;
            },
            0x8000 if opcode & 0x000f == 0x0000 => {
                self.v[vx] = self.v[vy];
                self.pc += 2;
            },
            0x8000 if opcode & 0x000f == 0x0001 => {
                self.v[vx] |= self.v[vy];
                self.pc += 2;
            },
            0x8000 if opcode & 0x000f == 0x0002 => {
                self.v[vx] &= self.v[vy];
                self.pc += 2;
            },
            0x8000 if opcode & 0x000f == 0x0003 => {
                self.v[vx] ^= self.v[vy];
                self.pc += 2;
            },
            0x8000 if opcode & 0x000f == 0x0004 => {
                let (v, of) = self.v[vx].overflowing_add(self.v[vy]);
                self.v[0xf] = if of { 1 } else { 0 };
                self.v[vx] = v;
                self.pc += 2;
            },
            0x8000 if opcode & 0x000f == 0x0005 => {
                let (v, of) = self.v[vx].overflowing_sub(self.v[vy]);
                self.v[0xf] = if of { 0 } else { 1 };
                self.v[vx] = v;
                self.pc += 2;
            },
            0x8000 if opcode & 0x000f == 0x0006 => {
                self.v[0xf] = self.v[vx] & 0x01;
                self.v[vx] >>= 1;
                self.pc += 2;
            },
            0x8000 if opcode & 0x000f == 0x0007 => {
                let (v, of) = self.v[vy].overflowing_sub(self.v[vx]);
                self.v[0xf] = if of { 0 } else { 1 };
                self.v[vx] = v;
                self.pc += 2;
            },
            0x8000 if opcode & 0x000f == 0x000e => {
                self.v[0xf] = self.v[vx] & 0x80;
                self.v[vx] <<= 1;
                self.pc += 2;
            },
            0x9000 => {
                self.pc += if self.v[vx] != self.v[vy] { 4 } else { 2 };
            },
            0xa000 => {
                self.i = nnn;
                self.pc += 2;
            },
            0xb000 => {
                self.pc = nnn + self.v[0x0] as u16;
            },
            0xc000 => {
                self.v[vx] = random::<u8>() & nn;
                self.pc += 2;
            },
            0xd000 => {
                let x = self.v[vx] as usize;
                let y = self.v[vy] as usize;
                let height = opcode & 0x000f;

                self.v[0xf] = 0;

                for line in 0..height {
                    let data = self.memory[(self.i + line) as usize];

                    for b in 0..8 {
                        if (data & (0x80 >> b)) != 0 {
                            let mut pixel = x as u16 + b as u16 + ((y as u16 + line) * SCREEN_WIDTH as u16) as u16;
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
                self.pc += 2;
            },
            0xe000 if opcode & 0x00ff == 0x009e => {
                self.pc += if self.key[self.v[vx] as usize] { 4 } else { 2 };
            },
            0xe000 if opcode & 0x00ff == 0x00a1 => {
                self.pc += if !self.key[self.v[vx] as usize] { 4 } else { 2 };
            },
            0xf000 if opcode & 0x00ff == 0x0007 => {
                self.v[vx] = self.delay_timer;
                self.pc += 2;
            },
            0xf000 if opcode & 0x00ff == 0x000a => {
                for (index, &key) in self.key.iter().enumerate() {
                    if key {
                        self.v[vx] = index as u8;
                        self.pc += 2;
                        break;
                    }
                }
            },
            0xf000 if opcode & 0x00ff == 0x0015 => {
                self.delay_timer = self.v[vx];
                self.pc += 2;
            },
            0xf000 if opcode & 0x00ff == 0x0018 => {
                self.sound_timer = self.v[vx];
                self.pc += 2;
            },
            0xf000 if opcode & 0x00ff == 0x001e => {
                self.sound_timer = self.v[vx];
                self.i += self.v[vx] as u16;
                if self.i >= MEM_SIZE as u16 {
                    self.i %= MEM_SIZE as u16;
                    self.v[0xf] = 1;
                } else {
                    self.v[0xf] = 0;
                }
                self.pc += 2;
            },
            0xf000 if opcode & 0x00ff == 0x0029 => {
                self.i = FONT_START as u16 + (self.v[vx] as u16 & 0xf) * 5;
                self.pc += 2;
            },
            0xf000 if opcode & 0x00ff == 0x0033 => {
                self.memory[self.i as usize] = self.v[vx] / 100;
                self.memory[self.i as usize + 1] = (self.v[vx] / 10) % 10;
                self.memory[self.i as usize + 2] = (self.v[vx] % 100) % 10;
                self.pc += 2;
            },
            0xf000 if opcode & 0x00ff == 0x0055 => {
                for index in 0..vx + 1 {
                    self.memory[self.i as usize + index] = self.v[index];
                }
                self.pc += 2;
            },
            0xf000 if opcode & 0x00ff == 0x0065 => {
                for index in 0..vx + 1 {
                    self.v[index] = self.memory[self.i as usize + index];
                }
                self.pc += 2;
            },
            _ => panic!("unsupported opcode 0x{:04x}", opcode),
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
