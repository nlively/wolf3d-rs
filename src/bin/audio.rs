
/* ASSIGNMENT
- Read `AUDIOHED.WL6` (chunk offsets)
- Read `AUDIOT.WL6` raw chunks
- Identify and decode PC-speaker and digitized SFX chunks
- Reference: `ID_CA.C::CA_LoadAllSounds`, `ID_SD.C`
*/

use std::fs;
use std::io::{Cursor, Read};

const NUMSOUNDS: usize = 87;
const NUMSNDCHUNKS: usize = 288;

const STARTPCSOUNDS: usize = 0;
const STARTADLIBSOUNDS: usize = 87;
const STARTDIGISOUNDS: usize = 174;
const STARTMUSIC: usize = 261;

struct PCSound {
    length: u32, // number of timer bytes
    priority: u16,
    data: [u8], // one byte per tick: timer divisor for PC speaker port 0x42. 0x00 = silence
}

struct AdLibSound {
    length: u32, // number of data bytes
    priority: u16,
    instrument: Instrument, // 16  bytes of OPL2 register values
    block: u8,
    data: [u8], // note sequence bytes
}

struct Instrument {
    mChar: u8,
    cChar: u8, 
    mScale: u8,
    cScale: u8,
    mAttack: u8,
    cAttack: u8,
    mSus: u8,
    cSus: u8,
    mWave: u8,
    cWave: u8,
    nConn: u8,
    voice: u8,
    mode: u8,
    unused: [u8; 3],
}

struct DigitizedSFX {
    length: u32,
    priority: u16,
    hertz: u16,
    bits: u8,
    reference: u8, // unsigned center
    data: [u8], // raw pcm
}

// imf format - no header struct, just a sequence of the following:
struct MusicIMF {
    reg: u8, // opl2 register
    val: u8, // value to write
    delay: u16, // ticks to wait (at 700hz)
}

enum AudioChunk {
    PCSound(Vec<u8>), // raw timer bytes after the 6-byte header
    AdLib(Vec<u8>), // raw chunk bytes (parse instrument + note data)
    Digitized(Vec<u8>), // raw pcm bytes after the 8-byte header
    Music(Vec<u8>), // raw imf events
}

enum SoundMode {
    Off,
    PC,
    AdLib,
}

enum DigitizedSoundMode {
    Off,
    SoundSource,
    SoundBlaster,
}

enum MusicMode {
    Off,
    AdLib,
}

// copied straight from C
enum SoundName {
		HITWALLSND,              // 0
		SELECTWPNSND,            // 1
		SELECTITEMSND,           // 2
		HEARTBEATSND,            // 3
		MOVEGUN2SND,             // 4
		MOVEGUN1SND,             // 5
		NOWAYSND,                // 6
		NAZIHITPLAYERSND,        // 7
		SCHABBSTHROWSND,         // 8
		PLAYERDEATHSND,          // 9
		DOGDEATHSND,             // 10
		ATKGATLINGSND,           // 11
		GETKEYSND,               // 12
		NOITEMSND,               // 13
		WALK1SND,                // 14
		WALK2SND,                // 15
		TAKEDAMAGESND,           // 16
		GAMEOVERSND,             // 17
		OPENDOORSND,             // 18
		CLOSEDOORSND,            // 19
		DONOTHINGSND,            // 20
		HALTSND,                 // 21
		DEATHSCREAM2SND,         // 22
		ATKKNIFESND,             // 23
		ATKPISTOLSND,            // 24
		DEATHSCREAM3SND,         // 25
		ATKMACHINEGUNSND,        // 26
		HITENEMYSND,             // 27
		SHOOTDOORSND,            // 28
		DEATHSCREAM1SND,         // 29
		GETMACHINESND,           // 30
		GETAMMOSND,              // 31
		SHOOTSND,                // 32
		HEALTH1SND,              // 33
		HEALTH2SND,              // 34
		BONUS1SND,               // 35
		BONUS2SND,               // 36
		BONUS3SND,               // 37
		GETGATLINGSND,           // 38
		ESCPRESSEDSND,           // 39
		LEVELDONESND,            // 40
		DOGBARKSND,              // 41
		ENDBONUS1SND,            // 42
		ENDBONUS2SND,            // 43
		BONUS1UPSND,             // 44
		BONUS4SND,               // 45
		PUSHWALLSND,             // 46
		NOBONUSSND,              // 47
		PERCENT100SND,           // 48
		BOSSACTIVESND,           // 49
		MUTTISND,                // 50
		SCHUTZADSND,             // 51
		AHHHGSND,                // 52
		DIESND,                  // 53
		EVASND,                  // 54
		GUTENTAGSND,             // 55
		LEBENSND,                // 56
		SCHEISTSND,              // 57
		NAZIFIRESND,             // 58
		BOSSFIRESND,             // 59
		SSFIRESND,               // 60
		SLURPIESND,              // 61
		TOT_HUNDSND,             // 62
		MEINGOTTSND,             // 63
		SCHABBSHASND,            // 64
		HITLERHASND,             // 65
		SPIONSND,                // 66
		NEINSOVASSND,            // 67
		DOGATTACKSND,            // 68
		FLAMETHROWERSND,         // 69
		MECHSTEPSND,             // 70
		GOOBSSND,                // 71
		YEAHSND,                 // 72
		DEATHSCREAM4SND,         // 73
		DEATHSCREAM5SND,         // 74
		DEATHSCREAM6SND,         // 75
		DEATHSCREAM7SND,         // 76
		DEATHSCREAM8SND,         // 77
		DEATHSCREAM9SND,         // 78
		DONNERSND,               // 79
		EINESND,                 // 80
		ERLAUBENSND,             // 81
		KEINSND,                 // 82
		MEINSND,                 // 83
		ROSESND,                 // 84
		MISSILEFIRESND,          // 85
		MISSILEHITSND,           // 86
		LASTSOUND
}

fn main () {
    let header_path = "assets/data/AUDIOHED.WL6"; // chunk offsets
    let audio_path = "assets/data/AUDIOT.WL6"; // raw chunks

    let header_bytes = fs::read(header_path).expect("failed to load audio headers");
    let audio_bytes = fs::read(audio_path).expect("failed to load raw audio");
    // let audio_cursor = Cursor::new(audio_bytes);

    // header_bytes needs to become an array of 32-bit offsets.
    let offsets: Vec<u32> = header_bytes.chunks_exact(4)
        .map(|chunk| {
            u32::from_le_bytes(chunk.try_into().unwrap())
        })   
        .collect();

    let mut audio_chunks: Vec<AudioChunk> = Vec::<AudioChunk>::new();

    for (i, pair) in offsets.windows(2).enumerate() {
        let start = pair[0] as usize;
        let end = pair[1] as usize;

        // audio_cursor.set_position(start as u64);
        let compressed_chunk: Vec<u8> = audio_bytes[start..end].to_vec();
        let chunk = match i {
            STARTPCSOUNDS..STARTADLIBSOUNDS => AudioChunk::PCSound(compressed_chunk),
            STARTADLIBSOUNDS..STARTDIGISOUNDS => AudioChunk::AdLib(compressed_chunk),
            STARTDIGISOUNDS..STARTMUSIC => AudioChunk::Digitized(compressed_chunk),
            STARTMUSIC..NUMSNDCHUNKS => AudioChunk::Music(compressed_chunk),
            _ => unreachable!(),
        };

        audio_chunks.push(chunk);
    }
}
