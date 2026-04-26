//! Asset extraction tool.
//!
//! Reads the original Wolfenstein 3D data files from `assets/data/` and
//! dumps everything into `assets/extracted/` in modern formats (PNG, WAV,
//! JSON).  Files are named by the chunk indices and identifiers used in
//! the original C source, so runtime code can look them up either by
//! number or by symbolic name (e.g. `003_H_BJPIC.png`).
//!
//! Run with:  cargo run --bin extract --release

#![allow(dead_code)]

use std::fs;
use std::io::{Cursor, ErrorKind, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use byteorder::{LE, ReadBytesExt};

// -----------------------------------------------------------------------------
// Paths
// -----------------------------------------------------------------------------

const DATA_DIR: &str = "assets/data";
const PALETTE_PATH: &str = "assets/GAMEPAL.OBJ";
const OUT_DIR: &str = "assets/extracted";

// -----------------------------------------------------------------------------
// VGAGRAPH layout (GFXV_WL6.H)
// -----------------------------------------------------------------------------

const STRUCTPIC: usize = 0;
const STARTFONT: usize = 1;
const NUMFONT: usize = 2;
const STARTPICS: usize = 3;
const NUMPICS: usize = 132;
const STARTTILE8: usize = 135;
const NUMTILE8: usize = 72;

// -----------------------------------------------------------------------------
// GAMEMAPS layout
// -----------------------------------------------------------------------------

const NUMMAPS: usize = 60;
const MAPPLANES: usize = 2;
const CARMACK_NEAR_TAG: u8 = 0xA7;
const CARMACK_FAR_TAG: u8 = 0xA8;

// -----------------------------------------------------------------------------
// AUDIOT layout (AUDIOWL6.H)
// -----------------------------------------------------------------------------

const STARTPCSOUNDS: usize = 0;
const STARTADLIBSOUNDS: usize = 87;
const STARTDIGISOUNDS: usize = 174;
const STARTMUSIC: usize = 261;
const NUMSNDCHUNKS: usize = 288;
const NUMSOUNDS: usize = 87;

// -----------------------------------------------------------------------------
// VSWAP / page manager
// -----------------------------------------------------------------------------

const PM_PAGE_SIZE: usize = 4096;

// Digitized sound playback rate (ID_SD.C used 7000 Hz on Sound Blaster).
const DIGI_SAMPLE_RATE: u32 = 7000;

// -----------------------------------------------------------------------------
// Naming tables (lifted from GFXV_WL6.H, AUDIOWL6.H, WL_MAIN.C)
// -----------------------------------------------------------------------------

/// Symbolic names for VGAGRAPH pic chunks 3..=134, in order.
#[rustfmt::skip]
const PIC_NAMES: &[&str] = &[
    "H_BJPIC", "H_CASTLEPIC", "H_BLAZEPIC", "H_TOPWINDOWPIC", "H_LEFTWINDOWPIC",
    "H_RIGHTWINDOWPIC", "H_BOTTOMINFOPIC",
    "C_OPTIONSPIC", "C_CURSOR1PIC", "C_CURSOR2PIC", "C_NOTSELECTEDPIC",
    "C_SELECTEDPIC", "C_FXTITLEPIC", "C_DIGITITLEPIC", "C_MUSICTITLEPIC",
    "C_MOUSELBACKPIC", "C_BABYMODEPIC", "C_EASYPIC", "C_NORMALPIC", "C_HARDPIC",
    "C_LOADSAVEDISKPIC", "C_DISKLOADING1PIC", "C_DISKLOADING2PIC",
    "C_CONTROLPIC", "C_CUSTOMIZEPIC", "C_LOADGAMEPIC", "C_SAVEGAMEPIC",
    "C_EPISODE1PIC", "C_EPISODE2PIC", "C_EPISODE3PIC", "C_EPISODE4PIC",
    "C_EPISODE5PIC", "C_EPISODE6PIC", "C_CODEPIC", "C_TIMECODEPIC",
    "C_LEVELPIC", "C_NAMEPIC", "C_SCOREPIC", "C_JOY1PIC", "C_JOY2PIC",
    "L_GUYPIC", "L_COLONPIC",
    "L_NUM0PIC", "L_NUM1PIC", "L_NUM2PIC", "L_NUM3PIC", "L_NUM4PIC",
    "L_NUM5PIC", "L_NUM6PIC", "L_NUM7PIC", "L_NUM8PIC", "L_NUM9PIC",
    "L_PERCENTPIC",
    "L_APIC", "L_BPIC", "L_CPIC", "L_DPIC", "L_EPIC", "L_FPIC", "L_GPIC",
    "L_HPIC", "L_IPIC", "L_JPIC", "L_KPIC", "L_LPIC", "L_MPIC", "L_NPIC",
    "L_OPIC", "L_PPIC", "L_QPIC", "L_RPIC", "L_SPIC", "L_TPIC", "L_UPIC",
    "L_VPIC", "L_WPIC", "L_XPIC", "L_YPIC", "L_ZPIC",
    "L_EXPOINTPIC", "L_APOSTROPHEPIC", "L_GUY2PIC", "L_BJWINSPIC",
    "STATUSBARPIC", "TITLEPIC", "PG13PIC", "CREDITSPIC", "HIGHSCORESPIC",
    "KNIFEPIC", "GUNPIC", "MACHINEGUNPIC", "GATLINGGUNPIC",
    "NOKEYPIC", "GOLDKEYPIC", "SILVERKEYPIC",
    "N_BLANKPIC", "N_0PIC", "N_1PIC", "N_2PIC", "N_3PIC", "N_4PIC",
    "N_5PIC", "N_6PIC", "N_7PIC", "N_8PIC", "N_9PIC",
    "FACE1APIC", "FACE1BPIC", "FACE1CPIC",
    "FACE2APIC", "FACE2BPIC", "FACE2CPIC",
    "FACE3APIC", "FACE3BPIC", "FACE3CPIC",
    "FACE4APIC", "FACE4BPIC", "FACE4CPIC",
    "FACE5APIC", "FACE5BPIC", "FACE5CPIC",
    "FACE6APIC", "FACE6BPIC", "FACE6CPIC",
    "FACE7APIC", "FACE7BPIC", "FACE7CPIC",
    "FACE8APIC",
    "GOTGATLINGPIC", "MUTANTBJPIC", "PAUSEDPIC", "GETPSYCHEDPIC",
];

/// Symbolic names for AUDIOWL6 sound enum (0..87).
/// PC, AdLib, and Digi chunks all share these names — only the offset
/// (STARTPCSOUNDS / STARTADLIBSOUNDS / STARTDIGISOUNDS) differs.
#[rustfmt::skip]
const SOUND_NAMES: &[&str] = &[
    "HITWALLSND", "SELECTWPNSND", "SELECTITEMSND", "HEARTBEATSND",
    "MOVEGUN2SND", "MOVEGUN1SND", "NOWAYSND", "NAZIHITPLAYERSND",
    "SCHABBSTHROWSND", "PLAYERDEATHSND", "DOGDEATHSND", "ATKGATLINGSND",
    "GETKEYSND", "NOITEMSND", "WALK1SND", "WALK2SND", "TAKEDAMAGESND",
    "GAMEOVERSND", "OPENDOORSND", "CLOSEDOORSND", "DONOTHINGSND", "HALTSND",
    "DEATHSCREAM2SND", "ATKKNIFESND", "ATKPISTOLSND", "DEATHSCREAM3SND",
    "ATKMACHINEGUNSND", "HITENEMYSND", "SHOOTDOORSND", "DEATHSCREAM1SND",
    "GETMACHINESND", "GETAMMOSND", "SHOOTSND", "HEALTH1SND", "HEALTH2SND",
    "BONUS1SND", "BONUS2SND", "BONUS3SND", "GETGATLINGSND", "ESCPRESSEDSND",
    "LEVELDONESND", "DOGBARKSND", "ENDBONUS1SND", "ENDBONUS2SND",
    "BONUS1UPSND", "BONUS4SND", "PUSHWALLSND", "NOBONUSSND", "PERCENT100SND",
    "BOSSACTIVESND", "MUTTISND", "SCHUTZADSND", "AHHHGSND", "DIESND",
    "EVASND", "GUTENTAGSND", "LEBENSND", "SCHEISTSND", "NAZIFIRESND",
    "BOSSFIRESND", "SSFIRESND", "SLURPIESND", "TOT_HUNDSND", "MEINGOTTSND",
    "SCHABBSHASND", "HITLERHASND", "SPIONSND", "NEINSOVASSND", "DOGATTACKSND",
    "FLAMETHROWERSND", "MECHSTEPSND", "GOOBSSND", "YEAHSND",
    "DEATHSCREAM4SND", "DEATHSCREAM5SND", "DEATHSCREAM6SND",
    "DEATHSCREAM7SND", "DEATHSCREAM8SND", "DEATHSCREAM9SND",
    "DONNERSND", "EINESND", "ERLAUBENSND", "KEINSND", "MEINSND", "ROSESND",
    "MISSILEFIRESND", "MISSILEHITSND",
];

/// Symbolic names for AUDIOWL6 music enum (0..27).
#[rustfmt::skip]
const MUSIC_NAMES: &[&str] = &[
    "CORNER_MUS", "DUNGEON_MUS", "WARMARCH_MUS", "GETTHEM_MUS",
    "HEADACHE_MUS", "HITLWLTZ_MUS", "INTROCW3_MUS", "NAZI_NOR_MUS",
    "NAZI_OMI_MUS", "POW_MUS", "SALUTE_MUS", "SEARCHN_MUS", "SUSPENSE_MUS",
    "VICTORS_MUS", "WONDERIN_MUS", "FUNKYOU_MUS", "ENDLEVEL_MUS",
    "GOINGAFT_MUS", "PREGNANT_MUS", "ULTIMATE_MUS", "NAZI_RAP_MUS",
    "ZEROHOUR_MUS", "TWELFTH_MUS", "ROSTER_MUS", "URAHERO_MUS", "VICMARCH_MUS",
    "PACMAN_MUS",
];

/// Mapping from VSWAP digi-list index → soundnames enum value.
/// Lifted from `wolfdigimap` in WL_MAIN.C (the !SPEAR / !SPEARDEMO branch).
/// The C array maps the other direction (sound -> digi); this is the inverse.
const DIGI_INDEX_TO_SOUND: &[(usize, &str)] = &[
    (21, "HALTSND"),
    (41, "DOGBARKSND"),
    (19, "CLOSEDOORSND"),
    (18, "OPENDOORSND"),
    (26, "ATKMACHINEGUNSND"),
    (24, "ATKPISTOLSND"),
    (11, "ATKGATLINGSND"),
    (51, "SCHUTZADSND"),
    (55, "GUTENTAGSND"),
    (50, "MUTTISND"),
    (59, "BOSSFIRESND"),
    (60, "SSFIRESND"),
    (29, "DEATHSCREAM1SND"),
    (22, "DEATHSCREAM2SND"),  // also doubly-mapped to DEATHSCREAM3SND
    (16, "TAKEDAMAGESND"),
    (46, "PUSHWALLSND"),
    (10, "DOGDEATHSND"),
    (52, "AHHHGSND"),
    (53, "DIESND"),
    (54, "EVASND"),
    (56, "LEBENSND"),
    (58, "NAZIFIRESND"),
    (61, "SLURPIESND"),
    (62, "TOT_HUNDSND"),
    (63, "MEINGOTTSND"),
    (64, "SCHABBSHASND"),
    (65, "HITLERHASND"),
    (66, "SPIONSND"),
    (67, "NEINSOVASSND"),
    (68, "DOGATTACKSND"),
    (40, "LEVELDONESND"),
    (70, "MECHSTEPSND"),
    (72, "YEAHSND"),
    (57, "SCHEISTSND"),
    (73, "DEATHSCREAM4SND"),
    (74, "DEATHSCREAM5SND"),
    (79, "DONNERSND"),
    (80, "EINESND"),
    (81, "ERLAUBENSND"),
    (75, "DEATHSCREAM6SND"),
    (76, "DEATHSCREAM7SND"),
    (77, "DEATHSCREAM8SND"),
    (78, "DEATHSCREAM9SND"),
    (82, "KEINSND"),
    (83, "MEINSND"),
    (84, "ROSESND"),
];

/// Sprite enum for WL6, lifted from WL_DEF.H (the !SPEAR branch).
/// Index = SPR_* enum value = (VSWAP page index − sprite_start).
/// One typo from the original survives intact: `MACHINEGUNATK3` is
/// missing the `SPR_` prefix in WL_DEF.H, so we keep it that way.
#[rustfmt::skip]
const SPRITE_NAMES: &[&str] = &[
    "SPR_DEMO", "SPR_DEATHCAM",
    // statics 0..47
    "SPR_STAT_0", "SPR_STAT_1", "SPR_STAT_2", "SPR_STAT_3",
    "SPR_STAT_4", "SPR_STAT_5", "SPR_STAT_6", "SPR_STAT_7",
    "SPR_STAT_8", "SPR_STAT_9", "SPR_STAT_10", "SPR_STAT_11",
    "SPR_STAT_12", "SPR_STAT_13", "SPR_STAT_14", "SPR_STAT_15",
    "SPR_STAT_16", "SPR_STAT_17", "SPR_STAT_18", "SPR_STAT_19",
    "SPR_STAT_20", "SPR_STAT_21", "SPR_STAT_22", "SPR_STAT_23",
    "SPR_STAT_24", "SPR_STAT_25", "SPR_STAT_26", "SPR_STAT_27",
    "SPR_STAT_28", "SPR_STAT_29", "SPR_STAT_30", "SPR_STAT_31",
    "SPR_STAT_32", "SPR_STAT_33", "SPR_STAT_34", "SPR_STAT_35",
    "SPR_STAT_36", "SPR_STAT_37", "SPR_STAT_38", "SPR_STAT_39",
    "SPR_STAT_40", "SPR_STAT_41", "SPR_STAT_42", "SPR_STAT_43",
    "SPR_STAT_44", "SPR_STAT_45", "SPR_STAT_46", "SPR_STAT_47",
    // guard
    "SPR_GRD_S_1", "SPR_GRD_S_2", "SPR_GRD_S_3", "SPR_GRD_S_4",
    "SPR_GRD_S_5", "SPR_GRD_S_6", "SPR_GRD_S_7", "SPR_GRD_S_8",
    "SPR_GRD_W1_1", "SPR_GRD_W1_2", "SPR_GRD_W1_3", "SPR_GRD_W1_4",
    "SPR_GRD_W1_5", "SPR_GRD_W1_6", "SPR_GRD_W1_7", "SPR_GRD_W1_8",
    "SPR_GRD_W2_1", "SPR_GRD_W2_2", "SPR_GRD_W2_3", "SPR_GRD_W2_4",
    "SPR_GRD_W2_5", "SPR_GRD_W2_6", "SPR_GRD_W2_7", "SPR_GRD_W2_8",
    "SPR_GRD_W3_1", "SPR_GRD_W3_2", "SPR_GRD_W3_3", "SPR_GRD_W3_4",
    "SPR_GRD_W3_5", "SPR_GRD_W3_6", "SPR_GRD_W3_7", "SPR_GRD_W3_8",
    "SPR_GRD_W4_1", "SPR_GRD_W4_2", "SPR_GRD_W4_3", "SPR_GRD_W4_4",
    "SPR_GRD_W4_5", "SPR_GRD_W4_6", "SPR_GRD_W4_7", "SPR_GRD_W4_8",
    "SPR_GRD_PAIN_1", "SPR_GRD_DIE_1", "SPR_GRD_DIE_2", "SPR_GRD_DIE_3",
    "SPR_GRD_PAIN_2", "SPR_GRD_DEAD",
    "SPR_GRD_SHOOT1", "SPR_GRD_SHOOT2", "SPR_GRD_SHOOT3",
    // dogs
    "SPR_DOG_W1_1", "SPR_DOG_W1_2", "SPR_DOG_W1_3", "SPR_DOG_W1_4",
    "SPR_DOG_W1_5", "SPR_DOG_W1_6", "SPR_DOG_W1_7", "SPR_DOG_W1_8",
    "SPR_DOG_W2_1", "SPR_DOG_W2_2", "SPR_DOG_W2_3", "SPR_DOG_W2_4",
    "SPR_DOG_W2_5", "SPR_DOG_W2_6", "SPR_DOG_W2_7", "SPR_DOG_W2_8",
    "SPR_DOG_W3_1", "SPR_DOG_W3_2", "SPR_DOG_W3_3", "SPR_DOG_W3_4",
    "SPR_DOG_W3_5", "SPR_DOG_W3_6", "SPR_DOG_W3_7", "SPR_DOG_W3_8",
    "SPR_DOG_W4_1", "SPR_DOG_W4_2", "SPR_DOG_W4_3", "SPR_DOG_W4_4",
    "SPR_DOG_W4_5", "SPR_DOG_W4_6", "SPR_DOG_W4_7", "SPR_DOG_W4_8",
    "SPR_DOG_DIE_1", "SPR_DOG_DIE_2", "SPR_DOG_DIE_3", "SPR_DOG_DEAD",
    "SPR_DOG_JUMP1", "SPR_DOG_JUMP2", "SPR_DOG_JUMP3",
    // ss
    "SPR_SS_S_1", "SPR_SS_S_2", "SPR_SS_S_3", "SPR_SS_S_4",
    "SPR_SS_S_5", "SPR_SS_S_6", "SPR_SS_S_7", "SPR_SS_S_8",
    "SPR_SS_W1_1", "SPR_SS_W1_2", "SPR_SS_W1_3", "SPR_SS_W1_4",
    "SPR_SS_W1_5", "SPR_SS_W1_6", "SPR_SS_W1_7", "SPR_SS_W1_8",
    "SPR_SS_W2_1", "SPR_SS_W2_2", "SPR_SS_W2_3", "SPR_SS_W2_4",
    "SPR_SS_W2_5", "SPR_SS_W2_6", "SPR_SS_W2_7", "SPR_SS_W2_8",
    "SPR_SS_W3_1", "SPR_SS_W3_2", "SPR_SS_W3_3", "SPR_SS_W3_4",
    "SPR_SS_W3_5", "SPR_SS_W3_6", "SPR_SS_W3_7", "SPR_SS_W3_8",
    "SPR_SS_W4_1", "SPR_SS_W4_2", "SPR_SS_W4_3", "SPR_SS_W4_4",
    "SPR_SS_W4_5", "SPR_SS_W4_6", "SPR_SS_W4_7", "SPR_SS_W4_8",
    "SPR_SS_PAIN_1", "SPR_SS_DIE_1", "SPR_SS_DIE_2", "SPR_SS_DIE_3",
    "SPR_SS_PAIN_2", "SPR_SS_DEAD",
    "SPR_SS_SHOOT1", "SPR_SS_SHOOT2", "SPR_SS_SHOOT3",
    // mutant
    "SPR_MUT_S_1", "SPR_MUT_S_2", "SPR_MUT_S_3", "SPR_MUT_S_4",
    "SPR_MUT_S_5", "SPR_MUT_S_6", "SPR_MUT_S_7", "SPR_MUT_S_8",
    "SPR_MUT_W1_1", "SPR_MUT_W1_2", "SPR_MUT_W1_3", "SPR_MUT_W1_4",
    "SPR_MUT_W1_5", "SPR_MUT_W1_6", "SPR_MUT_W1_7", "SPR_MUT_W1_8",
    "SPR_MUT_W2_1", "SPR_MUT_W2_2", "SPR_MUT_W2_3", "SPR_MUT_W2_4",
    "SPR_MUT_W2_5", "SPR_MUT_W2_6", "SPR_MUT_W2_7", "SPR_MUT_W2_8",
    "SPR_MUT_W3_1", "SPR_MUT_W3_2", "SPR_MUT_W3_3", "SPR_MUT_W3_4",
    "SPR_MUT_W3_5", "SPR_MUT_W3_6", "SPR_MUT_W3_7", "SPR_MUT_W3_8",
    "SPR_MUT_W4_1", "SPR_MUT_W4_2", "SPR_MUT_W4_3", "SPR_MUT_W4_4",
    "SPR_MUT_W4_5", "SPR_MUT_W4_6", "SPR_MUT_W4_7", "SPR_MUT_W4_8",
    "SPR_MUT_PAIN_1", "SPR_MUT_DIE_1", "SPR_MUT_DIE_2", "SPR_MUT_DIE_3",
    "SPR_MUT_PAIN_2", "SPR_MUT_DIE_4", "SPR_MUT_DEAD",
    "SPR_MUT_SHOOT1", "SPR_MUT_SHOOT2", "SPR_MUT_SHOOT3", "SPR_MUT_SHOOT4",
    // officer
    "SPR_OFC_S_1", "SPR_OFC_S_2", "SPR_OFC_S_3", "SPR_OFC_S_4",
    "SPR_OFC_S_5", "SPR_OFC_S_6", "SPR_OFC_S_7", "SPR_OFC_S_8",
    "SPR_OFC_W1_1", "SPR_OFC_W1_2", "SPR_OFC_W1_3", "SPR_OFC_W1_4",
    "SPR_OFC_W1_5", "SPR_OFC_W1_6", "SPR_OFC_W1_7", "SPR_OFC_W1_8",
    "SPR_OFC_W2_1", "SPR_OFC_W2_2", "SPR_OFC_W2_3", "SPR_OFC_W2_4",
    "SPR_OFC_W2_5", "SPR_OFC_W2_6", "SPR_OFC_W2_7", "SPR_OFC_W2_8",
    "SPR_OFC_W3_1", "SPR_OFC_W3_2", "SPR_OFC_W3_3", "SPR_OFC_W3_4",
    "SPR_OFC_W3_5", "SPR_OFC_W3_6", "SPR_OFC_W3_7", "SPR_OFC_W3_8",
    "SPR_OFC_W4_1", "SPR_OFC_W4_2", "SPR_OFC_W4_3", "SPR_OFC_W4_4",
    "SPR_OFC_W4_5", "SPR_OFC_W4_6", "SPR_OFC_W4_7", "SPR_OFC_W4_8",
    "SPR_OFC_PAIN_1", "SPR_OFC_DIE_1", "SPR_OFC_DIE_2", "SPR_OFC_DIE_3",
    "SPR_OFC_PAIN_2", "SPR_OFC_DIE_4", "SPR_OFC_DEAD",
    "SPR_OFC_SHOOT1", "SPR_OFC_SHOOT2", "SPR_OFC_SHOOT3",
    // ghosts (pacman bonus level)
    "SPR_BLINKY_W1", "SPR_BLINKY_W2", "SPR_PINKY_W1", "SPR_PINKY_W2",
    "SPR_CLYDE_W1", "SPR_CLYDE_W2", "SPR_INKY_W1", "SPR_INKY_W2",
    // hans (boss)
    "SPR_BOSS_W1", "SPR_BOSS_W2", "SPR_BOSS_W3", "SPR_BOSS_W4",
    "SPR_BOSS_SHOOT1", "SPR_BOSS_SHOOT2", "SPR_BOSS_SHOOT3", "SPR_BOSS_DEAD",
    "SPR_BOSS_DIE1", "SPR_BOSS_DIE2", "SPR_BOSS_DIE3",
    // schabbs
    "SPR_SCHABB_W1", "SPR_SCHABB_W2", "SPR_SCHABB_W3", "SPR_SCHABB_W4",
    "SPR_SCHABB_SHOOT1", "SPR_SCHABB_SHOOT2",
    "SPR_SCHABB_DIE1", "SPR_SCHABB_DIE2", "SPR_SCHABB_DIE3", "SPR_SCHABB_DEAD",
    "SPR_HYPO1", "SPR_HYPO2", "SPR_HYPO3", "SPR_HYPO4",
    // fake
    "SPR_FAKE_W1", "SPR_FAKE_W2", "SPR_FAKE_W3", "SPR_FAKE_W4",
    "SPR_FAKE_SHOOT", "SPR_FIRE1", "SPR_FIRE2",
    "SPR_FAKE_DIE1", "SPR_FAKE_DIE2", "SPR_FAKE_DIE3", "SPR_FAKE_DIE4",
    "SPR_FAKE_DIE5", "SPR_FAKE_DEAD",
    // hitler
    "SPR_MECHA_W1", "SPR_MECHA_W2", "SPR_MECHA_W3", "SPR_MECHA_W4",
    "SPR_MECHA_SHOOT1", "SPR_MECHA_SHOOT2", "SPR_MECHA_SHOOT3", "SPR_MECHA_DEAD",
    "SPR_MECHA_DIE1", "SPR_MECHA_DIE2", "SPR_MECHA_DIE3",
    "SPR_HITLER_W1", "SPR_HITLER_W2", "SPR_HITLER_W3", "SPR_HITLER_W4",
    "SPR_HITLER_SHOOT1", "SPR_HITLER_SHOOT2", "SPR_HITLER_SHOOT3", "SPR_HITLER_DEAD",
    "SPR_HITLER_DIE1", "SPR_HITLER_DIE2", "SPR_HITLER_DIE3", "SPR_HITLER_DIE4",
    "SPR_HITLER_DIE5", "SPR_HITLER_DIE6", "SPR_HITLER_DIE7",
    // giftmacher
    "SPR_GIFT_W1", "SPR_GIFT_W2", "SPR_GIFT_W3", "SPR_GIFT_W4",
    "SPR_GIFT_SHOOT1", "SPR_GIFT_SHOOT2",
    "SPR_GIFT_DIE1", "SPR_GIFT_DIE2", "SPR_GIFT_DIE3", "SPR_GIFT_DEAD",
    // rocket / smoke / explosion
    "SPR_ROCKET_1", "SPR_ROCKET_2", "SPR_ROCKET_3", "SPR_ROCKET_4",
    "SPR_ROCKET_5", "SPR_ROCKET_6", "SPR_ROCKET_7", "SPR_ROCKET_8",
    "SPR_SMOKE_1", "SPR_SMOKE_2", "SPR_SMOKE_3", "SPR_SMOKE_4",
    "SPR_BOOM_1", "SPR_BOOM_2", "SPR_BOOM_3",
    // gretel
    "SPR_GRETEL_W1", "SPR_GRETEL_W2", "SPR_GRETEL_W3", "SPR_GRETEL_W4",
    "SPR_GRETEL_SHOOT1", "SPR_GRETEL_SHOOT2", "SPR_GRETEL_SHOOT3", "SPR_GRETEL_DEAD",
    "SPR_GRETEL_DIE1", "SPR_GRETEL_DIE2", "SPR_GRETEL_DIE3",
    // fat face
    "SPR_FAT_W1", "SPR_FAT_W2", "SPR_FAT_W3", "SPR_FAT_W4",
    "SPR_FAT_SHOOT1", "SPR_FAT_SHOOT2", "SPR_FAT_SHOOT3", "SPR_FAT_SHOOT4",
    "SPR_FAT_DIE1", "SPR_FAT_DIE2", "SPR_FAT_DIE3", "SPR_FAT_DEAD",
    // bj
    "SPR_BJ_W1", "SPR_BJ_W2", "SPR_BJ_W3", "SPR_BJ_W4",
    "SPR_BJ_JUMP1", "SPR_BJ_JUMP2", "SPR_BJ_JUMP3", "SPR_BJ_JUMP4",
    // player attack frames
    "SPR_KNIFEREADY", "SPR_KNIFEATK1", "SPR_KNIFEATK2", "SPR_KNIFEATK3",
    "SPR_KNIFEATK4",
    "SPR_PISTOLREADY", "SPR_PISTOLATK1", "SPR_PISTOLATK2", "SPR_PISTOLATK3",
    "SPR_PISTOLATK4",
    "SPR_MACHINEGUNREADY", "SPR_MACHINEGUNATK1", "SPR_MACHINEGUNATK2",
    "MACHINEGUNATK3", "SPR_MACHINEGUNATK4",
    "SPR_CHAINREADY", "SPR_CHAINATK1", "SPR_CHAINATK2", "SPR_CHAINATK3",
    "SPR_CHAINATK4",
];

// -----------------------------------------------------------------------------
// Game palette (GAMEPAL.OBJ)
// -----------------------------------------------------------------------------

type Rgb = (u8, u8, u8);

/// Parse the 256-entry VGA palette out of GAMEPAL.OBJ (a Borland OMF object
/// file).  We hunt for the LEDATA record (type 0xA0) whose payload is exactly
/// 768 bytes (256 × 3-byte RGB values, 6-bit each).
fn load_game_palette(path: &str) -> Result<Vec<Rgb>> {
    let data = fs::read(path).with_context(|| format!("reading palette {path}"))?;
    let mut i = 0;
    while i + 3 <= data.len() {
        let rec_type = data[i];
        let rec_len = u16::from_le_bytes([data[i + 1], data[i + 2]]) as usize;
        if rec_type == 0xA0 && i + 3 + rec_len <= data.len() {
            // skip seg_index (1) + data_offset (2) = 3 bytes;
            // last byte of record is checksum (excluded).
            let payload = &data[i + 3 + 3..i + 3 + rec_len - 1];
            if payload.len() == 768 {
                return Ok(payload
                    .chunks(3)
                    .map(|c| (c[0].wrapping_mul(4), c[1].wrapping_mul(4), c[2].wrapping_mul(4)))
                    .collect());
            }
        }
        i += 3 + rec_len;
    }
    bail!("palette LEDATA record not found in {path}");
}

// -----------------------------------------------------------------------------
// VGAGRAPH: Huffman-decompressed chunks
// -----------------------------------------------------------------------------

struct HuffmanNode {
    val0: u16, // bit = 0
    val1: u16, // bit = 1
}

fn read_vga_offsets(path: &Path) -> Result<Vec<u32>> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let mut cursor = Cursor::new(bytes);
    let mut offsets = Vec::new();
    loop {
        match cursor.read_u24::<LE>() {
            Ok(v) => offsets.push(v),
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(offsets)
}

fn read_huffman_tree(path: &Path) -> Result<Vec<HuffmanNode>> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let mut cursor = Cursor::new(bytes);
    let mut nodes = Vec::new();
    loop {
        let val0 = match cursor.read_u16::<LE>() {
            Ok(v) => v,
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        };
        let val1 = cursor.read_u16::<LE>()?;
        nodes.push(HuffmanNode { val0, val1 });
    }
    Ok(nodes)
}

/// Decompress one chunk's bit-packed Huffman codes (LSB first per byte) into
/// `expanded_len` bytes.
fn huffman_decode(compressed: &[u8], expanded_len: usize, tree: &[HuffmanNode]) -> Vec<u8> {
    const ROOT: u16 = 254;
    let mut out = Vec::with_capacity(expanded_len);
    let mut node_idx = ROOT;
    'outer: for &byte in compressed {
        for bit in 0..8 {
            let node = &tree[node_idx as usize];
            let sel = if (byte >> bit) & 1 == 1 { node.val1 } else { node.val0 };
            if sel < 0x0100 {
                out.push(sel as u8);
                if out.len() == expanded_len {
                    break 'outer;
                }
                node_idx = ROOT;
            } else {
                node_idx = sel - 0x0100;
            }
        }
    }
    out
}

