use std::path::PathBuf;
use egui_macroquad::egui;
use egui_macroquad::egui::Rect;
use rfd::FileDialog;

pub(crate) struct Ui {
    pub(crate) reg_edit: egui_memory_editor::MemoryEditor,
    pub(crate) ram_edit: egui_memory_editor::MemoryEditor,
    pub(crate) is_mem_edit_open: bool,
    pub(crate) rect: Rect,
    pub(crate) rom_path: Option<PathBuf>,
}

impl Default for Ui {
    fn default() -> Self {
        let ram_edit = egui_memory_editor::MemoryEditor::new()
            .with_address_range("RAM", 0..0x1000)
            .with_window_title("RAM");

        let reg_edit = egui_memory_editor::MemoryEditor::new()
            .with_address_range("Registers", 0..0x10)
            .with_window_title("Registers");

        Self {
            reg_edit,
            ram_edit,
            is_mem_edit_open: false,
            rect: Rect::ZERO,
            rom_path: None,
        }
    }
}

impl Ui {
    pub(crate) fn render(
        &mut self,
        memory: &mut [u8; 4096],
        registres: &mut [u8; 16],
        show_grid: &mut bool,
        cycle_advance: &mut bool,
        shift_quirk: &mut bool,
        current_opcode: u16,
    ) {
        egui_macroquad::ui(|egui_ctx| {
            let side_panel = egui::SidePanel::right("Debug")
                .show(egui_ctx, |ui| {

                    ui.heading("CHIP8 Emulator");

                    if ui.button("Select ROM").clicked() {
                        let file = FileDialog::new()
                            .add_filter("CHIP8 ROM", &["ch8"])
                            .pick_file();
                        self.rom_path = file;
                    }

                    ui.separator();

                    ui.checkbox(show_grid, "Show grid");
                    ui.checkbox(&mut self.is_mem_edit_open, "Show memory editor");
                    ui.checkbox(cycle_advance, "Cycle advance")
                        .on_hover_text("Press 'L' to advance one cycle forward");

                    ui.label(format!("Current opcode: {:02X}", current_opcode));

                    ui.collapsing("Quirks", |ui| {
                        ui.checkbox(shift_quirk, "Shift quirk");
                    });
                });

            self.ram_edit.window_ui(
                egui_ctx, &mut self.is_mem_edit_open, memory,
                |memory, address| Some(memory[address]),
                |memory, address, value| memory[address] = value
            );
            self.reg_edit.window_ui(
                egui_ctx, &mut self.is_mem_edit_open, registres,
                |memory, address| Some(memory[address]),
                |memory, address, value| memory[address] = value
            );

            self.rect = side_panel.response.rect;
        });
    }

    pub(crate) fn draw(&self) {
        egui_macroquad::draw();
    }
}