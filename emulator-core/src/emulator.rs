use crate::{cpu::Cpu, mmu::Mmu};
pub struct Emulator {
    pub cpu: Cpu,
    pub mmu: Mmu,
}
impl Emulator {
    pub fn new(rom: Vec<u8>) -> Result<Self, String> {
        let mmu = Mmu::new(rom)?;
        let mut cpu = Cpu::new();
        if mmu.cgb {
            cpu.r.a = 0x11;
            cpu.r.f = 0x80;
            cpu.r.b = 0;
            cpu.r.c = 0;
            cpu.r.d = 0xff;
            cpu.r.e = 0x56;
            cpu.r.set_hl(0x000d);
        }
        Ok(Self { cpu, mmu })
    }
    pub fn step(&mut self) -> Result<String, String> {
        self.cpu.step(&mut self.mmu)
    }
}