/// Most VGAGRAPH chunks are prefixed by a u32 expanded-length header.  Tile
/// chunks (STARTTILE8..STARTEXTERNS) skip the prefix and use a fixed size
/// instead — see `CAL_ExpandGrChunk` in ID_CA.C.
fn huffman_expand(chunk: &[u8], tree: &[HuffmanNode]) -> Vec<u8> {
    if chunk.len() < 4 {
        return Vec::new();
    }
    let expanded_len = u32::from_le_bytes(chunk[0..4].try_into().unwrap()) as usize;
    huffman_decode(&chunk[4..], expanded_len, tree)
}

/// Returns (chunks, dimensions for pic chunks).  `chunks[i]` is the
/// decompressed bytes for chunk `i`; empty if the chunk has zero length.
struct VgaGraph {
    chunks: Vec<Vec<u8>>,
    pic_dims: Vec<(u16, u16)>, // index by (chunk - STARTPICS); only valid for pics
}

fn load_vga_graph(data_dir: &Path) -> Result<VgaGraph> {
    let offsets = read_vga_offsets(&data_dir.join("VGAHEAD.WL6"))?;
    let tree = read_huffman_tree(&data_dir.join("VGADICT.WL6"))?;
    let raw = fs::read(data_dir.join("VGAGRAPH.WL6")).context("reading VGAGRAPH.WL6")?;

    let mut chunks: Vec<Vec<u8>> = Vec::with_capacity(offsets.len());
    for (i, pair) in offsets.windows(2).enumerate() {
        let start = pair[0] as usize;
        let end = pair[1] as usize;
        if start >= end || end > raw.len() {
            chunks.push(Vec::new());
            continue;
        }
        let raw_chunk = &raw[start..end];
        // Tile8 chunks have no length prefix — fixed expanded size.
        // (See CAL_ExpandGrChunk: chunks in [STARTTILE8, STARTEXTERNS).)
        if i == STARTTILE8 {
            chunks.push(huffman_decode(raw_chunk, NUMTILE8 * 64, &tree));
        } else {
            chunks.push(huffman_expand(raw_chunk, &tree));
        }
    }

    // Parse the picture table (chunk STRUCTPIC == 0).  4 bytes per pic:
    // (width: u16, height: u16).  NUMPICS entries, one per pic chunk.
    let mut pic_dims = Vec::with_capacity(NUMPICS);
    let pictable = &chunks[STRUCTPIC];
    for i in 0..NUMPICS {
        let off = i * 4;
        if off + 4 > pictable.len() {
            pic_dims.push((0, 0));
        } else {
            let w = u16::from_le_bytes([pictable[off], pictable[off + 1]]);
            let h = u16::from_le_bytes([pictable[off + 2], pictable[off + 3]]);
            pic_dims.push((w, h));
        }
    }

    Ok(VgaGraph { chunks, pic_dims })
}

