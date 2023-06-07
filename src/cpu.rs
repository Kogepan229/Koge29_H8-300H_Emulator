use crate::{
    mcu::Mcu,
    memory::{MEMORY_END_ADDR, MEMORY_START_ADDR},
    setting,
};
use anyhow::{bail, Context as _, Result};
use std::time;
use std::time::Duration;

mod addressing_mode;
mod instruction;

const CPUCLOCK: usize = 20000000;

pub struct Cpu<'a> {
    pub mcu: &'a mut Mcu,
    pc: u32,
    ccr: u8,
    pub er: [u32; 8],
    pub exit_addr: u32, // address of ___exit
}

pub enum CCR {
    C,
    V,
    Z,
    N,
    U,
    H,
    UI,
    I,
}

macro_rules! unimpl {
    ($op:expr, $pc:expr ) => {
        bail!(
            "unimplemented instruction:[{:>04x}] pc:[{:x}({:x})]",
            $op,
            $pc - 2,
            $pc - 2 - MEMORY_START_ADDR
        )
    };
}

impl<'a> Cpu<'a> {
    pub fn new(mcu: &'a mut Mcu) -> Self {
        Cpu {
            mcu: mcu,
            pc: MEMORY_START_ADDR,
            ccr: 0,
            er: [0; 8],
            exit_addr: 0,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let mut state_sum: usize = 0;
        let mut loop_count: usize = 0;
        self.er[7] = MEMORY_END_ADDR - 0xf;
        let exec_time = time::Instant::now();
        let mut loop_time = time::Instant::now();
        loop {
            if *setting::ENABLE_PRINT_OPCODE.read().unwrap() {
                print!(" {:4x}:   ", self.pc.wrapping_sub(MEMORY_START_ADDR));
            }

            let opcode = self.fetch();
            let state = self.exec(opcode).with_context(|| {
                format!(
                    "[pc: {:0>8x}({:0>8x})] opcode1 [{:0>4x}]",
                    self.pc - 2,
                    self.pc - 2 - MEMORY_START_ADDR,
                    opcode
                )
            })?;
            state_sum += state;
            loop_count += state;

            if *setting::ENABLE_PRINT_OPCODE.read().unwrap() {
                println!("");
            }

            if self.pc == self.exit_addr {
                self.print_er();
                println!(
                    "state: {}, time: {}sec",
                    state_sum,
                    exec_time.elapsed().as_secs_f64()
                );
                return Ok(());
            }

            if loop_count >= 20000 {
                spin_sleep::sleep(
                    Duration::from_secs_f64(loop_count as f64 * 1.0 / CPUCLOCK as f64)
                        .saturating_sub(loop_time.elapsed()),
                );
                loop_count = 0;
                loop_time = time::Instant::now();
            }
        }
    }

    pub fn fetch(&mut self) -> u16 {
        let _pc = self.pc & !1;
        if _pc < MEMORY_START_ADDR || _pc > MEMORY_END_ADDR {
            panic!("fetch error [pc: {:0>8x}]", self.pc)
        }
        let op = (self.mcu.memory[(_pc - MEMORY_START_ADDR) as usize] as u16) << 8
            | (self.mcu.memory[(_pc - MEMORY_START_ADDR + 1) as usize] as u16);

        if *setting::ENABLE_PRINT_OPCODE.read().unwrap() {
            print!("{:0>2x} {:0>2x} ", (op >> 8) as u8, op as u8);
        }

        self.pc += 2;
        op
    }

    fn exec(&mut self, opcode: u16) -> Result<usize> {
        match ((opcode & 0xff00) >> 8) as u8 {
            0x0c | 0xf0..=0xf7 | 0x68 | 0x6e | 0x6c | 0x20..=0x27 | 0x6a => {
                return self.mov_b(opcode)
            }
            0x0d => return self.mov_w(opcode),
            0x69 | 0x6f | 0x6d | 0x6b => return self.mov_w(opcode),
            0x01 | 0x0f => return self.mov_l(opcode),

            0x78 => {
                let opcode2 = self.fetch();
                match (opcode2 >> 8) as u8 {
                    0x6a => return self.mov_b_disp24(opcode, opcode2),
                    0x6b => return self.mov_w_disp24(opcode, opcode2),
                    _ => unimpl!(opcode, self.pc),
                }
            }

            0x79 => match opcode & 0x00f0 {
                0x0 => return self.mov_w(opcode),
                0x0010 => return self.add_w(opcode),
                0x0020 => return self.cmp_w(opcode),
                0x0030 => return self.sub_w(opcode),
                _ => unimpl!(opcode, self.pc),
            },

            0x7a => match opcode & 0x00f0 {
                0x0 => return self.mov_l(opcode),
                0x0010 => return self.add_l(opcode),
                0x0020 => return self.cmp_l(opcode),
                0x0030 => return self.sub_l(opcode),
                _ => unimpl!(opcode, self.pc),
            },

            0x80..=0x8f | 0x08 => return self.add_b(opcode),
            0x09 => return self.add_w(opcode),
            0x0a => return self.add_l(opcode),
            0x0b => return self.adds(opcode),

            0x18 => return self.sub_b(opcode),
            0x19 => return self.sub_w(opcode),
            0x1a => return self.sub_l(opcode),
            0x1b => return self.subs(opcode),

            0x1c | 0xa0..=0xa7 => return self.cmp_b(opcode),
            0x1d => return self.cmp_w(opcode),
            0x1f => return self.cmp_l(opcode),

            0x59 | 0x5a | 0x5b => return self.jmp(opcode),
            0x5d | 0x5e | 0x5f => return self.jsr(opcode),
            0x40..=0x4f | 0x58 => return self.bcc(opcode),
            0x54 => return self.rts(),
            0x57 => Ok(14), // TRAPA命令は無視
            _ => unimpl!(opcode, self.pc),
        }
    }

    pub fn write_ccr(&mut self, target: CCR, val: u8) {
        match val {
            0 => self.ccr &= !(1 << target as u8),
            1 => self.ccr |= 1 << target as u8,
            _ => panic!("[write_ccr] invalid value [{:x}]", val),
        }
    }

    pub fn read_ccr(&self, target: CCR) -> u8 {
        (self.ccr >> target as u8) & 1
    }

    fn print_er(&self) {
        for i in 0..8 {
            print!("ER{}:[{:x}] ", i, self.er[i]);
        }
        println!("");
    }
}
