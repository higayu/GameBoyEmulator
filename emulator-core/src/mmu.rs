mod video;
pub struct Mmu {
    rom: Vec<u8>,
    mem: Vec<u8>,
    kind: u8,
    bank: u8,
    high: u8,
    mode: bool,
    enabled: bool,
    ram: Vec<u8>,
    rtc: [u8; 5],
    latched: [u8; 5],
    latch: u8,
    rtc_ticks: u32,
    pub cgb: bool,
    vram: Vec<u8>,
    wram: Vec<u8>,
    bg: [u8; 64],
    obj: [u8; 64],
    pub pixels: Vec<u8>,
    dot: u32,
    div: u16,
    reload: u8,
    keys: u8,
    stat_line: bool,
    speed: bool,
    hdma: u8,
}
impl Mmu {
    pub fn new(rom: Vec<u8>) -> Result<Self, String> {
        let h = crate::CartridgeHeader::parse(&rom)?;
        if !matches!(h.cartridge_type,0..=3|0x0f..=0x13) {
            return Err("Unsupported cartridge".into());
        }
        let mut mem = vec![0; 65536];
        mem[0xff40] = 0x91;
        mem[0xff47] = 0xfc;
        mem[0xff48] = 0xff;
        mem[0xff49] = 0xff;
        mem[0xff00] = 0x30;
        mem[0xff70] = 1;
        let ram_size = match h.ram_size_code {
            1 => 2048,
            2 => 8192,
            3 => 32768,
            4 => 131072,
            5 => 65536,
            _ => 0,
        };
        Ok(Self {
            rom,
            mem,
            kind: h.cartridge_type,
            bank: 1,
            high: 0,
            mode: false,
            enabled: false,
            ram: vec![0; ram_size],
            rtc: [0; 5],
            latched: [0; 5],
            latch: 0,
            rtc_ticks: 0,
            cgb: h.cgb_flag & 128 != 0,
            vram: vec![0; 0x4000],
            wram: vec![0; 0x8000],
            bg: [255; 64],
            obj: [255; 64],
            pixels: vec![255; 160 * 144 * 4],
            dot: 0,
            div: 0,
            reload: 0,
            keys: 0,
            stat_line: false,
            speed: false,
            hdma: 255,
        })
    }
    fn mbc1(&self) -> bool {
        matches!(self.kind, 1..=3)
    }
    fn mbc3(&self) -> bool {
        matches!(self.kind, 0x0f..=0x13)
    }
    fn rom_at(&self, b: usize, a: usize) -> u8 {
        self.rom
            .get((b % ((self.rom.len() / 0x4000).max(1))) * 0x4000 + a)
            .copied()
            .unwrap_or(255)
    }
    fn vb(&self) -> usize {
        if self.cgb {
            (self.mem[0xff4f] & 1) as usize
        } else {
            0
        }
    }
    fn wb(&self) -> usize {
        if self.cgb {
            (self.mem[0xff70] & 7).max(1) as usize
        } else {
            1
        }
    }
    fn ram_at(&self, a: usize) -> Option<usize> {
        let b = if self.mbc1() {
            if self.mode { self.high & 3 } else { 0 }
        } else {
            self.high
        };
        if self.enabled && b < 4 && !self.ram.is_empty() {
            Some((b as usize * 8192 + a - 0xa000) % self.ram.len())
        } else {
            None
        }
    }
    fn joy(&self) -> u8 {
        let s = self.mem[0xff00] & 48;
        let mut v = 15;
        if s & 16 == 0 {
            v &= !(self.keys & 15)
        }
        if s & 32 == 0 {
            v &= !(self.keys >> 4)
        }
        0xc0 | s | v
    }
    pub fn set_keys(&mut self, k: u8) {
        let old = self.joy();
        self.keys = k;
        if old & !self.joy() & 15 != 0 {
            self.mem[0xff0f] |= 16
        }
    }
    pub fn pending(&self) -> u8 {
        self.mem[0xffff] & self.mem[0xff0f] & 31
    }
    pub fn ack(&mut self, b: u8) {
        self.mem[0xff0f] &= !(1 << b)
    }
    pub fn read(&self, a: u16) -> u8 {
        let a = a as usize;
        match a {
            0..=0x3fff => self.rom_at(
                if self.mbc1() && self.mode {
                    self.high as usize * 32
                } else {
                    0
                },
                a,
            ),
            0x4000..=0x7fff => self.rom_at(
                if self.mbc1() {
                    self.high as usize * 32 + self.bank as usize
                } else {
                    self.bank as usize
                },
                a - 0x4000,
            ),
            0x8000..=0x9fff => self.vram[self.vb() * 8192 + a - 0x8000],
            0xa000..=0xbfff => {
                if self.mbc3() && self.enabled && (8..=12).contains(&self.high) {
                    self.latched[(self.high - 8) as usize]
                } else {
                    self.ram_at(a).map(|i| self.ram[i]).unwrap_or(255)
                }
            }
            0xc000..=0xcfff => self.wram[a - 0xc000],
            0xd000..=0xdfff => self.wram[self.wb() * 4096 + a - 0xd000],
            0xe000..=0xfdff => self.read((a - 0x2000) as u16),
            0xfea0..=0xfeff => 255,
            0xff00 => self.joy(),
            0xff04 => (self.div >> 8) as u8,
            0xff07 => self.mem[a] | 0xf8,
            0xff0f => self.mem[a] | 0xe0,
            0xff41 => self.mem[a] | 128,
            0xff4d => {
                if self.cgb {
                    0x7e | self.mem[a] & 1 | if self.speed { 128 } else { 0 }
                } else {
                    255
                }
            }
            0xff4f => 0xfe | self.vb() as u8,
            0xff55 => self.hdma,
            0xff69 => self.bg[(self.mem[0xff68] & 63) as usize],
            0xff6b => self.obj[(self.mem[0xff6a] & 63) as usize],
            0xff70 => 0xf8 | self.wb() as u8,
            _ => self.mem[a],
        }
    }
    pub fn write(&mut self, a: u16, v: u8) {
        let a = a as usize;
        match a {
            0..=0x1fff => self.enabled = v & 15 == 10,
            0x2000..=0x3fff => {
                if self.mbc1() {
                    self.bank = (v & 31).max(1)
                } else if self.mbc3() {
                    self.bank = (v & 127).max(1)
                }
            }
            0x4000..=0x5fff => {
                if self.mbc1() {
                    self.high = v & 3
                } else if self.mbc3() {
                    self.high = v
                }
            }
            0x6000..=0x7fff => {
                if self.mbc1() {
                    self.mode = v & 1 != 0
                } else if self.mbc3() {
                    if self.latch == 0 && v == 1 {
                        self.latched = self.rtc
                    }
                    self.latch = v
                }
            }
            0x8000..=0x9fff => {
                let i = self.vb() * 8192 + a - 0x8000;
                self.vram[i] = v
            }
            0xa000..=0xbfff => {
                if self.mbc3() && self.enabled && (8..=12).contains(&self.high) {
                    self.rtc[(self.high - 8) as usize] = match self.high {
                        8 | 9 => v % 60,
                        10 => v % 24,
                        12 => v & 0xc1,
                        _ => v,
                    }
                } else if let Some(i) = self.ram_at(a) {
                    self.ram[i] = v
                }
            }
            0xc000..=0xcfff => self.wram[a - 0xc000] = v,
            0xd000..=0xdfff => {
                let i = self.wb() * 4096 + a - 0xd000;
                self.wram[i] = v
            }
            0xe000..=0xfdff => self.write((a - 0x2000) as u16, v),
            0xfea0..=0xfeff => {}
            0xff00 => {
                let old = self.joy();
                self.mem[a] = v & 48;
                if old & !self.joy() & 15 != 0 {
                    self.mem[0xff0f] |= 16
                }
            }
            0xff04 | 0xff07 => {
                let old = self.timer_bit();
                if a == 0xff04 {
                    self.div = 0
                } else {
                    self.mem[a] = v & 7
                }
                if old && !self.timer_bit() {
                    self.timer_inc()
                }
            }
            0xff05 => {
                self.mem[a] = v;
                self.reload = 0
            }
            0xff40 => {
                self.mem[a] = v;
                if v & 128 == 0 {
                    self.dot = 0;
                    self.mem[0xff44] = 0;
                    self.pixels.fill(255)
                }
                self.stat()
            }
            0xff41 => {
                self.mem[a] = (self.mem[a] & 7) | (v & 0x78);
                self.stat()
            }
            0xff44 => {}
            0xff45 => {
                self.mem[a] = v;
                self.stat()
            }
            0xff46 => {
                self.mem[a] = v;
                for i in 0..160 {
                    let b = self.read((v as u16) << 8 | i as u16);
                    self.mem[0xfe00 + i] = b
                }
            }
            0xff55 => {
                if self.hdma & 128 == 0 && v & 128 == 0 {
                    self.hdma |= 128
                } else {
                    self.hdma = v & 127;
                    if v & 128 == 0 {
                        while self.hdma != 255 {
                            self.dma_block()
                        }
                    }
                }
            }
            0xff69 | 0xff6b => {
                let idx = a - 1;
                let i = (self.mem[idx] & 63) as usize;
                if a == 0xff69 {
                    self.bg[i] = v
                } else {
                    self.obj[i] = v
                }
                if self.mem[idx] & 128 != 0 {
                    self.mem[idx] = 128 | ((i as u8 + 1) & 63)
                }
            }
            0xff02 => {
                self.mem[a] = v;
                if v & 0x81 == 0x81 {
                    self.mem[a] &= 127;
                    self.mem[0xff0f] |= 8;
                    self.mem[0xff01] = 255
                }
            }
            _ => self.mem[a] = v,
        }
    }
    fn dma_block(&mut self) {
        let s = ((self.mem[0xff51] as u16) << 8) | (self.mem[0xff52] & 240) as u16;
        let d = 0x8000 | ((self.mem[0xff53] & 31) as u16) << 8 | (self.mem[0xff54] & 240) as u16;
        for i in 0..16 {
            let v = self.read(s.wrapping_add(i));
            self.write(d + i, v)
        }
        let s = s.wrapping_add(16);
        let d = d.wrapping_add(16);
        self.mem[0xff51] = (s >> 8) as u8;
        self.mem[0xff52] = s as u8;
        self.mem[0xff53] = (d >> 8) as u8 & 31;
        self.mem[0xff54] = d as u8;
        self.hdma = if self.hdma == 0 { 255 } else { self.hdma - 1 };
    }
    pub fn stop(&mut self) {
        if self.cgb && self.mem[0xff4d] & 1 != 0 {
            self.speed = !self.speed;
            self.mem[0xff4d] = 0;
            self.div = 0
        }
    }
    fn timer_bit(&self) -> bool {
        let t = self.mem[0xff07];
        t & 4 != 0 && self.div & (1 << [9, 3, 5, 7][(t & 3) as usize]) != 0
    }
    fn timer_inc(&mut self) {
        let (v, o) = self.mem[0xff05].overflowing_add(1);
        self.mem[0xff05] = v;
        if o {
            self.reload = 4
        }
    }
    pub fn tick(&mut self, t: u32) {
        for _ in 0..t {
            if self.reload > 0 {
                self.reload -= 1;
                if self.reload == 0 {
                    self.mem[0xff05] = self.mem[0xff06];
                    self.mem[0xff0f] |= 4
                }
            }
            let old = self.timer_bit();
            self.div = self.div.wrapping_add(1);
            if old && !self.timer_bit() {
                self.timer_inc()
            }
        }
        let dots = if self.speed { t / 2 } else { t };
        if self.rtc[4] & 64 == 0 {
            self.rtc_ticks += dots;
            if self.rtc_ticks >= 4194304 {
                self.rtc_ticks -= 4194304;
                self.rtc[0] += 1;
                if self.rtc[0] >= 60 {
                    self.rtc[0] = 0;
                    self.rtc[1] += 1;
                    if self.rtc[1] >= 60 {
                        self.rtc[1] = 0;
                        self.rtc[2] += 1;
                        if self.rtc[2] >= 24 {
                            self.rtc[2] = 0;
                            let day = self.rtc[3] as u16 + ((self.rtc[4] & 1) as u16) * 256 + 1;
                            self.rtc[3] = day as u8;
                            self.rtc[4] = (self.rtc[4] & 0xc0)
                                | ((day >> 8) as u8 & 1)
                                | if day > 511 { 128 } else { 0 }
                        }
                    }
                }
            }
        }
        if self.mem[0xff40] & 128 == 0 {
            return;
        }
        for _ in 0..dots {
            self.dot += 1;
            if self.dot == 252 && self.mem[0xff44] < 144 {
                self.render_line();
                if self.hdma & 128 == 0 {
                    self.dma_block()
                }
            }
            if self.dot == 456 {
                self.dot = 0;
                self.mem[0xff44] += 1;
                if self.mem[0xff44] == 144 {
                    self.mem[0xff0f] |= 1
                }
                if self.mem[0xff44] == 154 {
                    self.mem[0xff44] = 0
                }
            }
            if matches!(self.dot, 0 | 80 | 252) {
                self.stat()
            }
        }
    }
    fn stat(&mut self) {
        let ly = self.mem[0xff44];
        let mode = if self.mem[0xff40] & 128 == 0 {
            0
        } else if ly >= 144 {
            1
        } else if self.dot < 80 {
            2
        } else if self.dot < 252 {
            3
        } else {
            0
        };
        let eq = ly == self.mem[0xff45];
        let s = (self.mem[0xff41] & 0x78) | mode | if eq { 4 } else { 0 };
        self.mem[0xff41] = s;
        let line = self.mem[0xff40] & 128 != 0
            && ((eq && s & 64 != 0)
                || (mode == 0 && s & 8 != 0)
                || (mode == 1 && s & 16 != 0)
                || (mode == 2 && s & 32 != 0));
        if line && !self.stat_line {
            self.mem[0xff0f] |= 2
        }
        self.stat_line = line;
    }
    pub fn frame_cycles(&self) -> u64 {
        if self.speed { 140448 } else { 70224 }
    }
}