// -----------------------------------------------------------------------------
// Pic extraction (chunks STARTPICS..STARTPICS+NUMPICS)
// -----------------------------------------------------------------------------

fn extract_pics(graph: &VgaGraph, palette: &[Rgb], out_dir: &Path) -> Result<()> {
    let dir = out_dir.join("graphics/pics");
    fs::create_dir_all(&dir)?;

    for i in 0..NUMPICS {
        let chunk_idx = STARTPICS + i;
        let data = &graph.chunks[chunk_idx];
        let (w, h) = graph.pic_dims[i];
        if data.is_empty() || w == 0 || h == 0 {
            log::warn!("pic chunk {chunk_idx} ({}) is empty", PIC_NAMES[i]);
            continue;
        }

        // Pic chunks are stored chunky (4-plane interleave), like wall
        // textures: 4 planes of (w/4 × h) bytes, plane-major.  Each pixel's
        // plane is `x % 4`, byte offset `plane * (w/4 * h) + y * (w/4) + x/4`.
        let pixels = depic_planar(data, w as usize, h as usize, palette);

        let name = format!("{:03}_{}.png", chunk_idx, PIC_NAMES[i]);
        let path = dir.join(&name);
        image::RgbImage::from_raw(w as u32, h as u32, pixels)
            .ok_or_else(|| anyhow::anyhow!("bad pic dimensions {w}x{h} for {name}"))?
            .save(&path)
            .with_context(|| format!("writing {}", path.display()))?;
    }

    log::info!("wrote {NUMPICS} pics to {}", dir.display());
    Ok(())
}

