// SPDX-License-Identifier: GPL-2.0
use super::GuestWrapper;
use crate::exit::*;
use crate::mmu::*;
use crate::vmcs::*;
use crate::vmstat::*;
use crate::lapic::*;
use crate::x86reg::*;
use crate::{rkvm_debug, DEBUG_ON};
use core::arch::{asm, global_asm};
use core::ffi::c_void;
use core::pin::Pin;
use kernel::{ bindings, pages::Pages, prelude::*};
use kernel::sync::{Arc, Mutex, UniqueArc};
use kernel::{Result, PAGE_SIZE};

#[repr(C)]
#[allow(dead_code)]
pub(crate) struct RkvmRegs {
    /* out (KVM_GET_REGS) / in (KVM_SET_REGS) */
    pub(crate) rax: u64,
    pub(crate) rbx: u64,
    pub(crate) rcx: u64,
    pub(crate) rdx: u64,
    pub(crate) rsi: u64,
    pub(crate) rdi: u64,
    pub(crate) rsp: u64,
    pub(crate) rbp: u64,
    pub(crate) r8: u64,
    pub(crate) r9: u64,
    pub(crate) r10: u64,
    pub(crate) r11: u64,
    pub(crate) r12: u64,
    pub(crate) r13: u64,
    pub(crate) r14: u64,
    pub(crate) r15: u64,
    pub(crate) rip: u64,
    pub(crate) rflags: u64,
}

#[repr(C)]
#[allow(dead_code)]
pub(crate) struct RkvmSegment {
    pub(crate) base: u64,
    pub(crate) limit: u32,
    pub(crate) selector: u16,
    pub(crate) rtype: u8,
    pub(crate) present: u8,
    /* dpl, db, s, l, g, avl, unusable,padding*/
    pub(crate) padding: u64,
}

impl RkvmSegment {
    pub(crate) fn new() -> Self {
        Self {
            base: 0,
            limit: 0,
            selector: 0,
            rtype: 0,
            present: 0,
            padding: 0,
        }
    }
}

#[repr(C)]
#[allow(dead_code)]
pub(crate) struct RkvmDtable {
    pub(crate) base: u64,
    pub(crate) limit: u16,
    pub(crate) padding: [u16; 3],
}

impl RkvmDtable {
    pub(crate) fn new() -> Self {
        Self {
            base: 0,
            limit: 0,
            padding: [0, 0, 0],
        }
    }
}

#[repr(C)]
#[allow(dead_code)]
pub(crate) struct RkvmSregs {
    /* out (KVM_GET_SREGS) / in (KVM_SET_SREGS) */
    pub(crate) cs: RkvmSegment,
    pub(crate) ds: RkvmSegment,
    pub(crate) es: RkvmSegment,
    pub(crate) fs: RkvmSegment,
    pub(crate) gs: RkvmSegment,
    pub(crate) ss: RkvmSegment,
    pub(crate) tr: RkvmSegment,
    pub(crate) ldt: RkvmSegment,
    pub(crate) gdt: RkvmDtable,
    pub(crate) idt: RkvmDtable,
    pub(crate) cr0: u64,
    pub(crate) cr2: u64,
    pub(crate) cr3: u64,
    pub(crate) cr4: u64,
    pub(crate) cr8: u64,
    pub(crate) efer: u64,
    pub(crate) apic_base: u64,
    pub(crate) interrupt_bitmap: [u64; 4],
}

impl RkvmSregs {
    pub(crate) fn new() -> Self {
        Self {
            cs: RkvmSegment::new(),
            ds: RkvmSegment::new(),
            es: RkvmSegment::new(),
            fs: RkvmSegment::new(),
            gs: RkvmSegment::new(),
            ss: RkvmSegment::new(),
            tr: RkvmSegment::new(),
            ldt: RkvmSegment::new(),
            gdt: RkvmDtable::new(),
            idt: RkvmDtable::new(),
            cr0: 0,
            cr2: 0,
            cr3: 0,
            cr4: 0,
            cr8: 0,
            efer: 0,
            apic_base: 0,
            interrupt_bitmap: [0, 0, 0, 0],
        }
    }
}

#[repr(C)]
#[allow(dead_code)]
pub(crate) struct Pio {
    pub(crate) direction: u8,
    pub(crate) size: u8,
    pub(crate) port: u16,
    pub(crate) count: u32,
    pub(crate) data_offset: u64,
}

#[repr(C)]
pub(crate) struct Mmio {
    pub(crate) gpa: u64,
    pub(crate) inst_len: u16,
    pub(crate) inst_buf: [u8; 15],
}

