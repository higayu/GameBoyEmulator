#[derive(Debug, Clone, Copy)]
pub struct Registers {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub pc: u16,
    pub sp: u16,
}
impl Default for Registers {
    fn default() -> Self {
        Self {
            a: 1,
            f: 0xb0,
            b: 0,
            c: 0x13,
            d: 0,
            e: 0xd8,
            h: 1,
            l: 0x4d,
            pc: 0x100,
            sp: 0xfffe,
        }
    }
}
impl Registers {
    pub fn hl(&self) -> u16 {
        u16::from_be_bytes([self.h, self.l])
    }
    pub fn set_hl(&mut self, v: u16) {
        [self.h, self.l] = v.to_be_bytes();
    }
}
