pub mod cartridge;

pub use cartridge::CartridgeHeader;

pub fn version() -> &'static str {
    "0.2.0"
}
pub mod cpu;
pub mod emulator;
pub mod mmu;
