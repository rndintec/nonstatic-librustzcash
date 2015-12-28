const KECCAKF_RNDC: [u64; 24] = 
[
    0x0000000000000001, 0x0000000000008082, 0x800000000000808a,
    0x8000000080008000, 0x000000000000808b, 0x0000000080000001,
    0x8000000080008081, 0x8000000000008009, 0x000000000000008a,
    0x0000000000000088, 0x0000000080008009, 0x000000008000000a,
    0x000000008000808b, 0x800000000000008b, 0x8000000000008089,
    0x8000000000008003, 0x8000000000008002, 0x8000000000000080, 
    0x000000000000800a, 0x800000008000000a, 0x8000000080008081,
    0x8000000000008080, 0x0000000080000001, 0x8000000080008008
];

const KECCAKF_ROTC: [usize; 24] = 
[
    1,  3,  6,  10, 15, 21, 28, 36, 45, 55, 2,  14, 
    27, 41, 56, 8,  25, 43, 62, 18, 39, 61, 20, 44
];

const KECCAKF_PILN: [usize; 24] = 
[
    10, 7,  11, 17, 18, 3, 5,  16, 8,  21, 24, 4, 
    15, 23, 19, 13, 12, 2, 20, 14, 22, 9,  6,  1 
];

fn keccakf(st: &mut [Chunk], rounds: usize)
{
    assert_eq!(st.len(), 25);
    for round in 0..rounds {
        /*
        // Theta
        for (i = 0; i < 5; i++)     
            bc[i] = st[i] ^ st[i + 5] ^ st[i + 10] ^ st[i + 15] ^ st[i + 20];
        */

        let mut bc: Vec<Chunk> = (0..5).map(|i| st[i]
                                                .xor(&st[i+5])
                                                .xor(&st[i+10])
                                                .xor(&st[i+15])
                                                .xor(&st[i+20])
                                           ).collect();

        /*
        for (i = 0; i < 5; i++) {
            t = bc[(i + 4) % 5] ^ ROTL64(bc[(i + 1) % 5], 1);
            for (j = 0; j < 25; j += 5)
                st[j + i] ^= t;
        }
        */

        for i in 0..5 {
            let tmp = bc[(i + 4) % 5].xor(&bc[(i + 1) % 5].rotl(1));

            for j in (0..25).filter(|a| a % 5 == 0) {
                st[j + i] = tmp.xor(&st[j + i]);
            }
        }

        {
            /*
            // Rho Pi
            t = st[1];
            for (i = 0; i < 24; i++) {
                j = keccakf_piln[i];
                bc[0] = st[j];
                st[j] = ROTL64(t, keccakf_rotc[i]);
                t = bc[0];
            }
            */
            let mut tmp = st[1].clone();

            for i in 0..24 {
                let j = KECCAKF_PILN[i];

                bc[0] = st[j].clone();
                st[j] = tmp.rotl(KECCAKF_ROTC[i]);
                tmp = bc[0].clone();
            }
        }

        {
            /*
            //  Chi
            for (j = 0; j < 25; j += 5) {
                for (i = 0; i < 5; i++)
                    bc[i] = st[j + i];
                for (i = 0; i < 5; i++)
                    st[j + i] ^= (~bc[(i + 1) % 5]) & bc[(i + 2) % 5];
            }
            */

            for j in (0..25).filter(|a| a % 5 == 0) {
                for i in 0..5 {
                    bc[i] = st[j + i].clone();
                }

                for i in 0..5 {
                    st[j + i] = st[j + i].xor(&bc[(i + 1) % 5].notand(&bc[(i + 2) % 5]));
                }
            }
        }

        /*
        //  Iota
        st[0] ^= keccakf_rndc[round];
        */

        st[0] = st[0].xor(&KECCAKF_RNDC[round].into());
    }
}

fn sha3_256(message: &[Byte]) -> Vec<Byte> {
    // As defined by FIPS202
    keccak(1088, 512, message, 0x06, 32)
}

fn keccak(rate: usize, capacity: usize, mut input: &[Byte], delimited_suffix: u8, mut mdlen: usize)
    -> Vec<Byte>
{
    use std::cmp::min;

    let mut st: Vec<Byte> = Some(Bit::byte(0)).into_iter().cycle().take(200).collect();

    let rateInBytes = rate / 8;
    let mut inputByteLen = input.len();
    let mut blockSize = 0;

    if ((rate + capacity) != 1600) || ((rate % 8) != 0) {
        panic!("invalid parameters");
    }

    while inputByteLen > 0 {
        blockSize = min(inputByteLen, rateInBytes);

        for i in 0..blockSize {
            st[i] = st[i].xor(&input[i]);
        }

        input = &input[blockSize..];
        inputByteLen -= blockSize;

        if blockSize == rateInBytes {
            temporary_shim(&mut st);
            blockSize = 0;
        }
    }

    st[blockSize] = st[blockSize].xor(&Bit::byte(delimited_suffix));

    if ((delimited_suffix & 0x80) != 0) && (blockSize == (rateInBytes-1)) {
        temporary_shim(&mut st);
    }

    st[rateInBytes-1] = st[rateInBytes-1].xor(&Bit::byte(0x80));

    temporary_shim(&mut st);

    let mut output = Vec::with_capacity(mdlen);

    while mdlen > 0 {
        blockSize = min(mdlen, rateInBytes);
        output.extend_from_slice(&st[0..blockSize]);
        mdlen -= blockSize;

        if mdlen > 0 {
            temporary_shim(&mut st);
        }
    }

    output
}

