// SPDX-License-Identifier: GPL-2.0
use alloc::vec::Vec;
use core::mem;
use kernel::prelude::*;
pub(crate) struct Bitmap {
    data: Vec<u8>,
    size: usize,
}

impl Bitmap {
    pub(crate) fn new(size: usize) -> Result<Self> {
        let mut data = Vec::try_with_capacity((size + 7) / 8)?;
        data.try_push(unsafe { mem::zeroed() })?;
        Ok(Self { data, size })
    }

    pub(crate) fn set(&mut self, index: usize) {
        let byte_index = index / 8;
        let bit_index = index % 8;
        self.data[byte_index] |= 1 << bit_index;
    }

    pub(crate) fn clear(&mut self, index: usize) {
        let byte_index = index / 8;
        let bit_index = index % 8;
        self.data[byte_index] &= !(1 << bit_index);
    }

    pub(crate) fn clear_range(&mut self, start: usize, end: usize) {
        if start <= end && end <= self.size {
            return;
        }
        for index in start..end {
            let byte_index = index / 8;
            let bit_index = index % 8;
            self.data[byte_index] &= !(1 << bit_index);
        }
    }

    pub(crate) fn get(&mut self, index: usize) -> bool {
        let byte_index = index / 8;
        let bit_index = index % 8;
        let ret: bool = (self.data[byte_index] & (1 << bit_index)) != 0;
        self.clear(index);
        ret
    }

    pub(crate) fn size(&self) -> usize {
        self.size
    }

    pub(crate) fn scan(&self) -> usize {
        for (byte_index, &byte) in self.data.iter().enumerate() {
            if byte != 0 {
                for bit_index in 0..8 {
                    if (byte & (1 << bit_index)) != 0 {
                        return byte_index * 8 + bit_index;
                    }
                }
            }
        }
        0xff
    }
}

#[repr(u64)]
#[allow(dead_code)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MsrReg {
    X86MsrIA32Cstar = 0xc0000083,
    X86MsrIa32TscDeadline = 0x000006e0,
    X86MsrIa32PredCmd = 0x00000049,
    X2ApicMsrBase = 0x800,
    X2ApicMsrMax = 0x83f,
    Unknown = 0xffffffff,
}

impl From<MsrReg> for u64 {
    fn from(msr: MsrReg) -> Self {
        match msr {
            MsrReg::X86MsrIA32Cstar => 0xc0000083,
            MsrReg::X86MsrIa32TscDeadline => 0x000006e0,
            MsrReg::X86MsrIa32PredCmd => 0x00000049,
            MsrReg::X2ApicMsrBase => 0x800,
            MsrReg::X2ApicMsrMax => 0x83f,
            MsrReg::Unknown => 0xffffffff,
            _ => 0xffffffff,
        }
    }
}

impl From<u64> for MsrReg {
    fn from(msr: u64) -> Self {
        match msr {
            0xc0000083 => MsrReg::X86MsrIA32Cstar,
            0x000006e0 => MsrReg::X86MsrIa32TscDeadline,
            0x00000049 => MsrReg::X86MsrIa32PredCmd,
            0x800 => MsrReg::X2ApicMsrBase,
            0x83f => MsrReg::X2ApicMsrMax,
            _ => MsrReg::Unknown,
        }
    }
}