#[repr(C)]
#[allow(dead_code)]
pub(crate) struct RkvmRun {
    /* in */
    pub(crate) runin: u64,
    /* out */
    pub(crate) exit_reason: u32,
    pub(crate) ready_for_interrupt_injection: u8,
    pub(crate) if_flag: u8,
    pub(crate) flags: u16,
    pub(crate) cr8: u64,
    pub(crate) apic_base: u64,
    pub(crate) io: Pio,
    pub(crate) mmio: Mmio,
}

#[repr(C)]
#[allow(dead_code)]
pub(crate) struct RkvmVmcs {
    pub(crate) revision_id: u32,
    pub(crate) abort: u32,
}

#[allow(dead_code)]
pub(crate) struct RkvmPage {
    pub(crate) rpage: Pages<0>,
    pub(crate) va: u64,
}

impl RkvmPage {
    pub(crate) fn new(rpage: Pages<0>) -> Self {
        let va = unsafe { bindings::page_address(rpage.get()) as u64 };
        let ptr = va as *mut c_void;
        unsafe {
            bindings::memset(ptr, 0, PAGE_SIZE as u64);
        }

        Self {
            rpage: rpage,
            va: va,
        }
    }
    pub(crate) fn as_mut_ptr<T>(&self) -> *mut T {
        self.va as *mut T
    }
}

pub(crate) fn rkvm_irq_disable() {
    unsafe {
        asm!("cli");
    }
}

pub(crate) fn rkvm_irq_enable() {
    unsafe {
        asm!("sti");
    }
}

#[allow(dead_code)]
pub(crate) struct Vcpu_vmx {
    pub(crate) hv_deadline: u64,
}

impl Vcpu_vmx {
    pub(crate) fn new(data: u64) -> Result<Self> {
        Ok(Vcpu_vmx {
            hv_deadline: data,
        })
    }
}

#[allow(dead_code)]
pub(crate) struct Vcpu {
    pub(crate) guest: Arc<GuestWrapper>,
    pub(crate) guest_state: Pin<Box<GuestState>>,
    pub(crate) host_state: Pin<Box<HostState>>,
    // DefMut trait for UniqueArc, List use it
    pub(crate) mmu: UniqueArc<RkvmMmu>,
    pub(crate) lapic: Pin<Box<RkvmLapicState>>,
    pub(crate) run: RkvmPage,
    pub(crate) vmcs: RkvmPage,
    pub(crate) vcpu_id: u32,
    pub(crate) launched: bool,
    // vmx related
    pub(crate) vcpu_vmx: Vcpu_vmx,
}

pub(crate) fn alloc_vmcs(revision_id: u32) -> Result<RkvmPage> {
    let page = Pages::<0>::new()?;

    let vmcs = RkvmPage::new(page);
    unsafe {
        (*(vmcs.as_mut_ptr::<RkvmVmcs>())).revision_id = revision_id;
    }

    rkvm_debug!(
        "Rust kvm: vmcs={:x},revision={:?} \n",
        vmcs.va,
        (*(vmcs.as_mut_ptr::<RkvmVmcs>())).revision_id
    );

    Ok(vmcs)
}

fn vmcs_load(va: u64) {
    let phy = unsafe { bindings::phy_address(va) };
    if phy == 0 {
        pr_err!(" vmcs_load failed \n");
    }
    if vmptrld(phy).is_err() {
        pr_info!(" vmptrld failed phy={:x} \n", phy);
    }
}

fn vmcs_clear(va: u64) {
    let phy = unsafe { bindings::phy_address(va) };
    if vmclear(phy).is_err() {
        pr_info!(" vmclear failed phy={:x} \n", phy);
    }
}