fn temporary_shim(state: &mut [Byte]) {
    assert_eq!(state.len(), 200);

    println!("RUNNING TEMPORARY SHIM!");

    let mut chunks = Vec::with_capacity(25);
    for i in 0..25 {
        chunks.push(Chunk::from(0x0000000000000000));
    }

    for (chunk_bit, input_bit) in chunks.iter_mut().flat_map(|c| c.bits.iter_mut())
                                        //.zip(state.iter().flat_map(|c| c.bits.iter()))
                                        .zip(state.chunks(8).flat_map(|e| e.iter().rev()).flat_map(|c| c.bits.iter()))
    {
        *chunk_bit = input_bit.clone();
    }

    keccakf(&mut chunks, 24);

    for (chunk_bit, input_bit) in chunks.iter().flat_map(|c| c.bits.iter())
                                        .zip(state.chunks_mut(8).flat_map(|e| e.iter_mut().rev()).flat_map(|c| c.bits.iter_mut()))
    {
        *input_bit = chunk_bit.clone();
    }
}

#[derive(Clone)]
struct Chunk {
    bits: Vec<Bit>
}

impl Chunk {
    fn xor(&self, other: &Chunk) -> Chunk {
        Chunk {
            bits: self.bits.iter()
                           .zip(other.bits.iter())
                           .map(|(a, b)| a.xor(b))
                           .collect()
        }
    }

    fn notand(&self, other: &Chunk) -> Chunk {
        Chunk {
            bits: self.bits.iter()
                           .zip(other.bits.iter())
                           .map(|(a, b)| a.notand(b))
                           .collect()
        }
    }

    fn rotl(&self, mut by: usize) -> Chunk {
        by = by % 64;

        Chunk {
            bits: self.bits[by..].iter()
                                 .chain(self.bits[0..by].iter())
                                 .cloned()
                                 .collect()
        }
    }
}

impl PartialEq for Chunk {
    fn eq(&self, other: &Chunk) -> bool {
        for (a, b) in self.bits.iter().zip(other.bits.iter()) {
            if a != b { return false; }
        }

        true
    }
}

impl<'a> From<&'a [Byte]> for Chunk {
    fn from(bytes: &'a [Byte]) -> Chunk {
        assert!(bytes.len() == 8); // must be 64 bit

        Chunk {
            bits: bytes.iter().rev() // endianness
                       .flat_map(|x| x.bits.iter())
                       .cloned()
                       .collect()
        }
    }
}

impl<'a> From<&'a [Bit]> for Chunk {
    fn from(bits: &'a [Bit]) -> Chunk {
        assert!(bits.len() == 64); // must be 64 bit

        Chunk {
            bits: bits.iter().cloned().collect()
        }
    }
}

