use patina_3mf::project_settings::color::Color;
use patina_3mf::settings_id::filament_settings_id::{
    FilamentBrand, FilamentMaterial, FilamentSettingsId,
};
use patina_3mf::settings_id::nozzle::Nozzle;
use patina_3mf::settings_id::print_settings_id::{PrintQuality, PrintSettingsId};
use patina_3mf::settings_id::printer::Printer;
use patina_3mf::settings_id::printer_settings_id::PrinterSettingsId;
use patina_bambu::BambuFilament;

pub const SETTINGS_PRINTER: Printer = Printer::A1Mini;
pub const SETTINGS_NOZZLE: Nozzle = Nozzle::Nozzle0_4;
pub const SETTINGS_PLATE_WIDTH: f64 = 180.0;
pub const SETTINGS_PLATE_HEIGHT: f64 = 180.0;

pub fn settings_machine() -> PrinterSettingsId {
    let mut machine = PrinterSettingsId::new(SETTINGS_PRINTER.clone());
    machine.nozzle = Some(SETTINGS_NOZZLE.clone());
    machine
}

pub fn settings_process() -> PrintSettingsId {
    PrintSettingsId::new(
        0.2,
        PrintQuality::Standard,
        SETTINGS_PRINTER.clone(),
        SETTINGS_NOZZLE,
    )
}
fn pla_basic() -> FilamentSettingsId {
    FilamentSettingsId::new(
        FilamentBrand::Bambu,
        FilamentMaterial::PlaBasic,
        SETTINGS_PRINTER.clone(),
    )
}

pub fn settings_primary_filament() -> BambuFilament {
    let mut filament = BambuFilament::new();
    filament.color(Some(Color::new(90, 68, 177)));
    filament.support(Some(false));
    filament.settings_id(Some(pla_basic()));
    filament.diameter(Some(1.75));
    filament.shrink(Some("100%".to_string()));
    filament
}