/// Deplanarize a planar VGA pic: 4 planes laid out plane-major.  Each plane
/// is `(w/4) * h` bytes and owns columns where `x % 4 == plane`.
fn depic_planar(data: &[u8], w: usize, h: usize, palette: &[Rgb]) -> Vec<u8> {
    let plane_w = w / 4;
    let mut out = vec![0u8; w * h * 3];
    for y in 0..h {
        for x in 0..w {
            let plane = x % 4;
            let idx = plane * plane_w * h + y * plane_w + x / 4;
            let pal_idx = data.get(idx).copied().unwrap_or(0) as usize;
            let (r, g, b) = palette[pal_idx];
            let dst = (y * w + x) * 3;
            out[dst] = r;
            out[dst + 1] = g;
            out[dst + 2] = b;
        }
    }
    out
}

// -----------------------------------------------------------------------------
// Font extraction (chunks STARTFONT..STARTFONT+NUMFONT)
//
// Font header (770 bytes total):
//   int  height          // 2 bytes
//   int  location[256]   // 512 bytes — byte offset (from start of struct)
//                        //              to glyph row-major data
//   char width[256]      // 256 bytes — glyph width in pixels (0 if missing)
//   byte data[]          // glyph bytes; 0 = transparent, non-zero = ink
//
// Each glyph's bitmap is `width[ch] * height` bytes, row-major (i.e.
// data[loc + row * width + col]).  We pack all glyphs into a single
// horizontal atlas PNG with a JSON sidecar describing positions.
// -----------------------------------------------------------------------------

