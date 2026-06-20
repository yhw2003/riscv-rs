#include <stdint.h>

#define GPIO_OUT ((volatile uint32_t *)0x10000000u)
#define DATA_IN ((volatile uint32_t *)0x00008000u)

void _start(void) __attribute__((noreturn, section(".text.start")));

void _start(void) {
    uint32_t acc = *DATA_IN + 7u;
    uint32_t limit = (*DATA_IN & 0x7u) + 5u;

    for (uint32_t i = 0; i < limit; ++i) {
        acc = acc + i + 3u;
        if ((acc & 1u) == 0u) {
            acc = acc - 2u;
        } else {
            acc = acc + 4u;
        }
        *GPIO_OUT = acc;
    }

    uint32_t countdown = (*DATA_IN & 0x3u) + 3u;
    while (countdown != 0u) {
        acc = acc - countdown;
        if (acc > 20u) {
            *GPIO_OUT = acc - 1u;
        } else {
            *GPIO_OUT = acc + 1u;
        }
        countdown = countdown - 1u;
    }

    *GPIO_OUT = 0x80000000u | (acc & 0xffu);

    for (;;) {
    }
}
