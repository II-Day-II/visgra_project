use ggez::{graphics::Color};

pub struct Texture {
    width: usize,
    height: usize,
    buffer: Vec<Color>,
}

impl Texture {
    pub fn new(width: usize, height: usize) -> Self {
        let mut buffer = vec![Color::WHITE; width * height];
        #[cfg(debug_assertions)]
        {
            let colors = vec![
                Color::RED,
                Color::BLUE,
                Color::GREEN,
                Color::WHITE,
                Color::YELLOW,
                Color::MAGENTA,
                Color::CYAN,
                Color::BLACK,
            ];
            for y in 0..height {
                for x in 0..width {
                    buffer[x + width * y] = colors[y % colors.len()];
                }
            }
        }
        Self {
            width,
            height,
            buffer,
        }
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn set_color(&mut self, x: usize, y: usize, c: Color) {
        self.buffer[x + self.width * y] = c;
    }

    pub fn get_color(&self, x: usize, y: usize) -> Color {
        self.buffer[x + self.width * y]
    }

    pub fn clear(&mut self) {
        for i in 0..self.width * self.height {
            self.buffer[i] = Color::WHITE;
        }
    }

    pub fn sample_color(&self, x: f32, y: f32) -> Color {
        let sx = (self.width as f32 * x) as usize;
        let sy = (self.height as f32 * y) as usize;
        if sx > self.width || sy > self.height {
            Color::BLACK
        } else {
            self.buffer[(sx + self.width * sy).clamp(0,self.buffer.len()-1)]
        }
    }
    
    pub fn sample_color_weighted(&self, x: f32, y: f32, w: f32) -> Color {
        let sx = (self.width as f32 * x) as usize;
        let sy = (self.height as f32 * y) as usize;
        if sx > self.width || sy > self.height {
            Color::BLACK
        } else {
            let base = self.buffer[(sx + self.width * sy).clamp(0,self.buffer.len()-1)];
            Color::from((base.r * w, base.g * w, base.b * w, base.a * w))
        }
    }
}
