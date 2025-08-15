#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]

use std::{
    ffi::{CStr, CString},
    ptr,
};

use ratatui::layout::Size;

// Include bindings generated from "wrapper.h
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub struct NameValue {
    pub name: String,
    pub value: String,
}

pub struct Pv {
    pub pv_name: String,
    pub vg_name: String,
}

pub struct Lv {
    pub lv_name: String,
    pub vg_name: String,
}

pub struct LvmVgData {
    pub name: String,
    pub free: u64, // bytes 
    pub size: u64, // bytes 
    pub pv_count: u64,
}

pub fn init() -> bool {
    unsafe {
        if bd_lvm_init() != 1 {
            return false;
        } else {
            return true;
        }
    }
}

pub fn get_vg_info(vg_name: &String) -> LvmVgData {
    let error: *mut *mut GError = ptr::null_mut();
    let vg_name_ptr = CString::new(vg_name.to_string())
        .expect("failed to make CString")
        .into_raw();

    unsafe {
        let bd_lvm_vg_data = bd_lvm_vginfo(vg_name_ptr, error);
        let lvm_vg_data: LvmVgData = {
            LvmVgData {
                name: CStr::from_ptr((*bd_lvm_vg_data).name)
                    .to_str()
                    .unwrap()
                    .to_string(),
                free: (*bd_lvm_vg_data).free,
                size: (*bd_lvm_vg_data).size,
                pv_count: (*bd_lvm_vg_data).pv_count,
            }
        };
        let _ = CString::from_raw(vg_name_ptr); // free mem
        bd_lvm_vgdata_free(bd_lvm_vg_data);

        return lvm_vg_data;
    }
}

pub fn get_vgs() -> Vec<String> {
    let mut vg_list: Vec<String> = Vec::<String>::new();

    unsafe {
        let error: *mut *mut GError = ptr::null_mut();
        let mut lvm_vg_list = bd_lvm_vgs(error);

        if lvm_vg_list.is_null() {
            return vg_list;
        }

        while !(*lvm_vg_list).is_null() {
            let lvm_vg_data = *lvm_vg_list;
            vg_list.push(
                CStr::from_ptr((*lvm_vg_data).name)
                    .to_str()
                    .unwrap()
                    .to_string(),
            );
            bd_lvm_vgdata_free(lvm_vg_data);
            lvm_vg_list = lvm_vg_list.add(1);
        }
    }

    return vg_list;
}

pub fn get_pvs() -> Vec<Pv> {
    let mut pv_list: Vec<Pv> = Vec::<Pv>::new();

    unsafe {
        let error: *mut *mut GError = ptr::null_mut();
        let mut lvm_pv_arr = bd_lvm_pvs(error);

        if lvm_pv_arr.is_null() {
            return pv_list;
        }

        while !(*lvm_pv_arr).is_null() {
            let lvm_pv_data = *lvm_pv_arr;
            let pv_item: Pv = Pv {
                pv_name: CStr::from_ptr((*lvm_pv_data).pv_name)
                    .to_str()
                    .unwrap()
                    .to_string(),
                vg_name: CStr::from_ptr((*lvm_pv_data).vg_name)
                    .to_str()
                    .unwrap()
                    .to_string(),
            };

            pv_list.push(pv_item);
            bd_lvm_pvdata_free(lvm_pv_data);
            lvm_pv_arr = lvm_pv_arr.add(1);
        }
    }

    return pv_list;
}

pub fn get_lvs() -> Vec<Lv> {
    let mut lv_list: Vec<Lv> = Vec::<Lv>::new();

    unsafe {
        let error: *mut *mut GError = ptr::null_mut();
        let mut lvm_lv_arr = bd_lvm_lvs(ptr::null_mut(), error);

        if lvm_lv_arr.is_null() {
            return lv_list;
        }

        while !(*lvm_lv_arr).is_null() {
            let lvm_lv_data = *lvm_lv_arr;
            let lv_item: Lv = Lv {
                lv_name: CStr::from_ptr((*lvm_lv_data).lv_name)
                    .to_str()
                    .unwrap()
                    .to_string(),
                vg_name: CStr::from_ptr((*lvm_lv_data).vg_name)
                    .to_str()
                    .unwrap()
                    .to_string(),
            };

            lv_list.push(lv_item);
            bd_lvm_lvdata_free(lvm_lv_data);
            lvm_lv_arr = lvm_lv_arr.add(1);
        }
    }

    return lv_list;
}

// Convinient functions
pub fn find_pvs_by_vg(vg_name: &String, pv_list: &Vec<Pv>) -> Vec<String> {
    let mut pvs_in_vg_list = Vec::<String>::new();

    for pv_item in pv_list {
        if vg_name.eq(&pv_item.vg_name) {
            pvs_in_vg_list.push(pv_item.pv_name.clone());
        }
    }

    return pvs_in_vg_list;
}

pub fn find_lvs_by_vg(vg_name: &String, lv_list: &Vec<Lv>) -> Vec<String> {
    let mut lvs_in_vg_list = Vec::<String>::new();

    for lv_item in lv_list {
        if vg_name.eq(&lv_item.vg_name) {
            lvs_in_vg_list.push(lv_item.lv_name.clone());
        }
    }

    return lvs_in_vg_list;
}
