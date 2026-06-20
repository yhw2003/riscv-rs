#include <stdint.h>

#define GPIO_OUT ((volatile uint32_t *)0x10000000u)

void _start(void) __attribute__((noreturn));

void _start(void) {
    *GPIO_OUT = 0x00000005u;
    *GPIO_OUT = 0x0000000au;
    *GPIO_OUT = 0x0000003cu;
    *GPIO_OUT = 0x80000081u;

    for (;;) {
    }
}
