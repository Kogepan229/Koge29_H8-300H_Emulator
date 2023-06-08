use crate::cpu::{Cpu, CCR};
use anyhow::{bail, Context as _, Result};

impl<'a> Cpu<'a> {
    pub(in super::super) fn mov_b(&mut self, opcode: u16) -> Result<usize> {
        match opcode as u8 {
            0x0c => return self.mov_b_rn(opcode),
            0xf0..=0xf7 => return self.mov_b_imm(opcode),
            0x68 => return self.mov_b_ern(opcode),
            0x6e => return self.mov_b_disp16(opcode),
            //0x78 => return self.mov_b_disp24(opcode),
            0x6c => return self.mov_b_inc_or_dec(opcode),
            0x20..=0x27 | 0x30..=0x37 => return self.mov_b_abs8(opcode),
            0x6a => return self.mov_b_abs_16_or_24(opcode),
            _ => bail!("invalid opcode [{:>04x}]", opcode),
        }
    }

    fn mov_b_proc_pcc(&mut self, src: u8) {
        if (src as i8) < 0 {
            self.write_ccr(CCR::N, 1);
        } else {
            self.write_ccr(CCR::N, 0);
        }
        if src == 0 {
            self.write_ccr(CCR::Z, 1);
        } else {
            self.write_ccr(CCR::Z, 0);
        }
        self.write_ccr(CCR::V, 0);
    }

    fn mov_b_rn(&mut self, opcode: u16) -> Result<usize> {
        let mut f = || -> Result<usize> {
            let value = self.read_rn_b(Cpu::get_nibble_opcode(opcode, 3)?)?;
            self.write_rn_b(Cpu::get_nibble_opcode(opcode, 4)?, value)?;
            self.mov_b_proc_pcc(value);
            return Ok(2);
        };
        f().with_context(|| format!("opcode [{:x}]", opcode))
    }

    fn mov_b_imm(&mut self, opcode: u16) -> Result<usize> {
        let mut f = || -> Result<usize> {
            self.write_rn_b(Cpu::get_nibble_opcode(opcode, 2)?, opcode as u8)?;
            self.mov_b_proc_pcc(opcode as u8);
            return Ok(2);
        };
        f().with_context(|| format!("opcode [{:x}]", opcode))
    }

    fn mov_b_ern(&mut self, opcode: u16) -> Result<usize> {
        let mut f = || -> Result<usize> {
            if opcode & 0x0080 == 0 {
                let value = self.read_ern_b(Cpu::get_nibble_opcode(opcode, 3)?)?;
                self.write_rn_b(Cpu::get_nibble_opcode(opcode, 4)?, value)?;
                self.mov_b_proc_pcc(value);
            } else {
                let value = self.read_rn_b(Cpu::get_nibble_opcode(opcode, 4)?)?;
                self.write_ern_b(Cpu::get_nibble_opcode(opcode, 3)? & 0x07, value)?;
                self.mov_b_proc_pcc(value);
            }
            return Ok(4);
        };
        f()
    }

    fn mov_b_disp16(&mut self, opcode: u16) -> Result<usize> {
        let disp = self.fetch();
        let mut f = || -> Result<usize> {
            if opcode & 0x0080 == 0 {
                let value = self.read_disp16_ern_b(Cpu::get_nibble_opcode(opcode, 3)?, disp)?;
                self.write_rn_b(Cpu::get_nibble_opcode(opcode, 4)?, value)?;
                self.mov_b_proc_pcc(value);
            } else {
                let value = self.read_rn_b(Cpu::get_nibble_opcode(opcode, 4)?)?;
                self.write_disp16_ern_b(Cpu::get_nibble_opcode(opcode, 3)? & 0x07, disp, value)?;
                self.mov_b_proc_pcc(value);
            }
            return Ok(6);
        };
        f().with_context(|| format!("disp [{:x}]", disp))
    }