fn extract_fonts(graph: &VgaGraph, out_dir: &Path) -> Result<()> {
    let dir = out_dir.join("fonts");
    fs::create_dir_all(&dir)?;

    for i in 0..NUMFONT {
        let chunk_idx = STARTFONT + i;
        let data = &graph.chunks[chunk_idx];
        if data.len() < 770 {
            log::warn!("font chunk {chunk_idx} too small: {} bytes", data.len());
            continue;
        }

        let height = u16::from_le_bytes([data[0], data[1]]) as usize;
        let mut locations = [0u16; 256];
        for ch in 0..256 {
            let off = 2 + ch * 2;
            locations[ch] = u16::from_le_bytes([data[off], data[off + 1]]);
        }
        let widths: &[u8] = &data[2 + 512..2 + 512 + 256];

        let total_w: usize = widths.iter().map(|&w| w as usize).sum();
        if total_w == 0 || height == 0 {
            log::warn!("font {i} has no glyphs");
            continue;
        }

        // Build atlas: 1px-tall row for each pixel row, total_w wide.
        let mut atlas = vec![0u8; total_w * height * 4];
        let mut glyphs: Vec<(u8, usize, u8)> = Vec::new(); // (ch, x, width)
        let mut x_cursor = 0usize;

        for ch in 0..256 {
            let w = widths[ch] as usize;
            if w == 0 {
                continue;
            }
            let loc = locations[ch] as usize;
            let glyph_end = loc + w * height;
            if glyph_end > data.len() {
                log::warn!("font {i} glyph {ch} OOB: loc={loc} w={w} h={height}");
                continue;
            }

            for row in 0..height {
                for col in 0..w {
                    let src = data[loc + row * w + col];
                    if src != 0 {
                        let px = x_cursor + col;
                        let py = row;
                        let dst = (py * total_w + px) * 4;
                        atlas[dst] = 255;
                        atlas[dst + 1] = 255;
                        atlas[dst + 2] = 255;
                        atlas[dst + 3] = 255;
                    }
                }
            }

            glyphs.push((ch as u8, x_cursor, w as u8));
            x_cursor += w;
        }

        let png_path = dir.join(format!("font_{}.png", chunk_idx));
        image::RgbaImage::from_raw(total_w as u32, height as u32, atlas)
            .context("building font atlas image")?
            .save(&png_path)
            .with_context(|| format!("writing {}", png_path.display()))?;

        // Hand-rolled JSON sidecar: { height, glyphs: { "<ch>": { x, width } } }
        let mut json = String::new();
        json.push_str("{\n");
        json.push_str(&format!("  \"height\": {height},\n"));
        json.push_str(&format!("  \"atlas_width\": {total_w},\n"));
        json.push_str("  \"glyphs\": {\n");
        for (i, (ch, x, w)) in glyphs.iter().enumerate() {
            let comma = if i + 1 < glyphs.len() { "," } else { "" };
            json.push_str(&format!(
                "    \"{ch}\": {{ \"x\": {x}, \"width\": {w} }}{comma}\n"
            ));
        }
        json.push_str("  }\n}\n");
        let json_path = dir.join(format!("font_{}.json", chunk_idx));
        fs::write(&json_path, json)
            .with_context(|| format!("writing {}", json_path.display()))?;
    }

    log::info!("wrote fonts to {}", dir.display());
    Ok(())
}

// -----------------------------------------------------------------------------
// VSWAP (page file)
// -----------------------------------------------------------------------------

struct VSwap {
    /// Raw bytes for each page (length per page table).
    pages: Vec<Vec<u8>>,
    sprite_start: usize,
    sound_start: usize,
}