impl From<u64> for Chunk {
    fn from(num: u64) -> Chunk {
        fn bit_at(num: u64, i: usize) -> u8 {
            ((num << i) >> 63) as u8
        }

        Chunk {
            bits: (0..64).map(|i| Bit::constant(bit_at(num, i))).collect()
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
enum Bit {
    Constant(u8)
}

#[derive(Clone, Debug, PartialEq)]
struct Byte {
    bits: Vec<Bit>
}

impl Byte {
    fn grab(&self) -> u8 {
        let mut cur = 7;
        let mut acc = 0;

        for bit in &self.bits {
            if let &Bit::Constant(1) = bit {
                acc |= 0b00000001 << cur;
            }
            cur -= 1;
        }

        acc
    }

    fn xor(&self, other: &Byte) -> Byte {
        Byte {
            bits: self.bits.iter()
                           .zip(other.bits.iter())
                           .map(|(a, b)| a.xor(b))
                           .collect()
        }
    }
}

impl Bit {
    fn byte(byte: u8) -> Byte {
        Byte {
            bits: (0..8).map(|i| byte & (0b00000001 << i) != 0)
                        .map(|b| Bit::constant(if b { 1 } else { 0 }))
                        .rev()
                        .collect()
        }
    }

    fn constant(num: u8) -> Bit {
        assert_eq!((1 - num) * num, 0); // haha

        Bit::Constant(num)
    }

    // self xor other
    fn xor(&self, other: &Bit) -> Bit {
        match (self, other) {
            (&Bit::Constant(a), &Bit::Constant(b)) => {
                Bit::constant(a ^ b)
            },
            //_ => unimplemented!()
        }
    }

    // (not self) and other
    fn notand(&self, other: &Bit) -> Bit {
        match (self, other) {
            (&Bit::Constant(a), &Bit::Constant(b)) => {
                Bit::constant((a ^ 1) & b)
            },
            //_ => unimplemented!()
        }
    }
}

#[test]
fn test_sha3_256() {
    let test_vector: Vec<(Vec<Byte>, [u8; 32])> = vec![
        (vec![Bit::byte(0x30)],
         [0xf9,0xe2,0xea,0xaa,0x42,0xd9,0xfe,0x9e,0x55,0x8a,0x9b,0x8e,0xf1,0xbf,0x36,0x6f,0x19,0x0a,0xac,0xaa,0x83,0xba,0xd2,0x64,0x1e,0xe1,0x06,0xe9,0x04,0x10,0x96,0xe4]
        ),
        (vec![Bit::byte(0x30),Bit::byte(0x30)],
         [0x2e,0x16,0xaa,0xb4,0x83,0xcb,0x95,0x57,0x7c,0x50,0xd3,0x8c,0x8d,0x0d,0x70,0x40,0xf4,0x67,0x26,0x83,0x23,0x84,0x46,0xc9,0x90,0xba,0xbb,0xca,0x5a,0xe1,0x33,0xc8]
        ),
        ((0..64).map(|_| Bit::byte(0x30)).collect::<Vec<_>>(),
         [0xc6,0xfd,0xd7,0xa7,0xf7,0x08,0x62,0xb3,0x6a,0x26,0xcc,0xd1,0x47,0x52,0x26,0x80,0x61,0xe9,0x81,0x03,0x29,0x9b,0x28,0xfe,0x77,0x63,0xbd,0x96,0x29,0x92,0x6f,0x4b]
        ),
        ((0..128).map(|_| Bit::byte(0x30)).collect::<Vec<_>>(),
         [0x99,0x9d,0xb4,0xd4,0x28,0x7b,0x52,0x15,0x20,0x8d,0x11,0xe4,0x0a,0x27,0xca,0x54,0xac,0xa0,0x09,0xb2,0x5c,0x4f,0x7a,0xb9,0x1a,0xd8,0xaa,0x93,0x60,0xf0,0x63,0x71]
        ),
        ((0..256).map(|_| Bit::byte(0x30)).collect::<Vec<_>>(),
         [0x11,0xea,0x74,0x37,0x7b,0x74,0xf1,0x53,0x9f,0x2e,0xd9,0x0a,0xb8,0xca,0x9e,0xb1,0xe0,0x70,0x8a,0x4b,0xfb,0xad,0x4e,0x81,0xcc,0x77,0xd9,0xa1,0x61,0x9a,0x10,0xdb]
        ),
        ((0..512).map(|_| Bit::byte(0x30)).collect::<Vec<_>>(),
         [0x1c,0x80,0x1b,0x16,0x3a,0x2a,0xbe,0xd0,0xe8,0x07,0x1e,0x7f,0xf2,0x60,0x4e,0x98,0x11,0x22,0x80,0x54,0x14,0xf3,0xc8,0xfd,0x96,0x59,0x5d,0x7e,0xe1,0xd6,0x54,0xe2]
        ),
    ];

    for (i, &(ref message, ref expected)) in test_vector.iter().enumerate() {
        let result: Vec<u8> = sha3_256(message).into_iter().map(|a| a.grab()).collect();

        if &*result != expected {
            print!("Expected: ");
            for i in result.iter() {
                print!("0x{:02x},", i);
            }
            panic!("Hash {} failed!", i+1);
        } else {
            println!("--- HASH {} SUCCESS ---", i+1);
        }
    }
}

#[test]
fn test_keccakf() {
    let base = Chunk::from(0xABCDEF0123456789);

    let mut a: Vec<Chunk> = (0..25).map(|i| base.rotl(i*4)).collect();

    keccakf(&mut a, 24);

    const TEST_VECTOR: [u64; 25] = [
        0x4c8948fcb6616044,
        0x75642a21f8bd1299,
        0xb2e949825ace668e,
        0x9b73a04c53826c35,
        0x914989b8d38ea4d1,
        0xdc73480ade4e2664,
        0x931394137c6fbd69,
        0x234fa173896019f5,
        0x906da29a7796b157,
        0x7666ebe222445610,
        0x41d77796738c884e,
        0x8861db16234437fa,
        0xf07cb925b71f27f2,
        0xfec25b4810a2202c,
        0xa8ba9bbfa9076b54,
        0x18d9b9e748d655b9,
        0xa2172c0059955be6,
        0xea602c863b7947b8,
        0xc77f9f23851bc2bd,
        0x0e8ab0a29b3fef79,
        0xfd73c2cd3b443de4,
        0x447892bf2c03c2ef,
        0xd5b3dae382c238b1,
        0x2103d8a64e9f4cb6,
        0xfe1f57d88e2de92f
    ];

    for i in 0..25 {
        assert!(a[i] == Chunk::from(TEST_VECTOR[i]));
    }
}