    pub(in super::super) fn mov_b_disp24(&mut self, opcode: u16, opcode2: u16) -> Result<usize> {
        let disp = ((self.fetch() as u32) << 16) | self.fetch() as u32;
        let mut f = || -> Result<usize> {
            if opcode2 & 0xfff0 == 0x6b20 {
                let value = self.read_disp24_ern_b(Cpu::get_nibble_opcode(opcode, 3)?, disp)?;
                self.write_rn_b(Cpu::get_nibble_opcode(opcode2, 4)?, value)?;
                self.mov_b_proc_pcc(value);
            } else {
                let value = self.read_rn_b(Cpu::get_nibble_opcode(opcode2, 4)?)?;
                self.write_disp24_ern_b(Cpu::get_nibble_opcode(opcode, 3)? & 0x07, disp, value)?;
                self.mov_b_proc_pcc(value);
            }
            return Ok(10);
        };
        f().with_context(|| format!("opcode2 [{:x}] disp [{:x}]", opcode2, disp))
    }

    fn mov_b_inc_or_dec(&mut self, opcode: u16) -> Result<usize> {
        let mut f = || -> Result<usize> {
            if opcode & 0x0080 == 0 {
                let value = self.read_inc_ern_b(Cpu::get_nibble_opcode(opcode, 3)?)?;
                self.write_rn_b(Cpu::get_nibble_opcode(opcode, 4)?, value)?;
                self.mov_b_proc_pcc(value);
            } else {
                let value = self.read_rn_b(Cpu::get_nibble_opcode(opcode, 4)?)?;
                self.write_dec_ern_b(Cpu::get_nibble_opcode(opcode, 3)? & 0x07, value)?;
                self.mov_b_proc_pcc(value);
            }
            return Ok(6);
        };
        f()
    }

    fn mov_b_abs8(&mut self, opcode: u16) -> Result<usize> {
        let mut f = || -> Result<usize> {
            if opcode & 0xf000 == 0x2000 {
                let value = self.read_abs8_b(opcode as u8)?;
                self.write_rn_b(Cpu::get_nibble_opcode(opcode, 2)?, value)?;
                self.mov_b_proc_pcc(value);
            } else {
                let value = self.read_rn_b(Cpu::get_nibble_opcode(opcode, 2)?)?;
                self.write_abs8_b(opcode as u8, value)?;
                self.mov_b_proc_pcc(value);
            }
            return Ok(4);
        };
        f()
    }

    fn mov_b_abs_16_or_24(&mut self, opcode: u16) -> Result<usize> {
        match opcode & 0xfff0 {
            0x6a00 | 0x6a80 => return self.mov_b_abs16(opcode),
            0x6a20 | 0x6aa0 => return self.mov_b_abs24(opcode),
            _ => bail!("invalid opcode [{:x}]", opcode),
        }
    }

    fn mov_b_abs16(&mut self, opcode: u16) -> Result<usize> {
        let abs_addr = self.fetch();
        let mut f = || -> Result<usize> {
            if opcode & 0xfff0 == 0x6b00 {
                let value = self.read_abs16_b(abs_addr)?;
                self.write_rn_b(Cpu::get_nibble_opcode(opcode, 4)?, value)?;
                self.mov_b_proc_pcc(value);
            } else {
                let value = self.read_rn_b(Cpu::get_nibble_opcode(opcode, 4)?)?;
                self.write_abs16_b(abs_addr, value)?;
                self.mov_b_proc_pcc(value);
            }
            return Ok(6);
        };
        f().with_context(|| format!("abs_addr [{:x}]", abs_addr))
    }

    fn mov_b_abs24(&mut self, opcode: u16) -> Result<usize> {
        let abs_addr = ((self.fetch() as u32) << 16) | self.fetch() as u32;
        let mut f = || -> Result<usize> {
            if opcode & 0xfff0 == 0x6b20 {
                let value = self.read_abs24_b(abs_addr)?;
                self.write_rn_b(Cpu::get_nibble_opcode(opcode, 4)?, value)?;
                self.mov_b_proc_pcc(value);
            } else {
                let value = self.read_rn_b(Cpu::get_nibble_opcode(opcode, 4)?)?;
                self.write_abs24_b(abs_addr, value)?;
                self.mov_b_proc_pcc(value);
            }
            return Ok(8);
        };
        f().with_context(|| format!("abs_addr [{:x}]", abs_addr))
    }
}