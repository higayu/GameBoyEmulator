use emulator_core::{CartridgeHeader, emulator::Emulator};
use serde::Serialize;
use wasm_bindgen::prelude::*;
#[derive(Serialize)]
struct RomInfo {
    title: String,
    hardware: String,
    cartridge_type: String,
    cartridge_type_code: u8,
    rom_size: usize,
    rom_size_code: u8,
    ram_size_code: u8,
    header_checksum_valid: bool,
}
fn err(e: String) -> JsValue {
    JsValue::from_str(&e)
}
#[wasm_bindgen]
pub fn get_version() -> String {
    emulator_core::version().into()
}
#[wasm_bindgen]
pub fn inspect_rom(rom: &[u8]) -> Result<JsValue, JsValue> {
    let h = CartridgeHeader::parse(rom).map_err(err)?;
    let info = RomInfo {
        hardware: h.hardware_name().into(),
        cartridge_type: h.cartridge_name().into(),
        cartridge_type_code: h.cartridge_type,
        rom_size: h.rom_size,
        rom_size_code: h.rom_size_code,
        ram_size_code: h.ram_size_code,
        header_checksum_valid: h.header_checksum_valid,
        title: h.title,
    };
    serde_wasm_bindgen::to_value(&info).map_err(|e| err(e.to_string()))
}
#[derive(Serialize)]
struct Snapshot {
    a: u8,
    f: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    pc: u16,
    sp: u16,
    cycles: f64,
    opcode: u8,
}
#[wasm_bindgen]
pub struct GbEmulator {
    inner: Emulator,
}
#[wasm_bindgen]
impl GbEmulator {
    #[wasm_bindgen(constructor)]
    pub fn new(rom: &[u8]) -> Result<GbEmulator, JsValue> {
        Ok(Self {
            inner: Emulator::new(rom.to_vec()).map_err(err)?,
        })
    }
    pub fn step(&mut self) -> Result<String, JsValue> {
        self.inner.step().map_err(err)
    }
    pub fn snapshot(&self) -> Result<JsValue, JsValue> {
        let r = self.inner.cpu.r;
        serde_wasm_bindgen::to_value(&Snapshot {
            a: r.a,
            f: r.f,
            b: r.b,
            c: r.c,
            d: r.d,
            e: r.e,
            h: r.h,
            l: r.l,
            pc: r.pc,
            sp: r.sp,
            cycles: self.inner.cpu.cycles as f64,
            opcode: self.inner.mmu.read(r.pc),
        })
        .map_err(|e| err(e.to_string()))
    }
}

#[wasm_bindgen]
impl GbEmulator {
    pub fn run_frame(&mut self) -> Result<(), JsValue> {
        let end = self.inner.cpu.cycles + self.inner.mmu.frame_cycles();
        while self.inner.cpu.cycles < end {
            self.inner.step().map_err(err)?;
        }
        Ok(())
    }
    pub fn framebuffer(&self) -> Vec<u8> {
        self.inner.mmu.pixels.clone()
    }
    pub fn set_keys(&mut self, keys: u8) {
        self.inner.mmu.set_keys(keys)
    }
}
