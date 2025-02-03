use crate::system::System;

use crate::config::output::{
    BootGroupStateOutput, BootStateOutput, SlotStateOutput, SystemStateOutput,
};

pub fn state_from_system(system: &System) -> SystemStateOutput {
    let boot_flow = system.boot_flow().name().to_owned();
    let slots = system
        .slots()
        .iter()
        .map(|(_, slot)| {
            (
                slot.name().to_owned(),
                SlotStateOutput {
                    active: Some(slot.active()),
                },
            )
        })
        .collect();
    let active_boot_group = system
        .active_boot_entry()
        .map(|idx| system.boot_entries()[idx].name().to_owned());
    let default_boot_group = Some(
        system.boot_entries()[system.boot_flow().get_default(system).unwrap()]
            .name()
            .to_owned(),
    );
    let boot_groups = system
        .boot_entries()
        .iter()
        .map(|(_, group)| (group.name().to_owned(), BootGroupStateOutput {}))
        .collect();
    SystemStateOutput::new(slots).with_boot(Some(BootStateOutput {
        boot_flow,
        active_group: active_boot_group,
        default_group: default_boot_group,
        groups: boot_groups,
    }))
}
