#include "testlib.h"

u8 CHACHA_GLOBAL_STATE[64] = {};
u8 CHACHA_GLOBAL_KEY[32] = {};
u8 CHACHA_GLOBAL_NONCE[12] = {};

const u32 SIGMA[4] = {
    0x61707865, 0x3320646e, 0x79622d32, 0x6b206574
};

u32  read32_le(u8* bytes, int offset) {
    return ((u32)(bytes[offset    ]))   |
    (((u32)(bytes[offset + 1])) <<  8)  |
    (((u32)(bytes[offset + 2])) << 16)  |
    (((u32)(bytes[offset + 3])) << 24)  ;
}

#define DISPLAY_BYTES(arr, len) \
    signal(64, (((u32)(arr)) & 0x00FFFFFF) | (((len) & 0xFF) << 24))
#define DISPLAY_INTS(arr, len) \
    signal(65, (((u32)(arr)) & 0x00FFFFFF) | (((len) & 0xFF) << 24))

void chacha20_init(u32* state, u8* key, u8* nonce);
void chacha20_block(u32* state, u8* dest_bytes);

void main() {
    u32* chacha_state = (u32*)CHACHA_GLOBAL_STATE;
    u8* chacha_key = CHACHA_GLOBAL_KEY;
    u8* chacha_nonce = CHACHA_GLOBAL_NONCE;

    // allow the exception handler to do something with these.
    signal(0, (u32) chacha_key);
    signal(1, (u32) chacha_nonce);
    u32 iterations = signal(2, 0);

    chacha20_init(chacha_state, chacha_key, chacha_nonce);

    u8 dest[64];
    signal(3, (u32)(&dest[0]));

    for (u32 iter = 0; iter < iterations; iter++) {
        chacha20_block(chacha_state, dest);
    }
    signal(3, (u32)(&dest[0]));

    halt();
}

void chacha20_init(u32* state, u8* key, u8* nonce) {
    // ChaCha20 Constants
    state[0] = SIGMA[0];
    state[1] = SIGMA[1];
    state[2] = SIGMA[2];
    state[3] = SIGMA[3];

    // Key
    state[ 4] = read32_le(key,  0);
    state[ 5] = read32_le(key,  4);
    state[ 6] = read32_le(key,  8);
    state[ 7] = read32_le(key, 12);
    state[ 8] = read32_le(key, 16);
    state[ 9] = read32_le(key, 20);
    state[10] = read32_le(key, 24);
    state[11] = read32_le(key, 28);

    // Block Counter
    state[12] = 1;

    // Nonce
    state[13] = read32_le(nonce, 0);
    state[14] = read32_le(nonce, 4);
    state[15] = read32_le(nonce, 8);
}

void quarter_round(u32* state, int a, int b, int c, int d) {
    // 1.  a += b; d ^= a; d <<<= 16;
    // 2.  c += d; b ^= c; b <<<= 12;
    // 3.  a += b; d ^= a; d <<<= 8;
    // 4.  c += d; b ^= c; b <<<= 7;

    state[a] += state[b]; state[d] ^= state[a]; state[d] = ROTATE_LEFT(state[d], 16);
    state[c] += state[d]; state[b] ^= state[c]; state[b] = ROTATE_LEFT(state[b], 12);
    state[a] += state[b]; state[d] ^= state[a]; state[d] = ROTATE_LEFT(state[d],  8);
    state[c] += state[d]; state[b] ^= state[c]; state[b] = ROTATE_LEFT(state[b],  7);
}

void chacha20_block(u32* state, u8* dest) {
    u32* working_state = (u32*) dest;

    for (int idx = 0; idx < 16; idx += 1) {
        working_state[idx] = state[idx];
    }

    for (int iter = 0; iter < 10; iter++) {
        quarter_round(working_state, 0, 4,  8, 12);
        quarter_round(working_state, 1, 5,  9, 13);
        quarter_round(working_state, 2, 6, 10, 14);
        quarter_round(working_state, 3, 7, 11, 15);
        quarter_round(working_state, 0, 5, 10, 15);
        quarter_round(working_state, 1, 6, 11, 12);
        quarter_round(working_state, 2, 7,  8, 13);
        quarter_round(working_state, 3, 4,  9, 14);
    }

    for (int idx = 0; idx < 16; idx++) {
        working_state[idx] += state[idx];
    }

    state[12] += 1;

    // should do a byte swap here on all u32s in working_state
    // if we're in big endian but I already know I'm running this
    // on a LITTLE ENDIAN "machine", so...
}