fn load_vswap(data_dir: &Path) -> Result<VSwap> {
    let bytes = fs::read(data_dir.join("VSWAP.WL6")).context("reading VSWAP.WL6")?;
    let mut cursor = Cursor::new(&bytes);
    let total_pages = cursor.read_u16::<LE>()? as usize;
    let sprite_start = cursor.read_u16::<LE>()? as usize;
    let sound_start = cursor.read_u16::<LE>()? as usize;

    let mut offsets = Vec::with_capacity(total_pages);
    for _ in 0..total_pages {
        offsets.push(cursor.read_u32::<LE>()? as usize);
    }
    let mut lengths = Vec::with_capacity(total_pages);
    for _ in 0..total_pages {
        lengths.push(cursor.read_u16::<LE>()? as usize);
    }

    let mut pages = Vec::with_capacity(total_pages);
    for i in 0..total_pages {
        let start = offsets[i];
        let end = start + lengths[i];
        if start == 0 || end > bytes.len() {
            pages.push(Vec::new());
        } else {
            pages.push(bytes[start..end].to_vec());
        }
    }

    Ok(VSwap { pages, sprite_start, sound_start })
}

// -----------------------------------------------------------------------------
// Wall texture extraction (VSWAP pages 0..sprite_start)
//
// Each wall is 64×64 paletted, stored column-major (column 0 top-to-bottom,
// then column 1, etc.) — a single contiguous 4096-byte page.
// -----------------------------------------------------------------------------

fn extract_walls(swap: &VSwap, palette: &[Rgb], out_dir: &Path) -> Result<()> {
    let dir = out_dir.join("graphics/walls");
    fs::create_dir_all(&dir)?;

    let count = swap.sprite_start;
    for idx in 0..count {
        let page = &swap.pages[idx];
        if page.len() < 64 * 64 {
            continue;
        }
        let mut pixels = vec![0u8; 64 * 64 * 3];
        for x in 0..64usize {
            for y in 0..64usize {
                let pal_idx = page[x * 64 + y] as usize;
                let (r, g, b) = palette[pal_idx];
                let dst = (y * 64 + x) * 3;
                pixels[dst] = r;
                pixels[dst + 1] = g;
                pixels[dst + 2] = b;
            }
        }

        let path = dir.join(format!("{:03}.png", idx));
        image::RgbImage::from_raw(64, 64, pixels)
            .context("building wall image")?
            .save(&path)
            .with_context(|| format!("writing {}", path.display()))?;
    }

    log::info!("wrote {count} walls to {}", dir.display());
    Ok(())
}

// -----------------------------------------------------------------------------
// Sprite extraction (VSWAP pages sprite_start..sound_start)
//
// Sprite chunk layout (t_compshape in WL_DEF.H):
//   u16 leftpix         leftmost non-empty column (0..63)
//   u16 rightpix        rightmost non-empty column (0..63, inclusive)
//   u16 dataofs[N]      byte offset (from start of chunk) to each column's
//                       command stream, N = rightpix - leftpix + 1
//   <command streams + raw pixel bytes follow>
//
// Each column's command stream is a sequence of "post" triples terminated
// by a u16 endY of 0:
//   u16 endY*2          row past the post's last pixel, doubled
//   u16 sprdataofs      byte offset such that chunk[sprdataofs + y]
//                       gives the pixel for row y
//   u16 startY*2        row of the post's first pixel, doubled
//
// Transparent regions are simply not covered by any post.  Output is
// 64×64 RGBA PNG, alpha=0 outside posts, alpha=255 inside.
// -----------------------------------------------------------------------------

fn extract_sprites(swap: &VSwap, palette: &[Rgb], out_dir: &Path) -> Result<()> {
    let dir = out_dir.join("graphics/sprites");
    fs::create_dir_all(&dir)?;

    let count = swap.sound_start.saturating_sub(swap.sprite_start);
    let mut written = 0usize;

    for i in 0..count {
        let page = &swap.pages[swap.sprite_start + i];
        if page.len() < 4 {
            continue;
        }

        let leftpix = u16::from_le_bytes([page[0], page[1]]) as usize;
        let rightpix = u16::from_le_bytes([page[2], page[3]]) as usize;
        if rightpix < leftpix || rightpix > 63 {
            log::warn!("sprite {i}: bad bounds left={leftpix} right={rightpix}");
            continue;
        }

        let mut rgba = vec![0u8; 64 * 64 * 4];
        let n_cols = rightpix - leftpix + 1;

        for col_idx in 0..n_cols {
            let x = leftpix + col_idx;
            let dataofs_pos = 4 + col_idx * 2;
            if dataofs_pos + 2 > page.len() {
                break;
            }
            let mut p = u16::from_le_bytes([page[dataofs_pos], page[dataofs_pos + 1]]) as usize;

            // Walk the post triples.
            loop {
                if p + 2 > page.len() {
                    break;
                }
                let endy = u16::from_le_bytes([page[p], page[p + 1]]);
                if endy == 0 {
                    break;
                }
                if p + 6 > page.len() {
                    break;
                }
                let sprdataofs = u16::from_le_bytes([page[p + 2], page[p + 3]]) as usize;
                let starty = u16::from_le_bytes([page[p + 4], page[p + 5]]);
                p += 6;

                let endy = (endy / 2) as usize;
                let starty = (starty / 2) as usize;
                if endy > 64 || starty >= endy {
                    continue;
                }

                for y in starty..endy {
                    let pixel_idx = sprdataofs + y;
                    if pixel_idx >= page.len() {
                        continue;
                    }
                    let pal_idx = page[pixel_idx] as usize;
                    let (r, g, b) = palette[pal_idx];
                    let dst = (y * 64 + x) * 4;
                    rgba[dst] = r;
                    rgba[dst + 1] = g;
                    rgba[dst + 2] = b;
                    rgba[dst + 3] = 255;
                }
            }
        }

        let name = SPRITE_NAMES.get(i).copied().unwrap_or("UNKNOWN");
        let path = dir.join(format!("{:04}_{}.png", i, name));
        image::RgbaImage::from_raw(64, 64, rgba)
            .context("building sprite image")?
            .save(&path)
            .with_context(|| format!("writing {}", path.display()))?;
        written += 1;
    }

    log::info!("wrote {written} sprites to {}", dir.display());
    Ok(())
}

// -----------------------------------------------------------------------------
// Digi sound extraction (VSWAP pages sound_start..)
//
// The very last VSWAP page is the digi list: pairs of u16 (page_offset, length)
// where page_offset is relative to sound_start and length is in bytes.
// Each digi sound's PCM data spans contiguous pages starting at that offset,
// then is truncated to `length` bytes.  Format: 8-bit unsigned mono @ 7000 Hz.
// -----------------------------------------------------------------------------

fn extract_digi(swap: &VSwap, out_dir: &Path) -> Result<()> {
    let dir = out_dir.join("sounds/digi");
    fs::create_dir_all(&dir)?;

    let last_idx = swap.pages.len().saturating_sub(1);
    let digi_list_page = &swap.pages[last_idx];

    // Build a per-digi-index name table for the WL6 mapping.
    let mut digi_names: Vec<Option<(usize, &str)>> = vec![None; 256];
    for (digi_idx, (sound_idx, name)) in DIGI_INDEX_TO_SOUND.iter().enumerate() {
        digi_names[digi_idx] = Some((*sound_idx, *name));
    }

    let max_entries = digi_list_page.len() / 4;
    let mut written = 0usize;
    for entry in 0..max_entries {
        let off = entry * 4;
        let page_offset =
            u16::from_le_bytes([digi_list_page[off], digi_list_page[off + 1]]) as usize;
        let length =
            u16::from_le_bytes([digi_list_page[off + 2], digi_list_page[off + 3]]) as usize;
        if length == 0 {
            break;
        }

        let start = swap.sound_start + page_offset;
        if start >= swap.pages.len() {
            break;
        }

        // Concatenate enough pages to cover `length` bytes, then truncate.
        let mut pcm = Vec::with_capacity(length);
        let mut p = start;
        while pcm.len() < length && p < swap.pages.len() {
            pcm.extend_from_slice(&swap.pages[p]);
            p += 1;
        }
        pcm.truncate(length);

        let (file_idx, name) = digi_names[entry]
            .map(|(s, n)| (s, n.to_string()))
            .unwrap_or_else(|| (entry, format!("DIGI_{:02}", entry)));

        let wav = encode_wav_u8(&pcm, DIGI_SAMPLE_RATE);
        let path = dir.join(format!("{:03}_{}.wav", file_idx, name));
        fs::write(&path, wav).with_context(|| format!("writing {}", path.display()))?;
        written += 1;
    }

    log::info!("wrote {written} digi sounds to {}", dir.display());
    Ok(())
}

