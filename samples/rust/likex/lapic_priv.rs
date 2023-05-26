#[repr(u32)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
pub(crate) enum LapicReg {
    LAPIC_REG_ID = 0x020,
    LAPIC_REG_VERSION = 0x030,
    LAPIC_REG_TASK_PRIORITY = 0x080,
    LAPIC_REG_PROCESSOR_PRIORITY = 0x0A0,
    LAPIC_REG_EOI = 0x0B0,
    LAPIC_REG_LOGICAL_DST = 0x0D0,
    LAPIC_REG_SPURIOUS_IRQ = 0x0F0,
    LAPIC_REG_ERROR_STATUS = 0x280,
    LAPIC_REG_LVT_CMCI = 0x2F0,
    LAPIC_REG_IRQ_CMD_LOW = 0x300,
    LAPIC_REG_IRQ_CMD_HIGH = 0x310,
    LAPIC_REG_LVT_TIMER = 0x320,
    LAPIC_REG_LVT_THERMAL = 0x330,
    LAPIC_REG_LVT_PERF = 0x340,
    LAPIC_REG_LVT_LINT0 = 0x350,
    LAPIC_REG_LVT_LINT1 = 0x360,
    LAPIC_REG_LVT_ERROR = 0x370,
    LAPIC_REG_INIT_COUNT = 0x380,
    LAPIC_REG_CURRENT_COUNT = 0x390,
    LAPIC_REG_DIVIDE_CONF = 0x3E0,
}

#[repr(u32)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
pub(crate) enum X2apicMsr {
    ID = 0x802,
    VERSION = 0x803,
    EOI = 0x80b,
    TPR = 0x808,
    LDR = 0x80d,
    SVR = 0x80f,
    ISR_31_0 = 0x810,
    ISR_63_32 = 0x811,
    ISR_95_64 = 0x812,
    ISR_127_96 = 0x813,
    ISR_159_128 = 0x814,
    ISR_191_160 = 0x815,
    ISR_223_192 = 0x816,
    ISR_255_224 = 0x817,
    TMR_31_0 = 0x818,
    TMR_63_32 = 0x819,
    TMR_95_64 = 0x81a,
    TMR_127_96 = 0x81b,
    TMR_159_128 = 0x81c,
    TMR_191_160 = 0x81d,
    TMR_223_192 = 0x81e,
    TMR_255_224 = 0x81f,
    IRR_31_0 = 0x820,
    IRR_63_32 = 0x821,
    IRR_95_64 = 0x822,
    IRR_127_96 = 0x823,
    IRR_159_128 = 0x824,
    IRR_191_160 = 0x825,
    IRR_223_192 = 0x826,
    IRR_255_224 = 0x827,
    ESR = 0x828,
    LVT_CMCI = 0x82f,
    ICR = 0x830,
    LVT_TIMER = 0x832,
    LVT_THERMAL_SENSOR = 0x833,
    LVT_MONITOR = 0x834,
    LVT_LINT0 = 0x835,
    LVT_LINT1 = 0x836,
    LVT_ERROR = 0x837,
    INITIAL_COUNT = 0x838,
    DCR = 0x83e,
    SELF_IPI = 0x83f,
    UNKNOWN = 0xfff,
}

impl From<u32> for X2apicMsr {
    fn from(msr: u32) -> Self {
        match msr {
            0x83f => X2apicMsr::SELF_IPI,
            0x83e => X2apicMsr::DCR,
            0x838 => X2apicMsr::INITIAL_COUNT,
            0x832 => X2apicMsr::LVT_TIMER,
            0x80b => X2apicMsr::EOI,
            _ => X2apicMsr::UNKNOWN,
        }
    }
}

#[repr(u8)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
pub(crate) enum InterruptDeliveryMode {
    FIXED = 0,
    SMI = 2,
    NMI = 4,
    INIT = 5,
    STARTUP = 6,
}

#[repr(u8)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
pub(crate) enum X86InterruptVector {
    X86_INT_DIVIDE_0 = 0,
    X86_INT_DEBUG = 1,
    X86_INT_NMI = 2,
    X86_INT_BREAKPOINT = 3,
    X86_INT_OVERFLOW = 4,
    X86_INT_BOUND_RANGE = 5,
    X86_INT_INVALID_OP,
    X86_INT_DEVICE_NA,
    X86_INT_DOUBLE_FAULT = 8,
    X86_INT_INVALID_TSS = 0xa,
    X86_INT_SEGMENT_NOT_PRESENT = 0xb,
    X86_INT_STACK_FAULT = 0xc,
    X86_INT_GP_FAULT = 0xd,
    X86_INT_PAGE_FAULT = 0xe,
    X86_INT_RESERVED = 0xf,
    X86_INT_FPU_FP_ERROR = 0x10,
    X86_INT_ALIGNMENT_CHECK,
    X86_INT_MACHINE_CHECK,
    X86_INT_SIMD_FP_ERROR,
    X86_INT_VIRT,
    X86_INT_MAX_INTEL_DEFINED = 0x1f,

