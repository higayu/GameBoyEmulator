pub mod cartridge;

pub use cartridge::CartridgeHeader;

pub fn version() -> &'static str {
    "0.1.0"
}