//! Contesto di disegno 2D per HUD e interfacce utente (Task T2.5).
//! Incapsula in modo sicuro le operazioni di disegno 2D senza richiedere accesso
//! diretto ai vertici o buffer GPU di Horizon.

use horizon_client_render::hud::{HudAtlas, HudBatch};

/// Colore RGBA a 32 bit per operazioni di disegno 2D.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const RED: Self = Self {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const GREEN: Self = Self {
        r: 0,
        g: 255,
        b: 0,
        a: 255,
    };
    pub const BLUE: Self = Self {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    #[inline]
    pub const fn as_u32(self) -> u32 {
        ((self.a as u32) << 24) | ((self.b as u32) << 16) | ((self.g as u32) << 8) | (self.r as u32)
    }
}

/// Contesto di disegno per HUD e schermate, fornito durante il render callback.
pub struct DrawContext<'a> {
    pub batch: &'a mut HudBatch,
    pub atlas: Option<&'a HudAtlas>,
    offset_x: f32,
    offset_y: f32,
}

impl<'a> DrawContext<'a> {
    pub fn new(batch: &'a mut HudBatch, atlas: Option<&'a HudAtlas>) -> Self {
        Self {
            batch,
            atlas,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }

    /// Applica una traslazione al sistema di coordinate.
    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.offset_x += dx;
        self.offset_y += dy;
    }

    /// Disegna un rettangolo pieno con colore uniforme.
    // PORT: GuiGraphics#fill(int, int, int, int, int)
    pub fn fill(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Color) {
        let min_x = (x1 + self.offset_x).min(x2 + self.offset_x);
        let max_x = (x1 + self.offset_x).max(x2 + self.offset_x);
        let min_y = (y1 + self.offset_y).min(y2 + self.offset_y);
        let max_y = (y1 + self.offset_y).max(y2 + self.offset_y);

        self.batch.quad(
            [
                [min_x, min_y],
                [max_x, min_y],
                [max_x, max_y],
                [min_x, max_y],
            ],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            color.as_u32(),
        );
    }

    /// Disegna il bordo di un rettangolo.
    pub fn draw_rect_outline(&mut self, x: f32, y: f32, width: f32, height: f32, color: Color) {
        self.fill(x, y, x + width, y + 1.0, color);
        self.fill(x, y + height - 1.0, x + width, y + height, color);
        self.fill(x, y + 1.0, x + 1.0, y + height - 1.0, color);
        self.fill(x + width - 1.0, y + 1.0, x + width, y + height - 1.0, color);
    }

    /// Disegna una stringa di testo all'offset specificato.
    // PORT: GuiGraphics#drawString(Font, String, int, int, int)
    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, color: Color) {
        if let Some(atlas) = self.atlas {
            let (pos_x, pos_y) = (x + self.offset_x, y + self.offset_y);
            self.batch
                .text(atlas, text, pos_x, pos_y, color.as_u32(), true);
        }
    }

    /// Larghezza e altezza logica dell'interfaccia.
    pub fn screen_size(&self) -> (f32, f32) {
        (self.batch.width, self.batch.height)
    }
}
