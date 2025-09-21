use std::{
    error::Error, ffi::{CStr, CString}, io::Stdout, ptr
};

use crate::lvm::lvmbind::{
    BDLVMSEGdata, GError, _GError, bd_lvm_init, bd_lvm_lvcreate, bd_lvm_lvdata_free, bd_lvm_lvs_tree, bd_lvm_pvdata_free, bd_lvm_pvs, bd_lvm_vgdata_free, bd_lvm_vginfo, bd_lvm_vgs, GDBusError
};

mod lvmbind {
    #![allow(unsafe_op_in_unsafe_fn)]
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(improper_ctypes)]
    #![allow(unsafe_code)]
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub struct LvmExtraArg {
    pub opt: String,
    pub value: String,
}

pub struct LvmPVData {
    pub pv_name: String,
    pub vg_name: String,
}

#[derive(Clone, Default)]
pub struct LvmLvData {
    pub lv_name: String,
    pub vg_name: String,
    pub size: u64, // bytes
    pub attr: String,
    pub segtype: String,
    pub uuid: String,
    pub lv_segs: Vec<LvmlvSegData>,
}

#[derive(Clone)]
pub struct LvmlvSegData {
    pub pvdev: String,
    pub pv_start_pe: u64,
    pub size_pe: u64,
}

#[derive(Clone, Default)]
pub struct LvmVgData {
    pub name: String,
    pub free: u64, // bytes
    pub size: u64, // bytes
    pub pv_count: u64,
    attr: String,
    uuid: String,
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
    use std::process::Command;

    let mut vgdisplay = Command::new("/usr/sbin/vgs");    
    let result= vgdisplay.args([
            vg_name,            
            "--headings", "none",
            "--separator", ",",
            "--reportformat", "basic",
            "-a", "--units", "B",
            "-o", "vg_name,vg_size,vg_free,pv_count,vg_attr,vg_uuid",
        ])
        .output();               
   
    match result {
        Ok(o) => {                     
            let s: std::borrow::Cow<'_, str> = String::from_utf8_lossy(&o.stdout);                  
            match parse_vgdo(&s) {
                Ok(s) => s,
                Err(e) => panic!("{e}"),
            }
        }
        Err(e) => panic!("{e}"),
    }
}