/// Wrap raw 8-bit unsigned mono PCM in a minimal RIFF/WAVE container.
fn encode_wav_u8(pcm: &[u8], sample_rate: u32) -> Vec<u8> {
    let data_len = pcm.len() as u32;
    let fmt_size: u32 = 16;
    let riff_size: u32 = 4 + (8 + fmt_size) + (8 + data_len);
    let mut out = Vec::with_capacity(8 + riff_size as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&riff_size.to_le_bytes());
    out.extend_from_slice(b"WAVE");

    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&fmt_size.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());          // PCM
    out.extend_from_slice(&1u16.to_le_bytes());          // 1 channel
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());   // byte rate (8-bit mono)
    out.extend_from_slice(&1u16.to_le_bytes());          // block align
    out.extend_from_slice(&8u16.to_le_bytes());          // bits per sample

    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    out.extend_from_slice(pcm);
    out
}

// -----------------------------------------------------------------------------
// AUDIOT extraction (PC speaker, AdLib SFX, IMF music)
// -----------------------------------------------------------------------------

fn extract_audiot(data_dir: &Path, out_dir: &Path) -> Result<()> {
    let head = fs::read(data_dir.join("AUDIOHED.WL6")).context("reading AUDIOHED.WL6")?;
    let body = fs::read(data_dir.join("AUDIOT.WL6")).context("reading AUDIOT.WL6")?;
    let offsets: Vec<u32> = head
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();

    let pc_dir = out_dir.join("sounds/pc");
    let adlib_dir = out_dir.join("sounds/adlib");
    let music_dir = out_dir.join("sounds/music");
    fs::create_dir_all(&pc_dir)?;
    fs::create_dir_all(&adlib_dir)?;
    fs::create_dir_all(&music_dir)?;

    let mut pc_n = 0;
    let mut adlib_n = 0;
    let mut music_n = 0;

    for (i, pair) in offsets.windows(2).enumerate() {
        if i >= NUMSNDCHUNKS {
            break;
        }
        let start = pair[0] as usize;
        let end = pair[1] as usize;
        if start >= end || end > body.len() {
            continue;
        }
        let chunk = &body[start..end];
        if chunk.is_empty() {
            continue;
        }

        let (dir, file_idx, name, ext) = match i {
            // PC speaker (sound enum 0..87): 6-byte header (length u32, prio u16),
            // then `length` timer divisor bytes.  Saved raw — playback isn't
            // wired up yet.
            x if x < STARTADLIBSOUNDS => {
                pc_n += 1;
                let s = x - STARTPCSOUNDS;
                (&pc_dir, s, SOUND_NAMES[s].to_string(), "pcs")
            }
            // AdLib SFX (sound enum 0..87): 16-byte instrument + length/prio
            // header, then OPL2 register write byte stream.
            x if x < STARTDIGISOUNDS => {
                adlib_n += 1;
                let s = x - STARTADLIBSOUNDS;
                (&adlib_dir, s, SOUND_NAMES[s].to_string(), "adlib")
            }
            // Digi range in AUDIOT — usually empty in the original DOS files;
            // real digi data lives in VSWAP.  Skip silently.
            x if x < STARTMUSIC => continue,
            // IMF music tracks.
            x => {
                music_n += 1;
                let m = x - STARTMUSIC;
                let nm = MUSIC_NAMES.get(m).copied().unwrap_or("UNKNOWN").to_string();
                (&music_dir, m, nm, "imf")
            }
        };

        let path = dir.join(format!("{:03}_{}.{}", file_idx, name, ext));
        fs::write(&path, chunk).with_context(|| format!("writing {}", path.display()))?;
    }

    log::info!("wrote {pc_n} PC, {adlib_n} AdLib, {music_n} music chunks");
    Ok(())
}

// -----------------------------------------------------------------------------
// Palette extraction
// -----------------------------------------------------------------------------

fn extract_palette(palette: &[Rgb], out_dir: &Path) -> Result<()> {
    let path = out_dir.join("palette.json");
    let mut json = String::new();
    json.push_str("{\n  \"format\": \"rgb888\",\n  \"entries\": [\n");
    for (i, (r, g, b)) in palette.iter().enumerate() {
        let comma = if i + 1 < palette.len() { "," } else { "" };
        json.push_str(&format!("    [{r}, {g}, {b}]{comma}\n"));
    }
    json.push_str("  ]\n}\n");
    fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
    log::info!("wrote palette ({} entries) to {}", palette.len(), path.display());
    Ok(())
}

// -----------------------------------------------------------------------------
// Tile8 extraction (VGAGRAPH chunk 135) — currently parked.
//
// In theory: one chunk packs NUMTILE8 (72) tiles of 8×8 pixels.  Each tile
// is 64 bytes of planar VGA data: 4 planes × 16 bytes (8 rows × 2 bytes per
// row).  Pixel(x, y) is at byte offset `plane*16 + y*2 + x/4`, plane = x%4.
// The 4-byte length prefix is omitted for tile chunks (see CAL_ExpandGrChunk).
//
// In practice: WL6's chunk 135 huffman-decodes to ~2240 bytes here, not
// 4608.  Either the offset table treats the chunk size differently in WL6
// or the chunk header has another quirk we haven't pinned down.  Tile8s
// are minor cosmetic assets (used by VWB_DrawTile8 for textured grid UI),
// not the in-game proportional fonts (those are chunks 1-2, already
// extracted), so this is parked rather than blocking other extraction.
// -----------------------------------------------------------------------------

#[allow(dead_code)]
fn extract_tile8s(graph: &VgaGraph, palette: &[Rgb], out_dir: &Path) -> Result<()> {
    let dir = out_dir.join("graphics/tile8");
    fs::create_dir_all(&dir)?;

    let chunk = &graph.chunks[STARTTILE8];
    if chunk.len() < NUMTILE8 * 64 {
        bail!(
            "tile8 chunk too small: {} bytes, need {}",
            chunk.len(),
            NUMTILE8 * 64
        );
    }

    for t in 0..NUMTILE8 {
        let base = t * 64;
        let mut pixels = vec![0u8; 8 * 8 * 3];
        for y in 0..8usize {
            for x in 0..8usize {
                let plane = x % 4;
                let pal_idx = chunk[base + plane * 16 + y * 2 + x / 4] as usize;
                let (r, g, b) = palette[pal_idx];
                let dst = (y * 8 + x) * 3;
                pixels[dst] = r;
                pixels[dst + 1] = g;
                pixels[dst + 2] = b;
            }
        }
        let path = dir.join(format!("{:02}.png", t));
        image::RgbImage::from_raw(8, 8, pixels)
            .context("building tile8 image")?
            .save(&path)
            .with_context(|| format!("writing {}", path.display()))?;
    }

    log::info!("wrote {NUMTILE8} tile8s to {}", dir.display());
    Ok(())
}

// -----------------------------------------------------------------------------
// Map extraction (MAPHEAD.WL6 + GAMEMAPS.WL6)
//
// MAPHEAD: u16 rlew_tag, then 100 u32 offsets into GAMEMAPS, then tile info
// (which we don't use).  Offset 0 means "no map at this slot".
//
// Each GAMEMAPS entry begins with a 38-byte header:
//   i32 planestart[3], u16 planelength[3], u16 width, u16 height, char name[16]
// Then each plane's bytes are at planestart[p].  The first 2 bytes of the
// compressed plane are the decompressed length in bytes; the rest is
// Carmack-encoded data which, once expanded, is RLEW-encoded with rlew_tag.
//
// Output: one JSON per map listing both planes as flat u16 arrays.
// -----------------------------------------------------------------------------

