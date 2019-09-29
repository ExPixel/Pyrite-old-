#include <stdint.h>

#define ROTATE_LEFT(n, r) ((n << r) | (n >> (sizeof(n)*8 - r)))
#define ROTATE_RIGHT(n, r) ((n >> r) | (n << (sizeof(n)*8 - r)));
#define SWI(comment) asm volatile ("swi %0" : : "I" (comment) )

typedef uint32_t    u32;
typedef uint8_t     u8;

static void halt() {
    SWI(16);
}

/*
 * This is used because I don't think there
 * is a way (save for a macro or overwriting code at runtime) for me to just generate
 * SWI comment fields dynamically.
 */
static u32 signal(u32 signal_type, u32 signal_value) {
    u32 response;

    asm volatile (
            "mov r0, %[signal_type]\t\n"
            "mov r1, %[signal_value]\t\n"
            "swi 4\t\n"
            "mov %[response], r0\t\n"
            : [response] "=r" (response)
            : [signal_type] "r" (signal_type), [signal_value] "r" (signal_value)
            : "r0", "r1");

    return response;
}