//
// VG,VSize,VFree,#PV,Attr,VG UUID
// vgssd_virt,120028397568B,54530146304B,1,wz--n-,ZFcFCx-fW2F-sWq6-PVy1-8PN2-2CVt-epVMVt
//
fn parse_vgdo(s: &std::borrow::Cow<'_, str>) -> Result<LvmVgData,  &'static str > {
    
    let err = "failed to parse/split vgdisplay output";

    if s.len() < 1 {
        return Err(err);
    }
    let v: Vec<&str> = s.split(",").collect();
    if v.len() < 6 {
        return Err(err);
    }

    let lvmvgdata: LvmVgData = LvmVgData {
        name: v.get(0).ok_or(err)?.trim().to_string(),
        size: parse_ds(v.get(1).ok_or(err)?.trim())?,      
        free: parse_ds(v.get(2).ok_or(err)?.trim())?,        
        pv_count: v.get(3).ok_or(err)?.trim().to_string().parse::<u64>().unwrap_or(0),
        attr: v.get(4).ok_or(err)?.trim().to_string(),
        uuid: v.get(5).ok_or(err)?.trim().to_string(),        
    };

    return Ok(lvmvgdata);
}

fn parse_ds(s: &str) -> Result<u64, &'static str> {
    let num = &s[0..(s.len() - 1)]; //  drop last char, eg. '123321B'
    let num = num.to_string().parse::<u64>();
    match num {
        Ok(num) => Ok(num),
        Err(_) => Err("Failed to convert string to int"),
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

pub fn get_pvs() -> Vec<LvmPVData> {
    let mut pv_list: Vec<LvmPVData> = Vec::<LvmPVData>::new();

    unsafe {
        let error: *mut *mut GError = ptr::null_mut();
        let mut lvm_pv_arr = bd_lvm_pvs(error);

        if lvm_pv_arr.is_null() {
            return pv_list;
        }

        while !(*lvm_pv_arr).is_null() {
            let lvm_pv_data = *lvm_pv_arr;
            let pv_item: LvmPVData = LvmPVData {
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

//
// Create logical volumne
//
pub fn create_lv(
    lv: &String,
    vg: &String,
    size: u64,
    segtype: &String,
    pvl: &Vec<String>,
    extra: &Vec<LvmExtraArg>,
) -> Result<String, &'static str> {
    let vg_cstr = CString::new(vg.as_str()).expect("CString::new fault");
    let vg_cstr = vg_cstr.as_ptr();

    let lv_cstr = CString::new(lv.as_str()).expect("CString::new fault");
    let lv_cstr = lv_cstr.as_ptr();

    let sg_cstr = CString::new(segtype.as_str()).expect("CString::new fault");
    let sg_cstr = sg_cstr.as_ptr();

    let mut in_pv_list: String = String::from("");
    for pvdev in pvl.iter() {
        todo!("pvdevlist cause segfault. *mut ");
        in_pv_list.push_str(format!("{} ", &pvdev).as_str());
    }
    let in_pv_list: &str = in_pv_list.trim();

    let pvl_cstr: CString = CString::new(in_pv_list).expect("CString::new fault");
    let pvl_tmp_ptr = pvl_cstr.into_raw();
    // TODO cant use CString for pv_list, cause segfault.
    let pv_list_ptr: *mut *const i8 = pvl_tmp_ptr.cast();
    let pv_list_ptr: *mut *const i8 = ptr::null_mut();

    let mut result: Result<String, &str> = Result::Ok(String::from("Created LV."));
    unsafe {
        let error: *mut _GError = ptr::null_mut();
        let mut error: Box<*mut _GError> = Box::new(error);
        let error: &mut *mut _GError = &mut *error;

        if bd_lvm_lvcreate(
            vg_cstr,
            lv_cstr,
            size,
            sg_cstr,
            pv_list_ptr,
            ptr::null_mut(),
            error,
        ) != 1
        {
            if !error.is_null() {
                let ptr_gerror = *error;
                let message = CStr::from_ptr((*ptr_gerror).message)
                    .to_str()
                    .clone()
                    .unwrap();
                // free the error ptr
                //g_error_free(*error);  // no, Box free when go out of scope.
                result = Err(message);
            } else {
                result = Err("Failed to create LV. Error unknown...");
            }

            // retake pointer to free memory
            let _ = CString::from_raw(pvl_tmp_ptr);
        }
    };

    return result;
}

pub fn get_lvs() -> Vec<LvmLvData> {
    let mut lv_list: Vec<LvmLvData> = Vec::<LvmLvData>::new();

    unsafe {
        let error: *mut *mut GError = ptr::null_mut();

        let mut lvm_lv_arr = bd_lvm_lvs_tree(ptr::null_mut(), error);

        if lvm_lv_arr.is_null() {
            return lv_list;
        }

        while !(*lvm_lv_arr).is_null() {
            let lvm_lv_data = *lvm_lv_arr;
            let lv_item: LvmLvData = LvmLvData {
                lv_name: CStr::from_ptr((*lvm_lv_data).lv_name)
                    .to_str()
                    .unwrap()
                    .to_string(),
                vg_name: CStr::from_ptr((*lvm_lv_data).vg_name)
                    .to_str()
                    .unwrap()
                    .to_string(),
                size: (*lvm_lv_data).size,
                attr: CStr::from_ptr((*lvm_lv_data).attr)
                    .to_str()
                    .unwrap()
                    .to_string(),
                segtype: CStr::from_ptr((*lvm_lv_data).segtype)
                    .to_str()
                    .unwrap()
                    .to_string(),
                uuid: CStr::from_ptr((*lvm_lv_data).uuid)
                    .to_str()
                    .unwrap()
                    .to_string(),
                lv_segs: conv_lv_segs((*lvm_lv_data).segs),
            };

            lv_list.push(lv_item);
            bd_lvm_lvdata_free(lvm_lv_data);
            lvm_lv_arr = lvm_lv_arr.add(1);
        }
    }

    return lv_list;
}

pub fn conv_lv_segs(mut segs_arr: *mut *mut BDLVMSEGdata) -> Vec<LvmlvSegData> {
    let mut segs_list: Vec<LvmlvSegData> = Vec::<LvmlvSegData>::new();

    if segs_arr.is_null() {
        return segs_list;
    }

    unsafe {
        while !(*segs_arr).is_null() {
            let lvm_lv_segdata = *segs_arr;
            let seg_item: LvmlvSegData = LvmlvSegData {
                pvdev: CStr::from_ptr((*lvm_lv_segdata).pvdev)
                    .to_str()
                    .unwrap()
                    .to_string(),
                pv_start_pe: (*lvm_lv_segdata).pv_start_pe,
                size_pe: (*lvm_lv_segdata).size_pe,
            };
            segs_list.push(seg_item);
            segs_arr = segs_arr.add(1);
        }
    }

    return segs_list;
}

// Convinient functions
pub fn find_pvs_by_vg(vg_name: &String, pv_list: &Vec<LvmPVData>) -> Vec<String> {
    let mut pvs_in_vg_list = Vec::<String>::new();

    for pv_item in pv_list {
        if vg_name.eq(&pv_item.vg_name) {
            pvs_in_vg_list.push(pv_item.pv_name.clone());
        }
    }

    return pvs_in_vg_list;
}

pub fn find_lvs_by_vg(vg_name: &String, lv_list: &Vec<LvmLvData>) -> Vec<String> {
    let mut lvs_in_vg_list = Vec::<String>::new();

    for lv_item in lv_list {
        if vg_name.eq(&lv_item.vg_name) {
            lvs_in_vg_list.push(lv_item.lv_name.clone());
        }
    }

    return lvs_in_vg_list;
}

pub fn get_lvinfo_by_vg(vg_name: &String, lv_list: &Vec<LvmLvData>) -> Vec<LvmLvData> {
    let mut lvs_in_vg_list = Vec::<LvmLvData>::new();

    for lv_item in lv_list {
        if vg_name.eq(&lv_item.vg_name) {
            lvs_in_vg_list.push(lv_item.clone());
        }
    }

    return lvs_in_vg_list;
}

#[cfg(test)]
mod tests {
    use crate::lvm::{LvmVgData, parse_vgdo};

    #[test]
    fn test_parse_vgdo() {
        // VG,VSize,VFree,#PV,Attr,VG UUID
        let s =
            "vgssd_virt,120028397568B,54530146304B,1,wz--n-,ZFcFCx-fW2F-sWq6-PVy1-8PN2-2CVt-epVMVt";
        let s: std::borrow::Cow<'_, str> = std::borrow::Cow::Borrowed(s);

        let lvm_vg_data: LvmVgData = parse_vgdo(&s).expect("error");
        assert_eq!("vgssd_virt", lvm_vg_data.name);
        assert_eq!(120028397568, lvm_vg_data.size);
        assert_eq!(54530146304, lvm_vg_data.free);
        assert_eq!(1, lvm_vg_data.pv_count);
        assert_eq!("wz--n-", lvm_vg_data.attr);
        assert_eq!("ZFcFCx-fW2F-sWq6-PVy1-8PN2-2CVt-epVMVt", lvm_vg_data.uuid);

        // To few options in result
        let s =
            "vgssd_virt,120028397568B,54530146304B,wz--n-,ZFcFCx-fW2F-sWq6-PVy1-8PN2-2CVt-epVMVt";
        let s: std::borrow::Cow<'_, str> = std::borrow::Cow::Borrowed(s);

        let result = parse_vgdo(&s);
        assert!(result.is_err());

        // bad data in size
        // VG,VSize,VFree,#PV,Attr,VG UUID
        let s =
            "vgssd_virt,120028a397568B,54530146304B,1,wz--n-,ZFcFCx-fW2F-sWq6-PVy1-8PN2-2CVt-epVMVt";
        let s: std::borrow::Cow<'_, str> = std::borrow::Cow::Borrowed(s);
        let result = parse_vgdo(&s);
        assert!(result.is_err());       
    }
}
