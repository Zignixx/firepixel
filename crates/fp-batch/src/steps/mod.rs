//! Eingebaute Schritte, je Gruppe eine Datei. Spätere Pakete hängen hier **eine** Zeile an
//! (`geometry::register(r);`) und legen ihre Datei daneben an (SPEC-BATCH §6.1). Vor jedem Edit
//! die Datei neu lesen; nie fremde Zeilen ändern.

use crate::step::StepRegistry;

pub mod files;
pub mod metadata;
pub mod optimize;

/// Registriert alle eingebauten Schritte.
pub fn register_all(r: &mut StepRegistry) {
    files::register(r);
    metadata::register(r);
    optimize::register(r);
}
