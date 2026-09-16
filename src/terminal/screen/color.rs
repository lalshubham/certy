pub const ANSI_COLORS: [u32; 16] = [
    0xFF6E7681, 0xFFFF4D4D, 0xFF2EE59D, 0xFFFFDD00, 0xFF4C9EFF, 0xFFFF55D4, 0xFF00E5FF, 0xFFE6EDF3,
    0xFF8B949E, 0xFFFF7B72, 0xFF56F39A, 0xFFFFF066, 0xFF79C0FF, 0xFFFFA8EC, 0xFF56FFFF, 0xFFFFFFFF,
];

pub const COLOR_TERMINAL_FG: u32 = ANSI_COLORS[15];

pub fn ansi_256_to_u32(idx: u8) -> u32 {
    if (idx as usize) < 16 {
        return ANSI_COLORS[idx as usize];
    }
    if idx >= 232 {
        let v = 8 + (idx - 232) * 10;
        return 0xFF000000 | ((v as u32) << 16) | ((v as u32) << 8) | (v as u32);
    }
    let idx = idx - 16;
    let b = (idx % 6) * 51;
    let g = ((idx / 6) % 6) * 51;
    let r = (idx / 36) * 51;
    0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}
