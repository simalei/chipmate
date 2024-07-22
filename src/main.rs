mod chip8;
mod screen;
mod ui;

use log::LevelFilter;
use macroquad::prelude::*;
use crate::chip8::Chip8;
use crate::ui::Ui;

#[macroquad::main("chipmate")]
async fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();

    let mut chip8 = Chip8::default();
    let mut ui = Ui::default();

    loop {
        clear_background(BLACK);

        // If user pressed button 'Select ROM'...
        match ui.rom_path {
            None => {} // If user pressed 'Cancel' in file dialog
            Some(ref path) => { // If user selected a file
                chip8.reset(); // Reset emulator state to prepare for loading

                match chip8.load_rom(path) { // Load new ROM
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("{e}");
                    }
                }
                ui.rom_path = None; // Reset status so that ROM won't be reloaded every frame
            }
        }

        chip8.process_input();


        if (chip8.cycle_advance && !chip8.block_cycle) 
            || (!chip8.cycle_advance && chip8.block_cycle) {
            match chip8.cycle() {
                Ok(_) => {}
                Err(e) => {
                    log::error!("{e}");
                }
            }
            chip8.block_cycle = true;
        }

        // Calculate UI
        ui.render(&mut chip8.memory,
                  &mut chip8.registers,
                  &mut chip8.screen.show_grid,
                  &mut chip8.cycle_advance,
                  &mut chip8.shift_quirk,
                  chip8.opcode
        );

        // Update emulator screen
        chip8.screen.update(&ui.rect);

        // Draw UI
        ui.draw();

        next_frame().await
    }
}