pub(crate) struct VcpuWrapper {
    pub(crate) vcpuinner: Mutex<Vcpu>,
}
impl VcpuWrapper {
    pub(crate) fn new(guest: Arc<GuestWrapper>, revision_id: u32) -> Result<Arc<Self>> {
        let state = Pin::from(Box::try_new(GuestState::new())?);
        let host_state = Pin::from(Box::try_new(HostState::new())?);
        // kvm_run
        let page = Pages::<0>::new()?;
        let run = RkvmPage::new(page);

        // alloc vmcs and init
        let vmcs = alloc_vmcs(revision_id)?;
        let mmu = RkvmMmu::new()?;

        let lapic = RkvmLapicState::new(0xfffff000)?;
        let lapic = Pin::from(Box::try_new(lapic)?);

        let mut v = Pin::from(UniqueArc::try_new(Self {
            vcpuinner: unsafe {
                Mutex::new(Vcpu {
                    guest: guest,
                    guest_state: state,
                    host_state: host_state,
                    lapic: lapic,
                    mmu: mmu,
                    run: run,
                    vmcs: vmcs,
                    vcpu_id: 0,
                    launched: false,
                    vcpu_vmx: Vcpu_vmx::new(0).unwrap(),
                })
            },
        })?);
        let pinned = unsafe { v.as_mut().map_unchecked_mut(|s| &mut s.vcpuinner) };
        kernel::mutex_init!(pinned, "VcpuWrapper::vcpuinner");

        Ok(v.into())
    }

    pub(crate) fn init(&self, vmcsconf: &mut VmcsConfig) {
        let mut vcpuinner = self.vcpuinner.lock();
        vmcs_clear(vcpuinner.vmcs.va);
        vmcs_load(vcpuinner.vmcs.va);
        vmcsconf.vcpu_vmcs_init();
        vcpuinner.mmu.init_mmu_root();
        let host_rsp = vcpuinner.guest_state.as_ref().get_ref() as *const _ as u64;
        vmcs_write64(VmcsField::HOST_RSP, host_rsp);
    }

    pub(crate) fn get_run(&self) -> u64 {
        self.vcpuinner.lock().run.va
    }

    pub(crate) fn vcpu_exit_handler(&self) -> Result<u64> {
        let exit_info = ExitInfo::from_vmcs();

        match exit_info.exit_reason {
            ExitReason::EXTERNAL_INTERRUPT => {
                let intr_info = vmcs_read32(VmcsField::IDT_VECTORING_INFO);
                rkvm_debug!(" interrupt: {:x} \n", intr_info);

                Ok(1)
            }
            ExitReason::CPUID => handle_cpuid(&exit_info, self),
            ExitReason::HLT => handle_hlt(&exit_info, self),
            ExitReason::IO_INSTRUCTION => handle_io(&exit_info, self),
            ExitReason::EPT_VIOLATION => handle_ept_violation(&exit_info, self),
            ExitReason::EPT_MISCONFIGURATION => {
                let vector_info = vmcs_read32(VmcsField::IDT_VECTORING_INFO);
                if vector_info & 0x80000000 != 0 {
                    pr_err!(" EPT_MISCONFIGURATION, vector_info: {:x} \n", vector_info);

                    let vcpuinner = self.vcpuinner.lock();
                    unsafe {
                        (*(vcpuinner.run.as_mut_ptr::<RkvmRun>())).exit_reason =
                            (RkvmUserExitReason::RKVM_EXIT_INTERNAL_ERROR) as u32;
                    }
                    return Err(EINVAL);
                }
                // The mmio spte page table method is not used, and the mmio command is emulated
                // in user mode, so the ept misconfig will not be triggered.
                pr_err!(
                    " EPT_MISCONFIGURATION is invalid, vector_info: {:x} \n",
                    vector_info
                );
                Err(EINVAL)
            }
            _ => {
                pr_err!(" vmx exit_reason = {:?} \n", exit_info.exit_reason);
                let vcpuinner = self.vcpuinner.lock();
                unsafe {
                    (*(vcpuinner.run.as_mut_ptr::<RkvmRun>())).exit_reason =
                        (RkvmUserExitReason::RKVM_EXIT_INTERNAL_ERROR) as u32;
                }
                Err(EINVAL)
            }
        }
    }

