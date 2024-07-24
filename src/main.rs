#![windows_subsystem = "windows"] // disable the console on windows when running exe

use std::{
    sync::mpsc::{self, SyncSender},
    time::Duration,
};

use windows::{
    core::PCSTR,
    Win32::Graphics::Gdi::{ChangeDisplaySettingsA, EnumDisplaySettingsA, DEVMODEA, DMDFO_DEFAULT},
};

use tray_item::{IconSource, TrayItem};

enum Message {
    Quit,
    SwitchResolution(DEVMODEA),
}

const FAVORITES: [(u32, u32, u32); 4] = [
    (1280, 720, 60),
    (1920, 1080, 60),
    (2560, 1440, 60),
    (3840, 2160, 60),
];

fn main() {
    let mut tray =
        TrayItem::new("Resolution Switcher", IconSource::Resource("checker-icon")).unwrap();

    // resolution options
    let dev_modes = enum_display_settings()
        .iter()
        .cloned()
        // DM_DISPLAYFIXEDOUTPUT controls how to display lower res onto higher res screen
        // for parity with python ver, default (as opposed to stretching or centering)
        .filter(|mode| unsafe {
            mode.dmDisplayFrequency == 60
                && mode.Anonymous1.Anonymous2.dmDisplayFixedOutput == DMDFO_DEFAULT
        })
        .collect::<Vec<DEVMODEA>>();

    let mut fav_devmodes = vec![];
    let mut other_devmodes = vec![];

    for d in &dev_modes {
        if FAVORITES.contains(&(d.dmPelsWidth, d.dmPelsHeight, d.dmDisplayFrequency)) {
            fav_devmodes.push(*d);
        } else {
            other_devmodes.push(*d);
        }
    }

    let (tx, rx) = mpsc::sync_channel(1);

    // favorites
    tray.inner_mut().add_label("Favorites").unwrap();
    add_modes(&mut tray, &tx, fav_devmodes);
    tray.inner_mut().add_separator().unwrap();

    // others
    add_modes(&mut tray, &tx, other_devmodes);

    // quit button
    tray.inner_mut().add_separator().unwrap();
    let quit_tx = tx.clone();
    tray.add_menu_item("Quit", move || {
        quit_tx.send(Message::Quit).unwrap();
    })
    .unwrap();

    // main loop
    loop {
        match rx.recv() {
            Ok(Message::Quit) => {
                break;
            }
            Ok(Message::SwitchResolution(devmode)) => {
                change_display_settings(devmode);
            }
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn change_display_settings(mode: DEVMODEA) {
    unsafe {
        // CDS type 0 just changes the mode
        // (other types do other stuff https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-changedisplaysettingsa)
        ChangeDisplaySettingsA(Some(&mode), windows::Win32::Graphics::Gdi::CDS_TYPE(0));
    }
}

fn enum_display_settings() -> Vec<DEVMODEA> {
    let mut out = vec![];
    let mut cur_mode: u32 = 0;

    while let Some(devmode) = get_display_setting(cur_mode) {
        out.push(devmode);
        cur_mode += 1;
    }

    out
}

fn get_display_setting(index: u32) -> Option<DEVMODEA> {
    let mut devmode: DEVMODEA = Default::default();
    unsafe {
        if EnumDisplaySettingsA(
            PCSTR::null(), // passing null here means default display device
            windows::Win32::Graphics::Gdi::ENUM_DISPLAY_SETTINGS_MODE(index),
            &mut devmode,
        )
        .as_bool()
        {
            Option::Some(devmode)
        } else {
            Option::None
        }
    }
}

fn add_modes(tray: &mut TrayItem, tx: &SyncSender<Message>, modes: Vec<DEVMODEA>) {
    for devmode in modes {
        let (w, h, r) = (
            devmode.dmPelsWidth,
            devmode.dmPelsHeight,
            devmode.dmDisplayFrequency,
        );
        let tx_clone = tx.clone();
        tray.add_menu_item(format!("{}x{}@{}", w, h, r).as_str(), move || {
            tx_clone.send(Message::SwitchResolution(devmode)).unwrap();
        })
        .unwrap()
    }
}
