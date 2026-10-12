> **Firepixel-Kopie (FV-8, 12.10.2026)** von egui-wgpu 0.36.2 (MIT/Apache-2.0), eingebunden über
> `[patch.crates-io]` im Workspace. Einzige Änderung: der Schalter `SOFTWARE_RASTERIZER_TUNING`
> (Vorgabe aus, Firepixel schaltet ihn in `fp_ui::run` ein). Er wirkt nur auf Software-Rasterern
> (`DeviceType::Cpu`, z. B. „Microsoft Basic Render Driver“ ohne Grafikkarte/in Fernsitzungen):
> * `src/setup.rs`: die Vorgabe-Gerätebeschreibung fordert dort `MAPPABLE_PRIMARY_BUFFERS` an;
> * `src/renderer.rs`: Vertex-/Indexdaten je Bild in CPU-sichtbare Puffer (`MAP_WRITE`) statt über
>   eine Pufferkopie (dort ~7 MB/s); je Texturoption eine Pipeline mit nachgebildeter Abtastung;
> * `src/egui.wgsl`: `override`-Konstanten `MANUAL_SAMPLING`, `MAG_LINEAR`, `MIN_LINEAR`,
>   `WRAP_MODE`; Abtastung mit `textureLoad` (nächster Nachbar bzw. bilinear, Randmodi) statt
>   `textureSample` (dort ~20-mal langsamer).
> Gemessen (`cratesp-ui	ests	ippen_messung.rs`): Bild beim Tippen auf einer großen Seite
> 230 → 31 ms. Bei einem Update von egui die Änderungen nachziehen oder die Kopie entfernen.

# egui-wgpu

[![Latest version](https://img.shields.io/crates/v/egui-wgpu.svg)](https://crates.io/crates/egui-wgpu)
[![Documentation](https://docs.rs/egui-wgpu/badge.svg)](https://docs.rs/egui-wgpu)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

This crates provides bindings between [`egui`](https://github.com/emilk/egui) and [wgpu](https://crates.io/crates/wgpu).

This was originally hosted at https://github.com/hasenbanck/egui_wgpu_backend
