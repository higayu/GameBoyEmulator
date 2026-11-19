#[derive(Debug, Clone)]
pub struct CartridgeHeader {
    pub title: String,
    pub cgb_flag: u8,
    pub cartridge_type: u8,
    pub rom_size_code: u8,
    pub ram_size_code: u8,
    pub rom_size: usize,
    pub header_checksum_valid: bool,
}

impl CartridgeHeader {
    pub fn parse(rom: &[u8]) -> Result<Self, String> {
        if rom.len() < 0x150 {
            return Err("ROMファイルが小さすぎます".to_string());
        }

        let cgb_flag = rom[0x0143];

        let title_end = match cgb_flag {
            0x80 | 0xC0 => 0x0143,
            _ => 0x0144,
        };

        let title_bytes = &rom[0x0134..title_end];

        let title: String = title_bytes
            .iter()
            .take_while(|&&b| b != 0)
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '?'
                }
            })
            .collect();

        let mut checksum: u8 = 0;

        for &byte in &rom[0x0134..=0x014C] {
            checksum = checksum.wrapping_sub(byte).wrapping_sub(1);
        }

        Ok(Self {
            title: title.trim().to_string(),
            cgb_flag,
            cartridge_type: rom[0x0147],
            rom_size_code: rom[0x0148],
            ram_size_code: rom[0x0149],
            rom_size: rom.len(),
            header_checksum_valid: checksum == rom[0x014D],
        })
    }

    pub fn hardware_name(&self) -> &'static str {
        match self.cgb_flag {
            0x80 => "Game Boy / Game Boy Color",
            0xC0 => "Game Boy Color",
            _ => "Game Boy",
        }
    }

    pub fn cartridge_name(&self) -> &'static str {
        match self.cartridge_type {
            0x00 => "ROM ONLY",
            0x01 => "MBC1",
            0x02 => "MBC1 + RAM",
            0x03 => "MBC1 + RAM + BATTERY",
            0x05 => "MBC2",
            0x06 => "MBC2 + BATTERY",
            0x0F => "MBC3 + TIMER + BATTERY",
            0x10 => "MBC3 + TIMER + RAM + BATTERY",
            0x11 => "MBC3",
            0x12 => "MBC3 + RAM",
            0x13 => "MBC3 + RAM + BATTERY",
            0x19 => "MBC5",
            0x1A => "MBC5 + RAM",
            0x1B => "MBC5 + RAM + BATTERY",
            _ => "Unknown / Other",
        }
    }
}
