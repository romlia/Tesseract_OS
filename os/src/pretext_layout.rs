use crate::html_parser::{HtmlTag, SemanticBlock};
use crate::font_8x8::BASIC_FONT;

pub struct PretextLayoutEngine {
    pub width: u32,
    pub height: u32,
    pub glyph_buffer: Vec<u32>, // 1920x1080 array of ARGB 32-bit text pixels
    pub is_empty: bool,
    cursor_x: u32,
    cursor_y: u32,
}

impl PretextLayoutEngine {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            glyph_buffer: vec![0; (width * height) as usize],
            is_empty: true,
            cursor_x: 20,
            cursor_y: 40,
        }
    }

    pub fn clear(&mut self) {
        self.glyph_buffer.fill(0);
        self.is_empty = true;
        self.cursor_x = 20;
        self.cursor_y = 40;
    }

    pub fn layout_blocks(&mut self, blocks: &[SemanticBlock<'_>]) {
        if !blocks.is_empty() {
            self.is_empty = false;
        }
        for block in blocks {
            let SemanticBlock::Text { content, tag } = block;
            
            let (font_size, color) = match tag {
                HtmlTag::H1 => (3, 0xFF_00_FF_00), // Large Green
                HtmlTag::H2 => (2, 0xFF_00_CC_00), // Medium Green
                HtmlTag::P => (1, 0xFF_AA_AA_AA),  // Normal Gray
                HtmlTag::Div => (1, 0xFF_AA_AA_AA),
                HtmlTag::Span => (1, 0xFF_FF_FF_00), // Yellow
                HtmlTag::Unknown => (1, 0xFF_FF_00_00), // Red
            };

            for c in content.chars() {
                if c == '\n' {
                    self.cursor_x = 20;
                    self.cursor_y += 20 * font_size;
                    continue;
                }
                
                // Extremely basic mathematical glyph rendering (just blocks for now)
                // In a full implementation, we'd use an 8x8 binary font atlas.
                self.draw_char(c, self.cursor_x, self.cursor_y, font_size, color);
                
                self.cursor_x += 10 * font_size;
                if self.cursor_x > self.width - 20 {
                    self.cursor_x = 20;
                    self.cursor_y += 20 * font_size;
                }
            }
            self.cursor_y += 30; // Block spacing
            self.cursor_x = 20;
        }
    }

    fn draw_char(&mut self, c: char, x: u32, y: u32, scale: u32, color: u32) {
        // Bitwise 64-bit Rasterization
        // By evaluating an entire byte row of the 8x8 font, we eliminate heavy branching.
        let char_idx = if (c as usize) < 128 { c as usize } else { 0 };
        let font_data = BASIC_FONT[char_idx];
        
        for dy in 0..8 {
            let row = font_data[dy as usize];
            if row == 0 { continue; } // Fast skip transparent rows
            
            for s_y in 0..scale {
                let py = y + dy * scale + s_y;
                if py >= self.height { break; }
                
                for dx in 0..8 {
                    // Check if the specific bit is set (1)
                    if (row >> (7 - dx)) & 1 == 1 {
                        for s_x in 0..scale {
                            let px = x + dx * scale + s_x;
                            if px < self.width {
                                let idx = (py * self.width + px) as usize;
                                self.glyph_buffer[idx] = color;
                            }
                        }
                    }
                }
            }
        }
    }
}
