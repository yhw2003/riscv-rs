#include "Vrv32i_bram_soc.h"
#include "verilated.h"

#include <array>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>

namespace {

constexpr uint32_t gpio_expected[] = {
    0x00000005u,
    0x0000000au,
    0x0000003cu,
    0x80000081u,
};

constexpr uint32_t control_flow_expected[] = {
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
    bool trace_valid;
    uint32_t trace_pc;
    uint32_t trace_inst;
    uint32_t trace_rd;
    bool trace_rd_write;
    uint32_t trace_rd_wdata;
    uint32_t trace_next_pc;
    bool trace_trap;
    bool trap;
};

SocOut decode_output(const Vrv32i_bram_soc& top) {
    SocOut out{};
    out.imem_addr = get_bits(top.o, 0, 32);
    out.dmem_req.valid = get_bits(top.o, 32, 1) != 0;
    out.dmem_req.is_write = get_bits(top.o, 33, 1) != 0;
    out.dmem_req.addr = get_bits(top.o, 34, 32);
    out.dmem_req.wdata = get_bits(top.o, 66, 32);
    out.dmem_req.wstrb = static_cast<uint8_t>(get_bits(top.o, 98, 4));
    out.gpio_pins = get_bits(top.o, 102, 32);
    out.trace_valid = get_bits(top.o, 134, 1) != 0;
    out.trace_pc = get_bits(top.o, 135, 32);
    out.trace_inst = get_bits(top.o, 167, 32);
    out.trace_rd = get_bits(top.o, 199, 5);
    out.trace_rd_write = get_bits(top.o, 204, 1) != 0;
    out.trace_rd_wdata = get_bits(top.o, 205, 32);
    out.trace_next_pc = get_bits(top.o, 237, 32);
    out.trace_trap = get_bits(top.o, 269, 1) != 0;
    out.trap = get_bits(top.o, 270, 1) != 0;
    return out;
}

void eval(Vrv32i_bram_soc& top, bool clock, bool reset) {
    top.clock_reset = (clock ? 1 : 0) | (reset ? 2 : 0);
    top.eval();
}

struct Expected {
    const uint32_t* values;
    size_t len;
    int max_cycles;
};

Expected expected_for(const char* name) {
    if (std::strcmp(name, "gpio") == 0) {
        return {gpio_expected, std::size(gpio_expected), 120};
    }
    if (std::strcmp(name, "control_flow") == 0) {
        return {control_flow_expected, std::size(control_flow_expected), 360};
    }
    std::fprintf(stderr, "unknown scenario: %s\n", name);
    std::exit(2);
}

} // namespace

int main(int argc, char** argv) {
    Verilated::commandArgs(argc, argv);
    if (argc != 2) {
        std::fprintf(stderr, "usage: %s <gpio|control_flow>\n", argv[0]);
        return 2;
    }

    const Expected expected = expected_for(argv[1]);
    Vrv32i_bram_soc top;

    eval(top, false, true);
    eval(top, true, true);
    eval(top, false, false);

    size_t observed = 0;
    uint32_t last_gpio = decode_output(top).gpio_pins;
    const bool trace = std::getenv("BRAM_SOC_TRACE") != nullptr;

    for (int cycle = 0; cycle < expected.max_cycles; ++cycle) {
        eval(top, false, false);
        const SocOut out = decode_output(top);
        if (trace) {
            std::fprintf(
                stderr,
                "cycle=%03d imem=0x%08x gpio=0x%08x trace_valid=%u pc=0x%08x inst=0x%08x rd=%u rdw=%u rddata=0x%08x next=0x%08x dmem_valid=%u dmem_write=%u dmem_addr=0x%08x dmem_wdata=0x%08x wstrb=0x%x trap=%u\n",
                cycle,
                out.imem_addr,
                out.gpio_pins,
                out.trace_valid ? 1u : 0u,
                out.trace_pc,
                out.trace_inst,
                out.trace_rd,
                out.trace_rd_write ? 1u : 0u,
                out.trace_rd_wdata,
                out.trace_next_pc,
                out.dmem_req.valid ? 1u : 0u,
                out.dmem_req.is_write ? 1u : 0u,
                out.dmem_req.addr,
                out.dmem_req.wdata,
                out.dmem_req.wstrb,
                out.trap ? 1u : 0u);
        }

        if (out.trap) {
            std::fprintf(
                stderr,
                "unexpected trap at cycle %d pc=0x%08x gpio=0x%08x dmem_valid=%u dmem_write=%u dmem_addr=0x%08x\n",
                cycle,
                out.imem_addr,
                out.gpio_pins,
                out.dmem_req.valid ? 1u : 0u,
                out.dmem_req.is_write ? 1u : 0u,
                out.dmem_req.addr);
            return 1;
        }

        if (out.gpio_pins != last_gpio) {
            if (observed >= expected.len) {
                std::fprintf(stderr, "unexpected extra gpio value 0x%08x at cycle %d\n", out.gpio_pins, cycle);
                return 1;
            }
            if (out.gpio_pins != expected.values[observed]) {
                std::fprintf(
                    stderr,
                    "gpio mismatch at index %zu cycle %d: got 0x%08x expected 0x%08x pc=0x%08x\n",
                    observed,
                    cycle,
                    out.gpio_pins,
                    expected.values[observed],
                    out.imem_addr);
                return 1;
            }
            last_gpio = out.gpio_pins;
            ++observed;
        }

        if (observed == expected.len) {
            return 0;
        }

        eval(top, true, false);
    }

    const SocOut out = decode_output(top);
    std::fprintf(
        stderr,
        "only observed %zu gpio writes after %d cycles; pc=0x%08x gpio=0x%08x trap=%u\n",
        observed,
        expected.max_cycles,
        out.imem_addr,
        out.gpio_pins,
        out.trap ? 1u : 0u);
    return 1;
}
