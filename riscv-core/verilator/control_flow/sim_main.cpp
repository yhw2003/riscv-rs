#include "Vrv32i_soc.h"
#include "verilated.h"

#include <array>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <string>
#include <vector>

namespace {

constexpr uint32_t expected_gpio[] = {
    0x00000008u,
    0x0000000au,
    0x00000013u,
    0x0000001du,
    0x00000022u,
    0x0000001eu,
    0x0000001cu,
    0x0000001bu,
    0x8000001cu,
};

uint32_t load32(const std::vector<uint8_t>& mem, uint32_t addr) {
    if (addr + 3 >= mem.size()) {
        return 0;
    }
    return static_cast<uint32_t>(mem[addr]) |
           (static_cast<uint32_t>(mem[addr + 1]) << 8) |
           (static_cast<uint32_t>(mem[addr + 2]) << 16) |
           (static_cast<uint32_t>(mem[addr + 3]) << 24);
}

void store32(std::vector<uint8_t>& mem, uint32_t addr, uint32_t data, uint8_t wstrb) {
    if (addr + 3 >= mem.size()) {
        return;
    }
    for (int lane = 0; lane < 4; ++lane) {
        if ((wstrb & (1u << lane)) != 0) {
            mem[addr + lane] = static_cast<uint8_t>((data >> (lane * 8)) & 0xffu);
        }
    }
}

uint32_t get_bits(const VlWide<9>& words, int lsb, int width) {
    uint32_t out = 0;
    for (int bit = 0; bit < width; ++bit) {
        const int src = lsb + bit;
        const uint32_t value = (words[src / 32] >> (src % 32)) & 1u;
        out |= value << bit;
    }
    return out;
}

struct MemReq {
    bool valid;
    bool is_write;
    uint32_t addr;
    uint32_t wdata;
    uint8_t wstrb;
};

struct SocOut {
    uint32_t imem_addr;
    MemReq dmem_req;
    uint32_t gpio_pins;
    bool trap;
};

SocOut decode_output(const Vrv32i_soc& top) {
    SocOut out{};
    out.imem_addr = get_bits(top.o, 0, 32);
    out.dmem_req.valid = get_bits(top.o, 32, 1) != 0;
    out.dmem_req.is_write = get_bits(top.o, 33, 1) != 0;
    out.dmem_req.addr = get_bits(top.o, 34, 32);
    out.dmem_req.wdata = get_bits(top.o, 66, 32);
    out.dmem_req.wstrb = static_cast<uint8_t>(get_bits(top.o, 98, 4));
    out.gpio_pins = get_bits(top.o, 102, 32);
    out.trap = get_bits(top.o, 270, 1) != 0;
    return out;
}

void eval(Vrv32i_soc& top, bool clock, bool reset, uint32_t inst, uint32_t dmem_rdata) {
    top.clock_reset = (clock ? 1 : 0) | (reset ? 2 : 0);
    top.i = (static_cast<uint64_t>(dmem_rdata) << 32) | inst;
    top.eval();
}

std::vector<uint8_t> read_binary(const char* path) {
    std::ifstream input(path, std::ios::binary);
    if (!input) {
        std::fprintf(stderr, "failed to open firmware binary: %s\n", path);
        std::exit(2);
    }
    return std::vector<uint8_t>(
        std::istreambuf_iterator<char>(input),
        std::istreambuf_iterator<char>());
}

} // namespace

int main(int argc, char** argv) {
    Verilated::commandArgs(argc, argv);
    if (argc != 2) {
        std::fprintf(stderr, "usage: %s firmware.bin\n", argv[0]);
        return 2;
    }

    std::vector<uint8_t> mem(64 * 1024, 0);
    const auto firmware = read_binary(argv[1]);
    if (firmware.size() > mem.size()) {
        std::fprintf(stderr, "firmware too large: %zu bytes\n", firmware.size());
        return 2;
    }
    std::copy(firmware.begin(), firmware.end(), mem.begin());

    Vrv32i_soc top;
    uint32_t inst = 0;
    uint32_t rdata = 0;

    eval(top, false, true, inst, rdata);
    eval(top, true, true, inst, rdata);
    eval(top, false, false, inst, rdata);

    size_t observed = 0;
    uint32_t last_gpio = decode_output(top).gpio_pins;

    for (int cycle = 0; cycle < 240; ++cycle) {
        SocOut out{};
        for (int settle = 0; settle < 4; ++settle) {
            const uint32_t pc = out.imem_addr;
            inst = load32(mem, pc);
            rdata = (out.dmem_req.valid && !out.dmem_req.is_write) ? load32(mem, out.dmem_req.addr) : 0;
            eval(top, false, false, inst, rdata);
            out = decode_output(top);
        }

        if (out.trap) {
            std::fprintf(stderr, "unexpected trap at cycle %d pc=0x%08x\n", cycle, out.imem_addr);
            return 1;
        }

        if (out.gpio_pins != last_gpio) {
            if (observed >= std::size(expected_gpio)) {
                std::fprintf(stderr, "unexpected extra gpio value 0x%08x\n", out.gpio_pins);
                return 1;
            }
            if (out.gpio_pins != expected_gpio[observed]) {
                std::fprintf(
                    stderr,
                    "gpio mismatch at index %zu: got 0x%08x expected 0x%08x\n",
                    observed,
                    out.gpio_pins,
                    expected_gpio[observed]);
                return 1;
            }
            last_gpio = out.gpio_pins;
            ++observed;
        }

        if (out.dmem_req.valid && out.dmem_req.is_write) {
            store32(mem, out.dmem_req.addr, out.dmem_req.wdata, out.dmem_req.wstrb);
        }

        eval(top, true, false, inst, rdata);
        eval(top, false, false, inst, rdata);

        if (observed == std::size(expected_gpio)) {
            return 0;
        }
    }

    std::fprintf(stderr, "only observed %zu gpio writes\n", observed);
    return 1;
}
