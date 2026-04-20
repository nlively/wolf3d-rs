# Notes about C Reference Code

The following table translates C to Rust data types, with regard to the C compilers that would have been used in the time Wolfenstein 3D was created.

- `word` => `u16` (`word` is a typedef for `unsigned int`)
- `long` => `u32` or `i32`
- `longword` => `u32` (`longword` is a typedef for `unsigned long`)
- `char` => `u8`
- `byte` => `u8` (`byte` is a typedef for `unsigned char`)
- `unsigned` => `u16`
- `int` => `i16`


Byte sequences use little endian order.