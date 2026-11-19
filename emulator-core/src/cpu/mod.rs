mod instructions;
pub mod registers;
use crate::mmu::Mmu;
use registers::Registers;
pub struct Cpu {
    pub r: Registers,
    pub cycles: u64,
    pub ime: bool,
    ime_delay: u8,
    halted: bool,
    halt_bug: bool,
}
impl Cpu {
    pub fn new() -> Self {
        Self {
            r: Registers::default(),
            cycles: 0,
            ime: false,
            ime_delay: 0,
            halted: false,
            halt_bug: false,
        }
    }
    pub fn step(&mut self, m: &mut Mmu) -> Result<String, String> {
        let pending = m.pending();
        if pending != 0 {
            self.halted = false;
            if self.ime {
                let bit = pending.trailing_zeros() as u8;
                self.ime = false;
                self.ime_delay = 0;
                m.ack(bit);
                self.push(m, self.r.pc);
                self.r.pc = 0x40 + 8 * bit as u16;
                self.cycles += 20;
                m.tick(20);
                return Ok("IRQ".into());
            }
        }
        if self.halted {
            self.cycles += 4;
            m.tick(4);
            return Ok("HALT idle".into());
        }
        let pc = self.r.pc;
        let op = m.read(pc);
        if self.halt_bug {
            self.halt_bug = false
        } else {
            self.r.pc = pc.wrapping_add(1)
        }
        match self.execute(m, op) {
            Ok((name, t)) => {
                self.cycles += t;
                m.tick(t as u32);
                if self.ime_delay > 0 {
                    self.ime_delay -= 1;
                    if self.ime_delay == 0 {
                        self.ime = true
                    }
                }
                Ok(format!("{pc:04X}: {op:02X} {name}"))
            }
            Err(e) => {
                self.r.pc = pc;
                Err(format!("{pc:04X}: {e}"))
            }
        }
    }
    fn byte(&mut self, m: &Mmu) -> u8 {
        let v = m.read(self.r.pc);
        self.r.pc = self.r.pc.wrapping_add(1);
        v
    }
    fn word(&mut self, m: &Mmu) -> u16 {
        let l = self.byte(m);
        let h = self.byte(m);
        u16::from_le_bytes([l, h])
    }
}
