// Menu palette constants from WOLFSRC/WL_MENU.H, decoded against gamepal
// (GAMEPAL.OBJ). 6-bit VGA values scaled to 8-bit via (v << 2) | (v >> 4).
// Format is [R, G, B, A] to match `pixels`'s framebuffer layout.

#[cfg(feature = "spear")]
pub const BORDCOLOR:  [u8; 4] = [0x00, 0x00, 0x8a, 0xff]; // idx 0x99
#[cfg(feature = "spear")]
pub const BORD2COLOR: [u8; 4] = [0x00, 0x00, 0xd7, 0xff]; // idx 0x93
#[cfg(feature = "spear")]
pub const DEACTIVE:   [u8; 4] = [0x00, 0x00, 0x71, 0xff]; // idx 0x9b
#[cfg(feature = "spear")]
pub const BKGDCOLOR:  [u8; 4] = [0x00, 0x00, 0x59, 0xff]; // idx 0x9d
// STRIPE intentionally absent in SPEAR (commented out in WL_MENU.H)

#[cfg(not(feature = "spear"))]
pub const BORDCOLOR:  [u8; 4] = [0x8a, 0x00, 0x00, 0xff]; // idx 0x29
#[cfg(not(feature = "spear"))]
pub const BORD2COLOR: [u8; 4] = [0xd7, 0x00, 0x00, 0xff]; // idx 0x23
#[cfg(not(feature = "spear"))]
pub const DEACTIVE:   [u8; 4] = [0x71, 0x00, 0x00, 0xff]; // idx 0x2b
#[cfg(not(feature = "spear"))]
pub const BKGDCOLOR:  [u8; 4] = [0x59, 0x00, 0x00, 0xff]; // idx 0x2d
#[cfg(not(feature = "spear"))]
pub const STRIPE:     [u8; 4] = [0x65, 0x00, 0x00, 0xff]; // idx 0x2c

pub const READCOLOR:  [u8; 4] = [0xb6, 0xae, 0x00, 0xff]; // idx 0x4a
pub const READHCOLOR: [u8; 4] = [0xff, 0xf7, 0x00, 0xff]; // idx 0x47
pub const VIEWCOLOR:  [u8; 4] = [0x00, 0x41, 0x41, 0xff]; // idx 0x7f
pub const TEXTCOLOR:  [u8; 4] = [0x8e, 0x8e, 0x8e, 0xff]; // idx 0x17
pub const HIGHLIGHT:  [u8; 4] = [0xc3, 0xc3, 0xc3, 0xff]; // idx 0x13
