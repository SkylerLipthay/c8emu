extern crate argh;
extern crate minifb;
extern crate rand;

mod cpu;

use argh::FromArgs;
use std::{fs::File, io::Read, time};

#[derive(FromArgs)]
#[argh(description = "A CHIP-8 emulator")]
struct C8EmuArgs {
    #[argh(option, description = "frames per second (default 60)", default = "60.0")]
    fps: f64,
    #[argh(option, description = "instructions per frame (default 10)", default = "10")]
    ipf: u64,
    #[argh(positional)]
    input: String,
}

fn main() {
    let args: C8EmuArgs = argh::from_env();

    let mut window = minifb::Window::new(
        "CHIP-8",
        cpu::SCREEN_WIDTH,
        cpu::SCREEN_HEIGHT,
        minifb::WindowOptions {
            borderless: false,
            title: true,
            resize: false,
            scale: minifb::Scale::X8,
            scale_mode: minifb::ScaleMode::AspectRatioStretch,
        },
    ).unwrap();

    window.limit_update_rate(Some(time::Duration::from_secs_f64(1.0 / args.fps)));

    let mut cpu = cpu::Cpu::new();
    let mut file = File::open(args.input).unwrap();
    let mut data = Vec::new();
    file.read_to_end(&mut data).unwrap();
    cpu.load(&data);

    let mut buffer = [0u32; cpu::SCREEN_SIZE];
    while window.is_open() && !window.is_key_down(minifb::Key::Escape) {
        cpu.set_key(0x1, window.is_key_down(minifb::Key::Key1));
        cpu.set_key(0x2, window.is_key_down(minifb::Key::Key2));
        cpu.set_key(0x3, window.is_key_down(minifb::Key::Key3));
        cpu.set_key(0xc, window.is_key_down(minifb::Key::Key4));
        cpu.set_key(0x4, window.is_key_down(minifb::Key::Q));
        cpu.set_key(0x5, window.is_key_down(minifb::Key::W));
        cpu.set_key(0x6, window.is_key_down(minifb::Key::E));
        cpu.set_key(0xd, window.is_key_down(minifb::Key::R));
        cpu.set_key(0x7, window.is_key_down(minifb::Key::A));
        cpu.set_key(0x8, window.is_key_down(minifb::Key::S));
        cpu.set_key(0x9, window.is_key_down(minifb::Key::D));
        cpu.set_key(0xe, window.is_key_down(minifb::Key::F));
        cpu.set_key(0xa, window.is_key_down(minifb::Key::Z));
        cpu.set_key(0x0, window.is_key_down(minifb::Key::X));
        cpu.set_key(0xb, window.is_key_down(minifb::Key::C));
        cpu.set_key(0xf, window.is_key_down(minifb::Key::V));

        for _ in 0..args.ipf {
            cpu.step();
        }

        if let Some(new_buffer) = cpu.new_frame() {
            for index in 0..cpu::SCREEN_SIZE {
                buffer[index] = if new_buffer[index] == 0 { 0x9bbc0f } else { 0x0f380f };
            }
        }
        window.update_with_buffer(&buffer, cpu::SCREEN_WIDTH, cpu::SCREEN_HEIGHT).unwrap();
    }

    loop {
        cpu.step();
        break;
    }
}
