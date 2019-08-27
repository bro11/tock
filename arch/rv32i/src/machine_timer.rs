//! Create a timer using the Machine Timer registers.

use kernel::common::cells::OptionalCell;
use kernel::common::registers::{register_bitfields, ReadOnly, ReadWrite};
use kernel::common::StaticRef;
use kernel::hil;
use kernel::ReturnCode;

const MTIME_BASE: StaticRef<MachineTimerRegisters> =
    unsafe { StaticRef::new(0x0200_0000 as *const MachineTimerRegisters) };

#[repr(C)]
struct MachineTimerRegisters {
    _reserved0: [u8; 0x4000],
    mtimecmp: ReadWrite<u64, MTimeCmp::Register>,
    _reserved1: [u8; 0x7FF0],
    mtime: ReadOnly<u64, MTime::Register>,
}

register_bitfields![u64,
    MTimeCmp [
        MTIMECMP OFFSET(0) NUMBITS(64) []
    ],
    MTime [
        MTIME OFFSET(0) NUMBITS(64) []
    ]
];

pub static mut MACHINETIMER: MachineTimer = MachineTimer::new();

pub struct MachineTimer<'a> {
    registers: StaticRef<MachineTimerRegisters>,
    client: OptionalCell<&'a hil::time::AlarmClient>,
}

impl MachineTimer<'a> {
    const fn new() -> MachineTimer<'a> {
        MachineTimer {
            registers: MTIME_BASE,
            client: OptionalCell::empty(),
        }
    }

    pub fn handle_interrupt(&self) {
        self.disable_machine_timer();

        self.client.map(|client| {
            client.fired();
        });
    }

    fn disable_machine_timer(&self) {
        // Disable by setting the mtimecmp register to its max value, which
        // we will never hit.
        self.registers
            .mtimecmp
            .write(MTimeCmp::MTIMECMP.val(0xFFFF_FFFF_FFFF_FFFF));
    }
}

impl hil::time::Time<u64> for MachineTimer<'a> {
    type Frequency = hil::time::Freq32KHz;

    fn now(&self) -> u64 {
        self.registers.mtime.get()
    }

    fn max_tics(&self) -> u64 {
        core::u64::MAX
    }
}

impl hil::time::Alarm<'a, u64> for MachineTimer<'a> {
    fn set_client(&self, client: &'a hil::time::AlarmClient) {
        self.client.set(client);
    }

    fn set_alarm(&self, tics: u32) {
        self.registers
            .mtimecmp
            .write(MTimeCmp::MTIMECMP.val(tics as u64));
    }

    fn get_alarm(&self) -> u32 {
        self.registers.mtimecmp.get() as u32
    }

    fn disable(&self) -> ReturnCode {
        self.disable_machine_timer();
        ReturnCode::SUCCESS
    }

    fn is_enabled(&self) -> bool {
        // Check if mtimecmp is the max value. If it is, then we are not armed,
        // otherwise we assume we have a value set.
        self.registers.mtimecmp.get() != 0xFFFF_FFFF_FFFF_FFFF
    }
}
