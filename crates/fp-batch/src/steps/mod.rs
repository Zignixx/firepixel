//! Eingebaute Schritte, je Gruppe eine Datei. Spätere Pakete hängen hier **eine** Zeile an
//! (`geometry::register(r);`) und legen ihre Datei daneben an (SPEC-BATCH §6.1). Vor jedem Edit
//! die Datei neu lesen; nie fremde Zeilen ändern.

use crate::step::StepRegistry;

pub mod background;
pub mod files;
pub mod metadata;
pub mod optimize;
pub mod overlay;
pub mod geometry; // BATCH-4
pub mod adjust; // BATCH-4
pub mod script; // BATCH-4
pub mod anim; // BATCH-7
pub mod gifopt; // BATCH-8
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
}
