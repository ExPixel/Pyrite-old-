// LCD I/O
pub const DISPCNT: u16      = 0x0000;
pub const GREENSWAP: u16    = 0x0002;
pub const DISPSTAT: u16     = 0x0004;
pub const VCOUNT: u16       = 0x0006;
pub const BG0CNT: u16       = 0x0008;
pub const BG1CNT: u16       = 0x000A;
pub const BG2CNT: u16       = 0x000C;
pub const BG3CNT: u16       = 0x000E;
pub const BG0HOFS: u16      = 0x0010;
pub const BG0VOFS: u16      = 0x0012;
pub const BG1HOFS: u16      = 0x0014;
pub const BG1VOFS: u16      = 0x0016;
pub const BG2HOFS: u16      = 0x0018;
pub const BG2VOFS: u16      = 0x001A;
pub const BG3HOFS: u16      = 0x001C;
pub const BG3VOFS: u16      = 0x001E;
pub const BG2PA: u16        = 0x0020;
pub const BG2PB: u16        = 0x0022;
pub const BG2PC: u16        = 0x0024;
pub const BG2PD: u16        = 0x0026;
pub const BG2X: u16         = 0x0028;
pub const BG2X_HI: u16      = 0x002A;
pub const BG2Y: u16         = 0x002C;
pub const BG2Y_HI: u16      = 0x002E;
pub const BG3PA: u16        = 0x0030;
pub const BG3PB: u16        = 0x0032;
pub const BG3PC: u16        = 0x0034;
pub const BG3PD: u16        = 0x0036;
pub const BG3X: u16         = 0x0038;
pub const BG3X_HI: u16      = 0x003A;
pub const BG3Y: u16         = 0x003C;
pub const BG3Y_HI: u16      = 0x003E;
pub const WIN0H: u16        = 0x0040;
pub const WIN1H: u16        = 0x0042;
pub const WIN0V: u16        = 0x0044;
pub const WIN1V: u16        = 0x0046;
pub const WININ: u16        = 0x0048;
pub const WINOUT: u16       = 0x004A;
pub const MOSAIC: u16       = 0x004C;
pub const BLDCNT: u16       = 0x0050;
pub const BLDALPHA: u16     = 0x0052;
pub const BLDY: u16         = 0x0054;

// Sound Registers
pub const SOUND1CNT_L: u16  = 0x0060; 
pub const SOUND1CNT_H: u16  = 0x0062; 
pub const SOUND1CNT_X: u16  = 0x0064; 
pub const SOUND2CNT_L: u16  = 0x0068; 
pub const SOUND2CNT_H: u16  = 0x006C; 
pub const SOUND3CNT_L: u16  = 0x0070; 
pub const SOUND3CNT_H: u16  = 0x0072; 
pub const SOUND3CNT_X: u16  = 0x0074; 
pub const SOUND4CNT_L: u16  = 0x0078; 
pub const SOUND4CNT_H: u16  = 0x007C; 
pub const SOUNDCNT_L: u16   = 0x0080; 
pub const SOUNDCNT_H: u16   = 0x0082; 
pub const SOUNDCNT_X: u16   = 0x0084; 
pub const SOUNDBIAS: u16    = 0x0088; 
pub const FIFO_A: u16       = 0x00A0; 
pub const FIFO_B: u16       = 0x00A4; 

// DMA Transfer Channels
pub const DMA0SAD: u16      = 0x00B0;
pub const DMA0DAD: u16      = 0x00B4;
pub const DMA0CNT_L: u16    = 0x00B8;
pub const DMA0CNT_H: u16    = 0x00BA;
pub const DMA1SAD: u16      = 0x00BC;
pub const DMA1DAD: u16      = 0x00C0;
pub const DMA1CNT_L: u16    = 0x00C4;
pub const DMA1CNT_H: u16    = 0x00C6;
pub const DMA2SAD: u16      = 0x00C8;
pub const DMA2DAD: u16      = 0x00CC;
pub const DMA2CNT_L: u16    = 0x00D0;
pub const DMA2CNT_H: u16    = 0x00D2;
pub const DMA3SAD: u16      = 0x00D4;
pub const DMA3DAD: u16      = 0x00D8;
pub const DMA3CNT_L: u16    = 0x00DC;
pub const DMA3CNT_H: u16    = 0x00DE;

// Timer Registers
pub const TM0CNT_L : u16    = 0x0100; 
pub const TM0CNT_H : u16    = 0x0102; 
pub const TM1CNT_L : u16    = 0x0104; 
pub const TM1CNT_H : u16    = 0x0106; 
pub const TM2CNT_L : u16    = 0x0108; 
pub const TM2CNT_H : u16    = 0x010A; 
pub const TM3CNT_L : u16    = 0x010C; 
pub const TM3CNT_H : u16    = 0x010E; 

// Serial Communication (1)_
pub const SIODATA32: u16    = 0x0120;
pub const SIOMULTI0: u16    = 0x0120;
pub const SIOMULTI1: u16    = 0x0122;
pub const SIOMULTI2: u16    = 0x0124;
pub const SIOMULTI3: u16    = 0x0126;
pub const SIOCNT: u16       = 0x0128;
pub const SIOMLT_SEND: u16  = 0x012A;
pub const SIODATA8: u16     = 0x012A;

// Keypad Input
pub const KEYINPUT: u16     = 0x0130;
pub const KEYCNT: u16       = 0x0132;

// Serial Communication (2)
pub const RCNT: u16         = 0x0134;
pub const IR: u16           = 0x0136;
pub const JOYCNT: u16       = 0x0140;
pub const JOY_RECV: u16     = 0x0150;
pub const JOY_TRANS: u16    = 0x0154;
pub const JOYSTAT: u16      = 0x0158;

// Interrupt, Waitstate, and Power-Down Control
pub const IE: u16           = 0x0200;
pub const IF: u16           = 0x0202;
pub const WAITCNT: u16      = 0x0204;
pub const IME: u16          = 0x0208;
pub const POSTFLG: u16      = 0x0300;
pub const HALTCNT: u16      = 0x0301;
pub const BUG410: u16       = 0x0410;
pub const IMC: u16          = 0x0800;
pub const IMC_HI: u16       = 0x0802;