    pub(crate) fn vcpu_run(&self) -> i64 {
        {
            let mut vcpuinner = self.vcpuinner.lock();
            vmcs_load(vcpuinner.vmcs.va);
            let rip = vmcs_read64(VmcsField::GUEST_RIP);

            rkvm_debug!(
                " vcpu_run state guest rip = {:x}, read guest rip = {:x} \n",
                vcpuinner.guest_state.rip,
                rip,
            );

            vcpuinner.guest_state.rip = rip;
        }
        loop {
            rkvm_irq_disable();
            let has_err_;
            {
                let mut vcpuinner = self.vcpuinner.lock();
                let launched = vcpuinner.guest_state.launched;

                // handle interrupt inject
                vcpuinner.lapic.lapicInterrupt();
                rkvm_debug!(
                    " vmentry: launched = {:?}, guest_rip={:x} \n",
                    launched,
                    vmcs_read64(VmcsField::GUEST_RIP)
                );

                unsafe {
                    has_err_ = _vmx_vcpu_run(&vcpuinner.guest_state);
                }
            }

            rkvm_debug!(
                " vmexit: guest_rip={:x} \n",
                vmcs_read64(VmcsField::GUEST_RIP)
            );

            if has_err_ == 1 {
                rkvm_irq_enable();
                let mut vcpuinner = self.vcpuinner.lock();
                dump_vmcs();
                let host_rsp = vmcs_read64(VmcsField::HOST_RSP);
                unsafe {
                    (*(vcpuinner.run.as_mut_ptr::<RkvmRun>())).exit_reason =
                        (RkvmUserExitReason::RKVM_EXIT_FAIL_ENTRY) as u32;
                }

                let ret = vmcs_read32(VmcsField::VM_INSTRUCTION_ERROR);
                let rflags = read_rflags();

                pr_err!(
                    "run loop after _vmx_vcpu_run, rflags={:x},ret={:x} \n",
                    rflags,
                    ret
                );

                return -1;
            }
            rkvm_irq_enable();
            {
                let mut vcpuinner = self.vcpuinner.lock();

                vcpuinner.guest_state.launched = true;
            }
            //match vmexit_handler
            let ret = self.vcpu_exit_handler();
            rkvm_debug!("ret={:?}, after vcpu_exit_handler \n", ret);
            // TODO: according to ret, update run
            match ret {
                Ok(r) => {
                    if r == 0 {
                        return r.try_into().unwrap();
                    }
                }
                Err(_err) => {
                    let vcpuinner = self.vcpuinner.lock();
                    pr_err!("  vcpu run failed \n");
                    dump_vmcs();
                    unsafe {
                        (*(vcpuinner.run.as_mut_ptr::<RkvmRun>())).exit_reason =
                            (RkvmUserExitReason::RKVM_EXIT_INTERNAL_ERROR) as u32;
                    }
                    return -1;
                }
            }
        } // loop
    }
    pub(crate) fn set_regs(&self, regs: &RkvmRegs) {
        let mut vcpuinner = self.vcpuinner.lock();
        vmcs_load(vcpuinner.vmcs.va);

        vcpuinner.guest_state.rax = regs.rax;
        vcpuinner.guest_state.rbx = regs.rbx;
        vcpuinner.guest_state.rcx = regs.rcx;
        vcpuinner.guest_state.rdx = regs.rdx;
        vcpuinner.guest_state.rsi = regs.rsi;
        vcpuinner.guest_state.rdi = regs.rdi;
        vcpuinner.guest_state.rsp = regs.rsp;
        vcpuinner.guest_state.rbp = regs.rbp;
        vcpuinner.guest_state.r8 = regs.r8;
        vcpuinner.guest_state.r9 = regs.r9;
        vcpuinner.guest_state.r10 = regs.r10;
        vcpuinner.guest_state.r11 = regs.r11;
        vcpuinner.guest_state.r12 = regs.r12;
        vcpuinner.guest_state.r13 = regs.r13;
        vcpuinner.guest_state.r14 = regs.r14;
        vcpuinner.guest_state.r15 = regs.r15;
        vcpuinner.guest_state.rip = regs.rip;
        vmcs_write64(VmcsField::GUEST_RIP, regs.rip);

        rkvm_debug!(
            " set_regs guest_rip = {:x}, state_rax = {:x}\n",
            regs.rip,
            vcpuinner.guest_state.rax,
        );

        vmcs_write64(VmcsField::GUEST_RFLAGS, regs.rflags);
    }

    pub(crate) fn get_regs(&self, regs: &mut RkvmRegs) {
        let vcpuinner = self.vcpuinner.lock();
        vmcs_load(vcpuinner.vmcs.va);
        //let guest_state = &vcpuinner.guest_state;
        regs.rax = vcpuinner.guest_state.rax;
        regs.rbx = vcpuinner.guest_state.rbx;
        regs.rcx = vcpuinner.guest_state.rcx;
        regs.rdx = vcpuinner.guest_state.rdx;
        regs.rsi = vcpuinner.guest_state.rsi;
        regs.rdi = vcpuinner.guest_state.rdi;
        regs.rsp = vcpuinner.guest_state.rsp;
        regs.rbp = vcpuinner.guest_state.rbp;
        regs.r8 = vcpuinner.guest_state.r8;
        regs.r9 = vcpuinner.guest_state.r9;
        regs.r10 = vcpuinner.guest_state.r10;
        regs.r11 = vcpuinner.guest_state.r11;
        regs.r12 = vcpuinner.guest_state.r12;
        regs.r13 = vcpuinner.guest_state.r13;
        regs.r14 = vcpuinner.guest_state.r14;
        regs.r15 = vcpuinner.guest_state.r15;
        regs.rip = vcpuinner.guest_state.rip;
        regs.rflags = vmcs_read64(VmcsField::GUEST_RFLAGS);
    }

