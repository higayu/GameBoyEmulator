use emulator_core::CartridgeHeader;
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

#[wasm_bindgen]
pub fn get_version() -> String {
    emulator_core::version().to_string()
}

#[wasm_bindgen]
pub fn inspect_rom(rom: &[u8]) -> Result<JsValue, JsValue> {
    let header = CartridgeHeader::parse(rom)
        .map_err(|error| JsValue::from_str(&error))?;

    let info = RomInfo {
        hardware: header.hardware_name().to_string(),
        cartridge_type: header.cartridge_name().to_string(),
        cartridge_type_code: header.cartridge_type,
        rom_size: header.rom_size,
        rom_size_code: header.rom_size_code,
        ram_size_code: header.ram_size_code,
        header_checksum_valid: header.header_checksum_valid,
        title: header.title,
    };

    serde_wasm_bindgen::to_value(&info)
        .map_err(|error| JsValue::from_str(&error.to_string()))
}