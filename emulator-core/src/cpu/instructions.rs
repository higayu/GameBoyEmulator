use super::Cpu;
use crate::mmu::Mmu;
impl Cpu {
    fn get(&self, m: &Mmu, i: u8) -> u8 {
        match i {
            0 => self.r.b,
            1 => self.r.c,
            2 => self.r.d,
            3 => self.r.e,
            4 => self.r.h,
            5 => self.r.l,
            6 => m.read(self.r.hl()),
            _ => self.r.a,
        }
    }
    fn put(&mut self, m: &mut Mmu, i: u8, v: u8) {
        match i {
            0 => self.r.b = v,
            1 => self.r.c = v,
            2 => self.r.d = v,
            3 => self.r.e = v,
            4 => self.r.h = v,
            5 => self.r.l = v,
            6 => m.write(self.r.hl(), v),
            _ => self.r.a = v,
        }
    }
    fn pair(&self, i: u8) -> u16 {
        match i {
            0 => u16::from_be_bytes([self.r.b, self.r.c]),
            1 => u16::from_be_bytes([self.r.d, self.r.e]),
            2 => self.r.hl(),
            _ => self.r.sp,
        }
    }
    fn set_pair(&mut self, i: u8, v: u16) {
        let [h, l] = v.to_be_bytes();
        match i {
            0 => {
                self.r.b = h;
                self.r.c = l
            }
            1 => {
                self.r.d = h;
                self.r.e = l
            }
            2 => self.r.set_hl(v),
            _ => self.r.sp = v,
        }
    }
    pub(super) fn push(&mut self, m: &mut Mmu, v: u16) {
        let [h, l] = v.to_be_bytes();
        self.r.sp = self.r.sp.wrapping_sub(1);
        m.write(self.r.sp, h);
        self.r.sp = self.r.sp.wrapping_sub(1);
        m.write(self.r.sp, l);
    }
    fn pop(&mut self, m: &Mmu) -> u16 {
        let l = m.read(self.r.sp);
        self.r.sp = self.r.sp.wrapping_add(1);
        let h = m.read(self.r.sp);
        self.r.sp = self.r.sp.wrapping_add(1);
        u16::from_le_bytes([l, h])
    }
    fn cond(&self, i: u8) -> bool {
        match i {
            0 => self.r.f & 128 == 0,
            1 => self.r.f & 128 != 0,
            2 => self.r.f & 16 == 0,
            _ => self.r.f & 16 != 0,
        }
    }
    fn flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.r.f = (z as u8) * 128 | (n as u8) * 64 | (h as u8) * 32 | (c as u8) * 16;
    }
    fn alu(&mut self, i: u8, v: u8) {
        let a = self.r.a;
        let c = if matches!(i, 1 | 3) && self.r.f & 16 != 0 {
            1
        } else {
            0
        };
        match i {
            0 | 1 => {
                let r = a.wrapping_add(v).wrapping_add(c);
                self.flags(
                    r == 0,
                    false,
                    (a & 15) + (v & 15) + c > 15,
                    a as u16 + v as u16 + c as u16 > 255,
                );
                self.r.a = r
            }
            2 | 3 | 7 => {
                let r = a.wrapping_sub(v).wrapping_sub(c);
                self.flags(
                    r == 0,
                    true,
                    (a & 15) < (v & 15) + c,
                    (a as u16) < v as u16 + c as u16,
                );
                if i != 7 {
                    self.r.a = r
                }
            }
            _ => {
                let r = match i {
                    4 => a & v,
                    5 => a ^ v,
                    _ => a | v,
                };
                self.flags(r == 0, false, i == 4, false);
                self.r.a = r
            }
        }
    }
    fn rotate(&mut self, i: u8, v: u8, zero: bool) -> u8 {
        let c = (self.r.f >> 4) & 1;
        let (r, k) = match i {
            0 => (v.rotate_left(1), v >> 7),
            1 => (v.rotate_right(1), v & 1),
            2 => ((v << 1) | c, v >> 7),
            3 => ((v >> 1) | (c << 7), v & 1),
            4 => (v << 1, v >> 7),
            5 => ((v >> 1) | (v & 128), v & 1),
            6 => (v.rotate_left(4), 0),
            _ => (v >> 1, v & 1),
        };
        self.flags(zero && r == 0, false, false, k != 0);
        r
    }
    fn cb(&mut self, m: &mut Mmu) -> (&'static str, u64) {
        let o = self.byte(m);
        let i = o & 7;
        let b = (o >> 3) & 7;
        let v = self.get(m, i);
        let group = o >> 6;
        match group {
            0 => {
                let r = self.rotate(b, v, true);
                self.put(m, i, r)
            }
            1 => self.r.f = (self.r.f & 16) | 32 | if v & (1 << b) == 0 { 128 } else { 0 },
            2 => self.put(m, i, v & !(1 << b)),
            _ => self.put(m, i, v | (1 << b)),
        };
        (
            ["CB ROT", "BIT", "RES", "SET"][group as usize],
            if i != 6 {
                8
            } else if group == 1 {
                12
            } else {
                16
            },
        )
    }
    pub(super) fn execute(&mut self, m: &mut Mmu, o: u8) -> Result<(&'static str, u64), String> {
        let x = o >> 6;
        let y = (o >> 3) & 7;
        let z = o & 7;
        let p = y >> 1;
        let q = y & 1;
        Ok(match x {
            0 => match z {
                0 => match y {
                    0 => ("NOP", 4),
                    1 => {
                        let a = self.word(m);
                        let [l, h] = self.r.sp.to_le_bytes();
                        m.write(a, l);
                        m.write(a.wrapping_add(1), h);
                        ("LD (a16),SP", 20)
                    }
                    2 => {
                        self.byte(m);
                        m.stop();
                        ("STOP", 4)
                    }
                    _ => {
                        let d = self.byte(m) as i8;
                        let take = y == 3 || self.cond(y - 4);
                        if take {
                            self.r.pc = self.r.pc.wrapping_add_signed(d as i16)
                        }
                        ("JR", if take { 12 } else { 8 })
                    }
                },
                1 => {
                    if q == 0 {
                        let v = self.word(m);
                        self.set_pair(p, v);
                        ("LD rr,d16", 12)
                    } else {
                        let a = self.r.hl();
                        let v = self.pair(p);
                        self.r.set_hl(a.wrapping_add(v));
                        self.r.f = (self.r.f & 128)
                            | if (a & 4095) + (v & 4095) > 4095 {
                                32
                            } else {
                                0
                            }
                            | if a as u32 + v as u32 > 65535 { 16 } else { 0 };
                        ("ADD HL,rr", 8)
                    }
                }
                2 => {
                    let a = if p < 2 { self.pair(p) } else { self.r.hl() };
                    if q == 0 {
                        m.write(a, self.r.a)
                    } else {
                        self.r.a = m.read(a)
                    }
                    if p == 2 {
                        self.r.set_hl(a.wrapping_add(1))
                    } else if p == 3 {
                        self.r.set_hl(a.wrapping_sub(1))
                    }
                    ("LD indirect", 8)
                }
                3 => {
                    let v = self.pair(p);
                    self.set_pair(
                        p,
                        if q == 0 {
                            v.wrapping_add(1)
                        } else {
                            v.wrapping_sub(1)
                        },
                    );
                    ("INC/DEC rr", 8)
                }
                4 | 5 => {
                    let v = self.get(m, y);
                    let n = z == 5;
                    let r = if n {
                        v.wrapping_sub(1)
                    } else {
                        v.wrapping_add(1)
                    };
                    self.flags(
                        r == 0,
                        n,
                        if n { v & 15 == 0 } else { v & 15 == 15 },
                        self.r.f & 16 != 0,
                    );
                    self.put(m, y, r);
                    (if n { "DEC" } else { "INC" }, if y == 6 { 12 } else { 4 })
                }
                6 => {
                    let v = self.byte(m);
                    self.put(m, y, v);
                    ("LD r,d8", if y == 6 { 12 } else { 8 })
                }
                _ => {
                    match y {
                        0..=3 => {
                            let a = self.r.a;
                            self.r.a = self.rotate(y, a, false)
                        }
                        4 => {
                            let mut a = self.r.a;
                            let n = self.r.f & 64 != 0;
                            let mut c = self.r.f & 16 != 0;
                            let mut d = 0;
                            if self.r.f & 32 != 0 || (!n && a & 15 > 9) {
                                d |= 6
                            }
                            if c || (!n && a > 0x99) {
                                d |= 0x60;
                                c = true
                            }
                            a = if n {
                                a.wrapping_sub(d)
                            } else {
                                a.wrapping_add(d)
                            };
                            self.r.a = a;
                            self.flags(a == 0, n, false, c)
                        }
                        5 => {
                            self.r.a = !self.r.a;
                            self.r.f |= 96
                        }
                        6 => self.r.f = (self.r.f & 128) | 16,
                        _ => self.r.f = (self.r.f & 128) | ((self.r.f ^ 16) & 16),
                    };
                    (
                        ["RLCA", "RRCA", "RLA", "RRA", "DAA", "CPL", "SCF", "CCF"][y as usize],
                        4,
                    )
                }
            },
            1 => {
                if o == 0x76 {
                    if !self.ime && m.pending() != 0 {
                        self.halt_bug = true
                    } else {
                        self.halted = true
                    }
                    ("HALT", 4)
                } else {
                    let v = self.get(m, z);
                    self.put(m, y, v);
                    ("LD r,r", if y == 6 || z == 6 { 8 } else { 4 })
                }
            }
            2 => {
                let v = self.get(m, z);
                self.alu(y, v);
                (
                    ["ADD", "ADC", "SUB", "SBC", "AND", "XOR", "OR", "CP"][y as usize],
                    if z == 6 { 8 } else { 4 },
                )
            }
            _ => match z {
                0 => match y {
                    0..=3 => {
                        let take = self.cond(y);
                        if take {
                            self.r.pc = self.pop(m)
                        }
                        ("RET cc", if take { 20 } else { 8 })
                    }
                    4 | 6 => {
                        let a = 0xff00 | self.byte(m) as u16;
                        if y == 4 {
                            m.write(a, self.r.a)
                        } else {
                            self.r.a = m.read(a)
                        }
                        ("LDH", 12)
                    }
                    _ => {
                        let d = self.byte(m);
                        let a = self.r.sp;
                        let r = a.wrapping_add_signed(d as i8 as i16);
                        self.flags(
                            false,
                            false,
                            (a & 15) + (d as u16 & 15) > 15,
                            (a & 255) + d as u16 > 255,
                        );
                        if y == 5 {
                            self.r.sp = r
                        } else {
                            self.r.set_hl(r)
                        }
                        ("ADD SP / LD HL", if y == 5 { 16 } else { 12 })
                    }
                },
                1 => {
                    if q == 0 {
                        let v = self.pop(m);
                        if p == 3 {
                            let [a, f] = v.to_be_bytes();
                            self.r.a = a;
                            self.r.f = f & 240
                        } else {
                            self.set_pair(p, v)
                        }
                        ("POP", 12)
                    } else {
                        match p {
                            0 | 1 => {
                                self.r.pc = self.pop(m);
                                if p == 1 {
                                    self.ime = true;
                                    self.ime_delay = 0
                                }
                                ("RET/RETI", 16)
                            }
                            2 => {
                                self.r.pc = self.r.hl();
                                ("JP HL", 4)
                            }
                            _ => {
                                self.r.sp = self.r.hl();
                                ("LD SP,HL", 8)
                            }
                        }
                    }
                }
                2 => match y {
                    0..=3 => {
                        let a = self.word(m);
                        let take = self.cond(y);
                        if take {
                            self.r.pc = a
                        }
                        ("JP cc", if take { 16 } else { 12 })
                    }
                    4 | 6 => {
                        let a = 0xff00 | self.r.c as u16;
                        if y == 4 {
                            m.write(a, self.r.a)
                        } else {
                            self.r.a = m.read(a)
                        }
                        ("LDH C", 8)
                    }
                    _ => {
                        let a = self.word(m);
                        if y == 5 {
                            m.write(a, self.r.a)
                        } else {
                            self.r.a = m.read(a)
                        }
                        ("LD a16", 16)
                    }
                },
                3 => match y {
                    0 => {
                        self.r.pc = self.word(m);
                        ("JP a16", 16)
                    }
                    1 => self.cb(m),
                    6 => {
                        self.ime = false;
                        self.ime_delay = 0;
                        ("DI", 4)
                    }
                    7 => {
                        if self.ime_delay == 0 {
                            self.ime_delay = 2
                        }
                        ("EI", 4)
                    }
                    _ => return Err(format!("Illegal opcode 0x{o:02X}")),
                },
                4 => {
                    if y > 3 {
                        return Err(format!("Illegal opcode 0x{o:02X}"));
                    }
                    let a = self.word(m);
                    let take = self.cond(y);
                    if take {
                        self.push(m, self.r.pc);
                        self.r.pc = a
                    }
                    ("CALL cc", if take { 24 } else { 12 })
                }
                5 => {
                    if q == 0 {
                        let v = if p == 3 {
                            u16::from_be_bytes([self.r.a, self.r.f])
                        } else {
                            self.pair(p)
                        };
                        self.push(m, v);
                        ("PUSH", 16)
                    } else if p == 0 {
                        let a = self.word(m);
                        self.push(m, self.r.pc);
                        self.r.pc = a;
                        ("CALL a16", 24)
                    } else {
                        return Err(format!("Illegal opcode 0x{o:02X}"));
                    }
                }
                6 => {
                    let v = self.byte(m);
                    self.alu(y, v);
                    ("ALU d8", 8)
                }
                _ => {
                    self.push(m, self.r.pc);
                    self.r.pc = y as u16 * 8;
                    ("RST", 16)
                }
            },
        })
    }
}
