use super::Mmu;
impl Mmu {
    fn color(&self, p: usize, c: u8, sprite: bool) -> [u8; 4] {
        if self.cgb {
            let a = if sprite { &self.obj } else { &self.bg };
            let i = p * 8 + c as usize * 2;
            let v = u16::from_le_bytes([a[i], a[i + 1]]);
            let conv = |x: u16| ((x & 31) * 255 / 31) as u8;
            [conv(v), conv(v >> 5), conv(v >> 10), 255]
        } else {
            let r = self.mem[if sprite { 0xff48 + p } else { 0xff47 }];
            let v = 255 - 85 * ((r >> (c * 2)) & 3);
            [v, v, v, 255]
        }
    }
    fn tile_pixel(&self, t: usize, bank: usize, x: usize, y: usize) -> u8 {
        let a = bank * 8192 + t + y * 2;
        let b = 7 - x;
        ((self.vram[a] >> b) & 1) | (((self.vram[a + 1] >> b) & 1) << 1)
    }
    pub(super) fn render_line(&mut self) {
        let y = self.mem[0xff44] as usize;
        let lcd = self.mem[0xff40];
        let mut ids = [0u8; 160];
        let mut pri = [false; 160];
        for x in 0..160 {
            let win = lcd & 32 != 0
                && y >= self.mem[0xff4a] as usize
                && (x as i32) >= self.mem[0xff4b] as i32 - 7;
            let (sx, sy, map) = if win {
                (
                    (x as i32 + 7 - self.mem[0xff4b] as i32) as usize,
                    y - self.mem[0xff4a] as usize,
                    if lcd & 64 != 0 { 0x1c00 } else { 0x1800 },
                )
            } else {
                (
                    (x + self.mem[0xff43] as usize) & 255,
                    (y + self.mem[0xff42] as usize) & 255,
                    if lcd & 8 != 0 { 0x1c00 } else { 0x1800 },
                )
            };
            let i = map + (sy / 8 % 32) * 32 + sx / 8 % 32;
            let t = self.vram[i];
            let a = if self.cgb { self.vram[8192 + i] } else { 0 };
            let base = if lcd & 16 != 0 {
                t as usize * 16
            } else {
                (0x1000i32 + t as i8 as i32 * 16) as usize
            };
            let tx = if a & 32 != 0 { 7 - sx % 8 } else { sx % 8 };
            let ty = if a & 64 != 0 { 7 - sy % 8 } else { sy % 8 };
            let c = if !self.cgb && lcd & 1 == 0 {
                0
            } else {
                self.tile_pixel(base, ((a >> 3) & 1) as usize, tx, ty)
            };
            ids[x] = c;
            pri[x] = a & 128 != 0;
            let col = self.color((a & 7) as usize, c, false);
            let i = (y * 160 + x) * 4;
            self.pixels[i..i + 4].copy_from_slice(&col)
        }
        if lcd & 2 == 0 {
            return;
        }
        let h = if lcd & 4 != 0 { 16 } else { 8 };
        let mut sprites = Vec::new();
        for i in 0..40 {
            let o = 0xfe00 + i * 4;
            let sy = self.mem[o] as i32 - 16;
            if y as i32 >= sy && (y as i32) < sy + h {
                sprites.push(o);
                if sprites.len() == 10 {
                    break;
                }
            }
        }
        if !self.cgb {
            sprites.sort_by_key(|&o| self.mem[o + 1])
        }
        for x in 0..160 {
            for &o in &sprites {
                let sx = self.mem[o + 1] as i32 - 8;
                let dx = x as i32 - sx;
                if !(0..8).contains(&dx) {
                    continue;
                }
                let a = self.mem[o + 3];
                let mut ty = y as i32 - (self.mem[o] as i32 - 16);
                if a & 64 != 0 {
                    ty = h - 1 - ty
                }
                let tx = if a & 32 != 0 { 7 - dx } else { dx };
                let t = if h == 16 {
                    self.mem[o + 2] & 254
                } else {
                    self.mem[o + 2]
                };
                let c = self.tile_pixel(
                    t as usize * 16,
                    if self.cgb { ((a >> 3) & 1) as usize } else { 0 },
                    tx as usize,
                    ty as usize,
                );
                if c == 0 {
                    continue;
                }
                if ids[x] != 0 && (!self.cgb || lcd & 1 != 0) && (a & 128 != 0 || pri[x]) {
                    break;
                }
                let p = if self.cgb { a & 7 } else { (a >> 4) & 1 };
                let col = self.color(p as usize, c, true);
                let i = (y * 160 + x) * 4;
                self.pixels[i..i + 4].copy_from_slice(&col);
                break;
            }
        }
    }
}