    pub(crate) fn set_sregs(&self, sregs: &RkvmSregs) {
        let vcpuinner = self.vcpuinner.lock();
        vmcs_load(vcpuinner.vmcs.va);
        let base = sregs.cs.base;
        let selector = sregs.cs.selector;
        vmcs_write64(VmcsField::GUEST_CS_BASE, base);
        vmcs_write16(VmcsField::GUEST_CS_SELECTOR, selector);
    }

    pub(crate) fn get_sregs(&self, sregs: &mut RkvmSregs) {
        let vcpuinner = self.vcpuinner.lock();
        vmcs_load(vcpuinner.vmcs.va);
        sregs.cs.base = vmcs_read64(VmcsField::GUEST_CS_BASE);
        sregs.cs.selector = vmcs_read16(VmcsField::GUEST_CS_SELECTOR);
    }
}

impl VcpuWrapper {
    pub(crate) fn Rkvm_set_tscdeadline_msr(&self, data: u64) {
        let mut vcpuinner = self.vcpuinner.lock();
        // apic is active and tscdeadline mode is on by default
        bindings::hrtimer_cancel(&vcpuinner.lapic.lapic_timer);
        vcpuinner.lapic.tscdeadline = data;
        vcpuinner.lapic.start_apic_timer();
    }

    pub(crate) fn read_time_stamp_counter() -> u64 {
        let mut low: u32;
        let mut high: u32;
    
        unsafe {
            asm!(
                "rdtsc",
                out("eax") low,
                out("edx") high,
                options(nomem, nostack, preserves_flags)
            );
        }
    
        (low as u64) | ((high as u64) << 32)
    }

    // unfinished
    pub(crate) fn vmx_set_hv_timer (&self, guest_deadline_tsc: u64) -> bool {
        let mut vcpuinner = self.vcpuinner.lock();
        let mut vmx  = vcpuinner.vcpu_vmx;

        // read current tsc value
        let tscl: u64 = read_time_stamp_counter();
        let guest_tscl: u64 = 0; // virtualized guest tsc
        let mut delta_tsc: u64;
        if guest_deadline_tsc > guest_tscl {
            delta_tsc = guest_deadline_tsc - tscl;
        } else {
            delta_tsc = guest_tscl - tscl;
        }

        let mut lapic_timer_advance_cycles: u64 = 0; // nsec_to_cycles(vcpu, ktimer->timer_advance_ns);

        if delta_tsc > lapic_timer_advance_cycles {
            delta_tsc -= lapic_timer_advance_cycles;
        } else {
            delta_tsc = 0;
        }

        // tsc scaling

        // make sure

        // store tsc deadline time
        vmx.hv_deadline = tscl + delta_tsc;

        // if delta == 0, timer expires, send true
        !delta_tsc
    }
}

impl Drop for VcpuWrapper {
    fn drop(&mut self) {
        pr_info!(" vcpu droped \n");
    }
}

extern "C" {
    fn _vmx_vcpu_run(guest_state: &GuestState) -> u32;
}

global_asm!(
    "
.global _vmx_vcpu_run
_vmx_vcpu_run:
    push   rbp
    mov    rbp,rsp
    push   r15
    push   r14
    push   r13
    push   r12
    push   rbx
    
    mov     [rdi], rsp
    mov     rsp, rdi

    add     rsp, 8
    pop     rax
    pop     rcx
    pop     rdx
    pop     rbx
    add     rsp, 8 // skip rsp
    pop     rbp
    pop     rsi
    pop     rdi
    pop     r8
    pop     r9
    pop     r10
    pop     r11
    pop     r12
    pop     r13
    pop     r14
    pop     r15

    cmp     byte ptr [rsp], 0
    je 3f
    vmresume
    jmp 4f
3:  vmlaunch

4:
    // We will only be here if vmlaunch or vmresume failed.
    // Restore host callee, RSP and return address.
    mov     rsp, [rsp - 17*8]
    pop     rbx
    pop     r12
    pop     r13
    pop     r14
    pop     r15
    pop     rbp

    // return true
    mov     eax, 1
    ret
"
);
