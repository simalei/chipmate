use macroquad::prelude::*;


pub(crate) const SCREEN_WIDTH: f32 = 64.0;
pub(crate) const SCREEN_HEIGHT: f32 = 32.0;
pub(crate) struct Screen {
    pub(crate) state: [[bool; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize],
    pub(crate) show_grid: bool
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            show_grid: false,
            state: [[false; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize]
        }
    }
}

impl Screen {

    pub(crate) fn reset(&mut self) {
        self.state = [[false; SCREEN_WIDTH as usize]; SCREEN_HEIGHT as usize]
    }

    pub(crate) fn update(&self, rect: &egui_macroquad::egui::Rect) {
        let mut y = 0.0;
        let y_inc = screen_height() / SCREEN_HEIGHT;

        for row in self.state {


            let mut x = 0.0;
            let x_inc = (screen_width() - rect.width()) / SCREEN_WIDTH;

            if self.show_grid {
                draw_line(0.0, y, screen_width(), y, 1.0, GREEN);
            }

            for pixel in row {
                if self.show_grid {
                    draw_line(x, 0.0, x, screen_height(), 1.0, GREEN);
                }
                if pixel {
                    draw_rectangle(x, y, x_inc, y_inc, WHITE);
                } else {
                    draw_rectangle(x, y, x_inc, y_inc, BLACK);
                }
                x += x_inc;
            }
            y += y_inc;
        }
    }
}