    X86_INT_PLATFORM_BASE = 0x20,
    X86_INT_PLATFORM_MAX = 0xef,

    //X86_INT_LOCAL_APIC_BASE = 0xf0,
    X86_INT_APIC_SPURIOUS = 0xf0,
    X86_INT_APIC_TIMER = 0xf1,
    X86_INT_APIC_ERROR = 0xf2,
    X86_INT_APIC_PMI,
    X86_INT_IPI_GENERIC,
    X86_INT_IPI_RESCHEDULE,
    X86_INT_IPI_INTERRUPT,
    X86_INT_IPI_HALT,

    X86_INT_MAX = 0xff,
}

impl From<X86InterruptVector> for u8 {
    fn from(inttype: X86InterruptVector) -> Self {
        match inttype {
            X86InterruptVector::X86_INT_DIVIDE_0 => 0,
            X86InterruptVector::X86_INT_DEBUG => 1,
            X86InterruptVector::X86_INT_NMI => 2,
            X86InterruptVector::X86_INT_BREAKPOINT => 3,
            X86InterruptVector::X86_INT_OVERFLOW => 4,
            X86InterruptVector::X86_INT_BOUND_RANGE => 5,
            X86InterruptVector::X86_INT_INVALID_OP => 6,
            X86InterruptVector::X86_INT_DEVICE_NA => 7,
            X86InterruptVector::X86_INT_DOUBLE_FAULT => 8,
            X86InterruptVector::X86_INT_INVALID_TSS => 10,
            X86InterruptVector::X86_INT_SEGMENT_NOT_PRESENT => 11,
            X86InterruptVector::X86_INT_STACK_FAULT => 12,
            X86InterruptVector::X86_INT_GP_FAULT => 13,
            X86InterruptVector::X86_INT_PAGE_FAULT => 14,
            X86InterruptVector::X86_INT_RESERVED => 15,
            X86InterruptVector::X86_INT_FPU_FP_ERROR => 16,
            X86InterruptVector::X86_INT_ALIGNMENT_CHECK => 17,
            X86InterruptVector::X86_INT_MACHINE_CHECK => 18,
            X86InterruptVector::X86_INT_SIMD_FP_ERROR => 19,
            X86InterruptVector::X86_INT_VIRT => 20,
            X86InterruptVector::X86_INT_MAX_INTEL_DEFINED => 31,
            X86InterruptVector::X86_INT_PLATFORM_BASE => 32,
            X86InterruptVector::X86_INT_PLATFORM_MAX => 0xef,
            X86InterruptVector::X86_INT_MAX => 0xff,
            _ => 0xff,
        }
    }
}

impl From<u8> for X86InterruptVector {
    fn from(inttype: u8) -> Self {
        match inttype {
            0 => X86InterruptVector::X86_INT_DIVIDE_0,
            1 => X86InterruptVector::X86_INT_DEBUG,
            2 => X86InterruptVector::X86_INT_NMI,
            3 => X86InterruptVector::X86_INT_BREAKPOINT,
            4 => X86InterruptVector::X86_INT_OVERFLOW,
            5 => X86InterruptVector::X86_INT_BOUND_RANGE,
            6 => X86InterruptVector::X86_INT_INVALID_OP,
            7 => X86InterruptVector::X86_INT_DEVICE_NA,
            8 => X86InterruptVector::X86_INT_DOUBLE_FAULT,
            10 => X86InterruptVector::X86_INT_INVALID_TSS,
            11 => X86InterruptVector::X86_INT_SEGMENT_NOT_PRESENT,
            12 => X86InterruptVector::X86_INT_STACK_FAULT,
            13 => X86InterruptVector::X86_INT_GP_FAULT,
            14 => X86InterruptVector::X86_INT_PAGE_FAULT,
            15 => X86InterruptVector::X86_INT_RESERVED,
            16 => X86InterruptVector::X86_INT_FPU_FP_ERROR,
            17 => X86InterruptVector::X86_INT_ALIGNMENT_CHECK,
            18 => X86InterruptVector::X86_INT_MACHINE_CHECK,
            19 => X86InterruptVector::X86_INT_SIMD_FP_ERROR,
            20 => X86InterruptVector::X86_INT_VIRT,
            31 => X86InterruptVector::X86_INT_MAX_INTEL_DEFINED,
            32 => X86InterruptVector::X86_INT_PLATFORM_BASE,
            0xef => X86InterruptVector::X86_INT_PLATFORM_MAX,
            0xff => X86InterruptVector::X86_INT_MAX,
            _ => X86InterruptVector::X86_INT_MAX,
        }
    }
}
