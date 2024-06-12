// SPDX-License-Identifier: GPL-2.0

use crate::lapic_priv::X86InterruptVector::X86_INT_NMI;
use crate::lapic_priv::X86InterruptVector::X86_INT_PLATFORM_BASE;
use crate::lapic_priv::X86InterruptVector::X86_INT_VIRT;
use crate::lib::Bitmap;
use crate::vmcs::*;
use crate::x86reg::RFlags;
use kernel::{bindings, container_of, prelude::*, Result};
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

static INTERRUPTIBILITY_STI_BLOCKING: u32 = 1 << 0;
static INTERRUPTIBILITY_MOV_SS_BLOCKING: u32 = 1 << 1;
static INTERRUPTIBILITY_NMI_BLOCKING: u32 = 1 << 3;

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

// LAPIC_TIMER: struct
pub(crate) struct LapicTimer {
    pub(crate) timer: bindings::hrtimer,
    // pub(crate) timer_dconfig: u32
    // pub(crate) timer_init: u32,
    pub(crate) period: i64,
    pub(crate) target_expiration: bindings::ktime_t,
    pub(crate) tscdeadline: u64,
    pub(crate) expired_tscdeadline: u64,
    pub(crate) timer_advance_ns: u64,
    // atomic_t pending here
    pub(crate) hv_timer_in_use: bool,
}

pub(crate) struct RkvmLapicState {
    pub(crate) base_address: u64,
    pub(crate) lapic_timer: bindings::hrtimer,
    /// just realize periodic timer
    pub(crate) timer_dconfig: u32,
    pub(crate) timer_init: u32,
    pub(crate) interrupt_bitmap: Bitmap,
    /// The highest vector set in ISR; if -1 - invalid, must scan ISR.
    pub(crate) highest_isr_cache: u32,
    // LAPIC_TIMER: try to add a LapicTimer struct
    pub(crate) test_ltimer: LapicTimer,
}

// apic timer fn
// 获取hrtimer所在结构和kvm_timer所在结构 -> apic
// apic_timer_expired(apic, true); 从timer_fn，apic_timer到期
// lapic is periodic =>
//     advance periodic target expiration(apic)
//     hrtimer add expires nx (ktimer->timer, ktimer->period)
// else => HRTIMER_NORESTART
extern "C" fn lapic_timer_callback(arg1: *mut bindings::hrtimer) -> bindings::hrtimer_restart {
    let lapic: &RkvmLapicState = unsafe {&*container_of!(arg1, RkvmLapicState, lapic_timer)};
    bindings::hrtimer_restart_HRTIMER_NORESTART
}

impl RkvmLapicState {
    pub(crate) fn new(base: u64) -> Result<Self> {
        let interrupt_bitmap = Bitmap::new(256)?;

        let mut lapic = Self {
            base_address: base,
            lapic_timer: bindings::hrtimer {
                // init hrtimer
                node: bindings::timerqueue_node {
                    node: bindings::rb_node {
                        __rb_parent_color: 0,
                        rb_right: core::ptr::null_mut(),
                        rb_left: core::ptr::null_mut(),
                    },
                    expires: 0,
                },
                _softexpires: 0,
                function: None,
                base: core::ptr::null_mut(),
                state: 0,
                is_rel: 0,
                is_soft: 0,
                is_hard: 0,
            },
            timer_dconfig: 0,
            timer_init: 0,
            interrupt_bitmap: interrupt_bitmap,
            highest_isr_cache: 0,
            // LAPIC_TIMER: init a timer for LapicTimer
            test_ltimer: LapicTimer {
                timer: bindings::hrtimer {
                    // init hrtimer
                    node: bindings::timerqueue_node {
                        node: bindings::rb_node {
                            __rb_parent_color: 0,
                            rb_right: core::ptr::null_mut(),
                            rb_left: core::ptr::null_mut(),
                        },
                        expires: 0,
                    },
                    _softexpires: 0,
                    function: None,
                    base: core::ptr::null_mut(),
                    state: 0,
                    is_rel: 0,
                    is_soft: 0,
                    is_hard: 0,
                },
            },
        };

        unsafe {
            bindings::hrtimer_init(
                &mut lapic.lapic_timer,
                bindings::CLOCK_MONOTONIC.try_into().unwrap(),
                bindings::hrtimer_mode_HRTIMER_MODE_ABS,
            );
            lapic.lapic_timer.function = Some(lapic_timer_callback);
        }

        // LAPIC_TIMER: init timer
        unsafe {
            bindings::hrtimer_init(
                &mut lapic.test_ltimer.timer,
                bindings::CLOCK_MONOTONIC.try_into().unwrap(),
                bindings::hrtimer_mode_HRTIMER_MODE_ABS,
            );
            lapic.lapic_timer.function = Some(lapic_timer_callback);
        }
        Ok(lapic)
    }
    pub(crate) fn lapicInterrupt(&mut self) -> Result<i32> {
        let vector: u8;
        let active = self.interrupt_bitmap.get(X86_INT_NMI as usize);
        if active {
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
            & (INTERRUPTIBILITY_NMI_BLOCKING | INTERRUPTIBILITY_MOV_SS_BLOCKING)
            == 0;
        let can_inj_int = (vmcs_read64(VmcsField::GUEST_RFLAGS) & RFlags::FLAGS_IF as u64 != 0)
            && (vmcs_read32(VmcsField::GUEST_INTERRUPTIBILITY_INFO)
                & (INTERRUPTIBILITY_STI_BLOCKING | INTERRUPTIBILITY_MOV_SS_BLOCKING))
                == 0;
        if vector > X86_INT_VIRT as u8 && vector < X86_INT_PLATFORM_BASE as u8 {
            pr_err!("Invalid interrupt vector: {}\n", vector);
            return Err(ENOTSUPP);
        } else if (vector >= X86_INT_PLATFORM_BASE as u8 && !can_inj_int)
            || (vector == X86_INT_NMI as u8 && !can_inj_nmi)
        {
            self.interrupt_bitmap.set(vector.into());
            // If interrupts are disabled, we set VM exit on interrupt enable.
            interrupt_window_exiting(true);
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
        Ok(0)
    }
}
