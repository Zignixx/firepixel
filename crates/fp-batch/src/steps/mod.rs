//! Eingebaute Schritte, je Gruppe eine Datei. Spätere Pakete hängen hier **eine** Zeile an
//! (`geometry::register(r);`) und legen ihre Datei daneben an (SPEC-BATCH §6.1). Vor jedem Edit
//! die Datei neu lesen; nie fremde Zeilen ändern.

use crate::step::StepRegistry;

pub mod adjust; // BATCH-4
pub mod anim; // BATCH-7
pub mod animfx; // B-3
pub mod background;
pub mod condition; // B-4
pub mod document; // B-3
pub mod extra; // BA-109
pub mod fileops; // B-3
pub mod files;
pub mod format; // B-3
pub mod geometry; // BATCH-4
pub mod gifopt; // BATCH-8
pub mod metadata;
pub mod multipage; // B-4
pub mod optimize;
pub mod overlay;
pub mod pixel; // B-3
pub mod script; // BATCH-4
pub mod script_extra; // B-4
pub mod straighten; // B-4
pub mod vector; // BATCH-9

/// Registriert alle eingebauten Schritte.
pub fn register_all(r: &mut StepRegistry) {
    files::register(r);
    metadata::register(r);
    optimize::register(r);
    background::register(r);
    overlay::register(r);
    geometry::register(r); // BATCH-4
    adjust::register(r); // BATCH-4
    script::register(r); // BATCH-4
    anim::register(r); // BATCH-7
    gifopt::register(r); // BATCH-8
    vector::register(r); // BATCH-9
    extra::register(r); // BA-109
    fileops::register(r); // B-3
    document::register(r); // B-3
    pixel::register(r); // B-3
    animfx::register(r); // B-3
    format::register(r); // B-3
    multipage::register(r); // B-4
    condition::register(r); // B-4
    script_extra::register(r); // B-4
}
