use rdif_intc::Intc;
use rdrive::{module_driver, probe::OnProbeError, register::ProbeFdt};

static mut TIMER_IRQ: Option<irq_framework::IrqId> = None;

pub fn systick_irq() -> irq_framework::IrqId {
    unsafe { TIMER_IRQ.expect("systick irq is not initialized") }
}

module_driver!(
    name: "ARMv8 Timer",
    level: ProbeLevel::PreKernel,
    priority: ProbePriority::TIMER,
    probe_kinds: &[
        ProbeKind::Fdt {
            compatibles: &["arm,armv8-timer"],
            on_probe: probe
        }
    ],
);

pub(crate) fn setup_systick_irq() {
    // The runtime IRQ framework owns timer PPI enablement after registering
    // the OS timer handler. Keep this hook as the architecture-local setup
    // point without unmasking an interrupt that may not yet have an action.
}

fn probe(probe: ProbeFdt<'_>) -> Result<(), OnProbeError> {
    let (fdt, dev) = probe.into_parts();
    let intc_id = dev.descriptor.irq_parent.unwrap();

    let mut intc = rdrive::get::<Intc>(intc_id).unwrap().lock().unwrap();
    let interrupts = fdt.interrupts();

    let irq_idx = someboot::timer::aarch64_timer_irq_index(someboot::timer::aarch64_timer_mode());
    let irq = &interrupts[irq_idx].specifier;
    let translation = intc
        .translate_fdt(irq)
        .map_err(|err| OnProbeError::other(alloc::format!("invalid timer IRQ: {err:?}")))?;
    intc.configure(&translation).map_err(|err| {
        OnProbeError::other(alloc::format!("failed to configure timer IRQ: {err:?}"))
    })?;
    let irq = translation.id;
    debug!("Armv8 timer irq: {:?}", irq);
    unsafe {
        TIMER_IRQ = Some(irq);
    }
    Ok(())
}
