// SPDX-License-Identifier: GPL-2.0

use crate::lib::Bitmap;
use kernel::{error, Result,bindings, prelude::*};
use kernel::sync::Arc;
use crate::lapic_priv::X86InterruptVector::X86_INT_NMI;
use crate::lapic_priv::X86InterruptVector::X86_INT_PLATFORM_BASE;
use crate::lapic_priv::X86InterruptVector::X86_INT_VIRT;
use crate::vmcs::*;
use crate::x86reg::RFlags;
macro_rules! ICR_DST {
    ($x:expr) => {
        ($x as u32) << 24
    };
}

macro_rules! ICR_DELIVERY_MODE {
    ($x:expr) => {
        ($x as u32) << 8
    };
}

macro_rules! ICR_DST_SHORTHAND {
    ($x:expr) => {
        ($x as u32) << 18
    };
}

static InterruptibilityStiBlocking: u32 = 1 << 0;
static InterruptibilityMovSsBlocking: u32 = 1 << 1;
static InterruptibilityNmiBlocking: u32 = 1 << 3;

// ICR_DST_BROADCAST ICR_DST(0xff)
// ICR_DST_SELF ICR_DST_SHORTHAND(1)
// ICR_DST_ALL ICR_DST_SHORTHAND(2)
// ICR_DST_ALL_MINUS_SELF ICR_DST_SHORTHAND(3)

macro_rules! LAPIC_REG_IN_SERVICE {
    ($x:expr) => {
        0x100 as u32 + ($x as u32) << 4
    };
}

macro_rules! LAPIC_REG_TRIGGER_MODE {
    ($x:expr) => {
        0x180 as u32 + ($x as u32) << 4
    };
}

macro_rules! LAPIC_REG_IRQ_REQUEST {
    ($x:expr) => {
        0x200 as u32 + ($x as u32) << 4
    };
}

pub(crate) struct LapicReg {}

pub(crate) struct RkvmLapicState {
    pub(crate) base_address: u64,
    //pub(crate) lapic_timer: bindings::hrtimer,
    pub(crate) timer_dconfig: u32,
    pub(crate) timer_init: u32,
    pub(crate) interrupt_bitmap: Bitmap,
    //pub(crate) regs: LapicReg,
    /// The highest vector set in ISR; if -1 - invalid, must scan ISR.
    pub(crate) highest_isr_cache: u32,
}

impl RkvmLapicState {
    pub(crate) fn new(base: u64) -> Result<Self> {
        let interrupt_bitmap = Bitmap::new(256);
        let interrupt_bitmap = match interrupt_bitmap {
            Ok(interrupt_bitmap) => interrupt_bitmap,
            Err(err) => return Err(err),
        };

        let lapic = Self {
            base_address: base,
            timer_dconfig: 0,
            timer_init:    0,
            interrupt_bitmap: interrupt_bitmap,
            highest_isr_cache: 0,
        };
        Ok(lapic)
    }
    pub(crate) fn lapicInterrupt(&mut self) -> Result<i32> {
        let vector: u8;
        let active = self.interrupt_bitmap.get(X86_INT_NMI as usize);
        if active == false {
            vector = X86_INT_NMI as u8;
        } else {
            // get normal interrupt vector
            vector = self.interrupt_bitmap.scan() as u8;
            if vector != 0xff {
                self.interrupt_bitmap.clear(vector.into());
            } else {
                return Ok(0);
            }
        }
        // inject interrupt
        let can_inj_nmi = vmcs_read32(VmcsField::GUEST_INTERRUPTIBILITY_INFO)
            & (InterruptibilityNmiBlocking | InterruptibilityMovSsBlocking)
            == 0;
        let can_inj_int = (vmcs_read64(VmcsField::GUEST_RFLAGS) & RFlags::FLAGS_IF as u64 != 0)
            && (vmcs_read32(VmcsField::GUEST_INTERRUPTIBILITY_INFO)
                & (InterruptibilityStiBlocking | InterruptibilityMovSsBlocking))
                == 0;
        if vector > X86_INT_VIRT as u8 && vector < X86_INT_PLATFORM_BASE as u8 {
            pr_err!("Invalid interrupt vector: {}\n", vector);
            return Err(ENOTSUPP);
        } else if (vector >= X86_INT_PLATFORM_BASE as u8 && !can_inj_int)
            || (vector == X86_INT_NMI as u8 && !can_inj_nmi)
        {
            self.interrupt_bitmap.set(vector.into());
            // If interrupts are disabled, we set VM exit on interrupt enable.
            InterruptWindowExiting(true);
            return Ok(0);
        }
        issue_interrupt(vector);

        // Volume 3, Section 6.9: Lower priority exceptions are discarded; lower priority interrupts are
        // held pending. Discarded exceptions are re-generated when the interrupt handler returns
        // execution to the point in the program or task where the exceptions and/or interrupts
        // occurred.
        self.interrupt_bitmap.clear_range(0, X86_INT_NMI as usize);
        self.interrupt_bitmap
            .clear_range(X86_INT_NMI as usize + 1, X86_INT_VIRT as usize + 1);
        return Ok(0);
    }
}