fn extract_maps(data_dir: &Path, out_dir: &Path) -> Result<()> {
    let dir = out_dir.join("maps");
    fs::create_dir_all(&dir)?;

    let head = fs::read(data_dir.join("MAPHEAD.WL6")).context("reading MAPHEAD.WL6")?;
    let body = fs::read(data_dir.join("GAMEMAPS.WL6")).context("reading GAMEMAPS.WL6")?;
    let mut head_cur = Cursor::new(&head);
    let rlew_tag = head_cur.read_u16::<LE>()?;

    let mut header_offsets = [0u32; 100];
    for off in &mut header_offsets {
        *off = head_cur.read_u32::<LE>()?;
    }

    let mut written = 0usize;
    for i in 0..NUMMAPS {
        let pos = header_offsets[i] as usize;
        if pos == 0 || pos + 38 > body.len() {
            continue;
        }
        let mut hdr = Cursor::new(&body[pos..pos + 38]);
        let mut planestart = [0i32; 3];
        for v in &mut planestart {
            *v = hdr.read_i32::<LE>()?;
        }
        let mut planelength = [0u16; 3];
        for v in &mut planelength {
            *v = hdr.read_u16::<LE>()?;
        }
        let width = hdr.read_u16::<LE>()? as usize;
        let height = hdr.read_u16::<LE>()? as usize;
        let mut name_bytes = [0u8; 16];
        hdr.read_exact(&mut name_bytes)?;
        let name = String::from_utf8_lossy(&name_bytes)
            .trim_end_matches('\0')
            .trim()
            .to_string();

        let mut planes = Vec::with_capacity(MAPPLANES);
        for p in 0..MAPPLANES {
            let start = planestart[p] as usize;
            let len = planelength[p] as usize;
            if start == 0 || len < 2 || start + len > body.len() {
                planes.push(Vec::new());
                continue;
            }
            let compressed = &body[start..start + len];
            let expanded_bytes =
                u16::from_le_bytes([compressed[0], compressed[1]]) as usize;
            let carmack = carmack_expand(&compressed[2..], expanded_bytes);
            // First word of the carmack output is the RLEW-expanded size
            // header — see CA_CacheMap (`buffer2seg+1`).  Skip it.
            let rlew_input = if !carmack.is_empty() { &carmack[1..] } else { &[][..] };
            let rlew = rlew_expand(rlew_input, rlew_tag, width * height);
            planes.push(rlew);
        }

        write_map_json(&dir, i, &name, width, height, &planes)?;
        written += 1;
    }

    log::info!("wrote {written} maps to {}", dir.display());
    Ok(())
}

/// Carmack expansion — see `CAL_CarmackExpand` in ID_CA.C.
/// `expanded_bytes` is the size of the output buffer in bytes (length is
/// stored as `bytes / 2` words once we're done).
fn carmack_expand(compressed: &[u8], expanded_bytes: usize) -> Vec<u16> {
    let mut remaining_words = expanded_bytes / 2;
    let mut inptr = 0usize;
    let mut out = Vec::<u16>::with_capacity(remaining_words);

    while remaining_words > 0 && inptr + 1 < compressed.len() {
        let lo = compressed[inptr];
        let hi = compressed[inptr + 1];
        inptr += 2;

        if hi == CARMACK_NEAR_TAG {
            if inptr >= compressed.len() {
                break;
            }
            let next = compressed[inptr];
            inptr += 1;
            if lo == 0 {
                // Escape: emit the tag word as literal data.
                let word = ((hi as u16) << 8) | (next as u16);
                out.push(word);
                remaining_words -= 1;
            } else {
                // Near copy: lo = count, next = back-distance in words.
                let count = lo as usize;
                let mut copyptr = out.len().saturating_sub(next as usize);
                for _ in 0..count {
                    if copyptr < out.len() {
                        let w = out[copyptr];
                        out.push(w);
                    } else {
                        out.push(0);
                    }
                    copyptr += 1;
                }
                remaining_words = remaining_words.saturating_sub(count);
            }
        } else if hi == CARMACK_FAR_TAG {
            if inptr >= compressed.len() {
                break;
            }
            let next = compressed[inptr];
            inptr += 1;
            if lo == 0 {
                let word = ((hi as u16) << 8) | (next as u16);
                out.push(word);
                remaining_words -= 1;
            } else {
                if inptr >= compressed.len() {
                    break;
                }
                let next_hi = compressed[inptr];
                inptr += 1;
                let count = lo as usize;
                let abs = ((next_hi as u16) << 8) | (next as u16);
                let mut copyptr = abs as usize;
                for _ in 0..count {
                    if copyptr < out.len() {
                        let w = out[copyptr];
                        out.push(w);
                    } else {
                        out.push(0);
                    }
                    copyptr += 1;
                }
                remaining_words = remaining_words.saturating_sub(count);
            }
        } else {
            out.push(((hi as u16) << 8) | (lo as u16));
            remaining_words -= 1;
        }
    }

    out
}

/// RLEW expansion — see `CA_RLEWexpand` in ID_CA.C.
/// `tag` matches the rlew_tag from MAPHEAD; whenever we see it, the next
/// two words are (count, value) for a run.  Bounded by `expected_words` —
/// the original is bounded by output length, not input length.
fn rlew_expand(compressed: &[u16], tag: u16, expected_words: usize) -> Vec<u16> {
    let mut out = Vec::with_capacity(expected_words);
    let mut i = 0;
    while out.len() < expected_words && i < compressed.len() {
        let word = compressed[i];
        i += 1;
        if word == tag {
            if i + 1 >= compressed.len() {
                break;
            }
            let count = compressed[i] as usize;
            let value = compressed[i + 1];
            i += 2;
            for _ in 0..count {
                if out.len() >= expected_words {
                    break;
                }
                out.push(value);
            }
        } else {
            out.push(word);
        }
    }
    out
}

fn write_map_json(
    dir: &Path,
    index: usize,
    name: &str,
    width: usize,
    height: usize,
    planes: &[Vec<u16>],
) -> Result<()> {
    let safe_name: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let path = dir.join(format!("{:02}_{}.json", index, safe_name));

    let mut s = String::new();
    s.push_str("{\n");
    s.push_str(&format!("  \"index\": {index},\n"));
    s.push_str(&format!("  \"name\": {},\n", json_string(name)));
    s.push_str(&format!("  \"width\": {width},\n"));
    s.push_str(&format!("  \"height\": {height},\n"));
    s.push_str("  \"planes\": [\n");
    let labels = ["wall", "object"];
    for (p, plane) in planes.iter().enumerate() {
        let comma = if p + 1 < planes.len() { "," } else { "" };
        s.push_str(&format!(
            "    {{ \"label\": \"{}\", \"tiles\": [",
            labels.get(p).copied().unwrap_or("plane")
        ));
        for (j, t) in plane.iter().enumerate() {
            if j > 0 {
                s.push_str(",");
            }
            s.push_str(&t.to_string());
        }
        s.push_str(&format!("] }}{comma}\n"));
    }
    s.push_str("  ]\n}\n");

    fs::write(&path, s).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Quote-escape a string for JSON embedding.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// -----------------------------------------------------------------------------
// main
// -----------------------------------------------------------------------------

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let data_dir = PathBuf::from(DATA_DIR);
    let out_dir = PathBuf::from(OUT_DIR);
    fs::create_dir_all(&out_dir)?;

    log::info!("loading palette");
    let palette = load_game_palette(PALETTE_PATH)?;
    extract_palette(&palette, &out_dir)?;

    log::info!("loading + decompressing VGAGRAPH");
    let graph = load_vga_graph(&data_dir)?;
    extract_pics(&graph, &palette, &out_dir)?;
    extract_fonts(&graph, &out_dir)?;
    // tile8 (chunk 135) is intentionally skipped — see extract_tile8s docs.

    log::info!("loading VSWAP");
    let swap = load_vswap(&data_dir)?;
    extract_walls(&swap, &palette, &out_dir)?;
    extract_sprites(&swap, &palette, &out_dir)?;
    extract_digi(&swap, &out_dir)?;

    log::info!("loading AUDIOT");
    extract_audiot(&data_dir, &out_dir)?;

    log::info!("loading GAMEMAPS");
    extract_maps(&data_dir, &out_dir)?;

    log::info!("done. extracted assets are in {}", out_dir.display());
    Ok(())
}
