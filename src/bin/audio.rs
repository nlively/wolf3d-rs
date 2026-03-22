
/* ASSIGNMENT
- Read `AUDIOHED.WL6` (chunk offsets)
- Read `AUDIOT.WL6` raw chunks
- Identify and decode PC-speaker and digitized SFX chunks
- Reference: `ID_CA.C::CA_LoadAllSounds`, `ID_SD.C`
*/

use std::fs;
use std::io::{Cursor, Read};
use anyhow::Error;
use byteorder::LE;
use byteorder::ReadBytesExt;

const NUMSOUNDS: usize = 87;
const NUMSNDCHUNKS: usize = 288;

const STARTPCSOUNDS: usize = 0;
const STARTADLIBSOUNDS: usize = 87;
const STARTDIGISOUNDS: usize = 174;
const STARTMUSIC: usize = 261;

struct PCSound {
    length: u32, // number of timer bytes
    priority: u16,
    data: Vec<u8>, // one byte per tick: timer divisor for PC speaker port 0x42. 0x00 = silence
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

struct DigitizedSFX {
    length: u32,
    priority: u16,
    // hertz: u16,
    // bits: u8,
    // reference: u8, // unsigned center
    data: Vec<u8>, // raw pcm
}

fn extract_digitized_sfx(chunk: &Vec<u8>) -> Result<DigitizedSFX, Error> {

    let mut cursor = Cursor::new(chunk);

    // extract length (4 bytes)
    // let length = cursor.read_u32::<LE>()?;
    // println!("extracting length {}", length);

    // extract priority (2 bytes)
    // let priority = cursor.read_u16::<LE>()?;
    // println!("extracting priority {}", priority);

    // hertz priority (2 bytes)
    // let hertz = cursor.read_u16::<LE>()?;

    // extract bits (1 byte)
    // let bits = cursor.read_u8()?;

    // extract reference (1 byte)
    // let reference = cursor.read_u8()?;

    // extract data (`length` bytes?)
    // let mut data = vec![0u8; length as usize];
    // cursor.read_exact(&mut data)?;

    println!("extracting digitized sfx chunk of {} bytes", chunk.len());

    let output = DigitizedSFX { 
        length: chunk.len() as u32, 
        priority: 0, 
        // hertz, 
        // bits, 
        // reference, 
        data: chunk.to_vec(),
    };

    Ok(output)
}

fn digitized_to_wav(digitized: DigitizedSFX) -> Vec<u8> {
    /*
    A WAV file is just a RIFF container with two chunks prepended before the raw PCM. You write it by hand — 
    no crate needed:                                                                                       

    1. RIFF header (12 bytes)                                                                                
    "RIFF"                        // 4 bytes
    <total file size - 8> as u32 LE  // 4 bytes  (= 28 + length)                                             
    "WAVE"                        // 4 bytes                                                                 
                                                                                                            
    2. fmt  chunk (24 bytes)                                                                                 
    "fmt "          // 4 bytes                                                                               
    16 as u32 LE    // 4 bytes  (chunk size, always 16 for PCM)
    1 as u16 LE     // 2 bytes  (audio format: 1 = PCM)                                                      
    1 as u16 LE     // 2 bytes  (num channels: mono)                                                         
    hertz as u32 LE // 4 bytes  (sample rate)                                                                
    hertz as u32 LE // 4 bytes  (byte rate = sample_rate * channels * bits/8, which for 8-bit mono = hertz)  
    1 as u16 LE     // 2 bytes  (block align = channels * bits/8)                                            
    bits as u16 LE  // 2 bytes  (bits per sample)                                                            
                                                
    3. data chunk                                                                                            
    "data"            // 4 bytes                                                                             
    length as u32 LE  // 4 bytes
    <data bytes>      // length bytes, copied verbatim from DigitizedSFX.data                                
     */

    // RIFF header
    let mut output = Vec::<u8>::new();
    output.extend_from_slice("RIFF".as_bytes());
    let header_len: u32 = digitized.length + 28;
    output.extend(header_len.to_le_bytes());
    output.extend_from_slice("WAVE".as_bytes());

    let hertz = 7000;
    let bits: u32 = 8;
    let channels: u16 = 1; // mono
    let byte_rate = hertz * (channels as u32) * (bits / 8);
    let block_align = channels * (bits as u16 / 8);

    // FMT chunk
    output.extend_from_slice("fmt ".as_bytes());
    output.extend((16 as u32).to_le_bytes()); // (chunk size, always 16 for PCM)
    output.extend((1 as u16).to_le_bytes());  // audio format: 1 = PCM
    output.extend(channels.to_le_bytes());  // num channels: mono
    output.extend(hertz.to_le_bytes()); // sample rate
    output.extend((byte_rate).to_le_bytes()); // byte rate = sample_rate * channels * bits/8, which for 8-bit mono = hertz
    output.extend(block_align.to_le_bytes());  // block align = channels * bits/8
    output.extend((bits as u16).to_le_bytes()); // bits per sample
    
    // data chunk
    output.extend_from_slice("data".as_bytes());
    output.extend((digitized.length as u32).to_le_bytes()); 
    output.extend(digitized.data);

    output
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

    for (i, chunk) in audio_chunks[STARTPCSOUNDS..STARTADLIBSOUNDS].iter().enumerate() {
        if let AudioChunk::PCSound(data) = chunk {
            let extracted = extract_pc_sound(data);
            println!("length = {}", extracted.length);
            println!("data.len() = {}", extracted.data.len());
        }
    }  

}

fn extract_pc_sound(chunk: &Vec<u8>) -> PCSound {
    let mut cursor = Cursor::new(chunk);
    let length = cursor.read_u32::<LE>().unwrap();
    let priority = cursor.read_u16::<LE>().unwrap();
    let mut data = vec![0u8; length as usize];
    cursor.read_exact(&mut data).unwrap();
    PCSound { length, priority, data }
}