// Pas de console en release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    launcher_pro_lib::run()
}
