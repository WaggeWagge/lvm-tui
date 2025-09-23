//
// Functions for talking with lvm and get info about logical volumns, PV etc.
//
// Since it seams lib (lvmapp.h ) does not exist anymore,
// "difficult to maintain acc to thread/mailinglist 2025",
// remaining option seams to be either dbus or "good old" execv(lvs/lvcreate)
// etc.
//
// Started with libblockdev (clib), but it turns out it spawns lvm cmds anyway.
// Hence 'Command' in rust dropping deps to c-libs, bindgen. Commands used are lvs, pvs and vgs.
//

use std::collections::HashMap;

const VGDISPLAY_BIN: &str = "/usr/sbin/vgs";
const PVS_BIN: &str = "/usr/sbin/pvs";
const LVS_BIN: &str = "/usr/sbin/lvs";
const LVCREATE_BIN: &str = "/usr/sbin/lvcreate";
//const LVCREATE_BIN: &str = "/tmp/foo.sh";

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
    pub stripes: u16,
    pub data_stripes: u16,
    pub parent_lv: String,
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
    pub attr: String,
    pub uuid: String,
}

fn run_cmd(cmd: &str, args: &[&str]) -> Result<std::process::Output, std::io::Error> {
    let mut command: std::process::Command = std::process::Command::new(cmd);
    command.args(args).output()
}

pub fn get_vg_info(vg_name: &String) -> LvmVgData {
    let args: [&str; 12] = [
        vg_name,
        "--headings",
        "none",
        "--separator",
        ",",
        "--reportformat",
        "basic",
        "-a",
        "--units",
        "B",
        "-o",
        "vg_name,vg_size,vg_free,pv_count,vg_attr,vg_uuid",
    ];

    match run_cmd(VGDISPLAY_BIN, &args) {
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
fn parse_vgdo(s: &std::borrow::Cow<'_, str>) -> Result<LvmVgData, &'static str> {
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
        size: parseu64_ds(v.get(1).ok_or_else(|| err)?.trim())?,
        free: parseu64_ds(v.get(2).ok_or_else(|| err)?.trim())?,
        pv_count: v
            .get(3)
            .ok_or(err)?
            .trim()
            .to_string()
            .parse::<u64>()
            .unwrap_or(0),
        attr: v.get(4).ok_or_else(|| err)?.trim().to_string(),
        uuid: v.get(5).ok_or_else(|| err)?.trim().to_string(),
    };

    return Ok(lvmvgdata);
}

fn parseu64_ds(s: &str) -> Result<u64, &'static str> {
    let num = &s[0..(s.len() - 1)]; //  drop last char, eg. '123321B'
    let num = num.to_string().parse::<u64>();
    match num {
        Ok(num) => Ok(num),
        Err(_) => Err("Failed to convert string to int"),
    }
}

fn parseu16(s: &str) -> Result<u16, &'static str> {
    let num = &s[0..(s.len())]; //  
    let num = num.to_string().parse::<u16>();
    match num {
        Ok(num) => Ok(num),
        Err(_) => Err("Failed to convert string to u16"),
    }
}

//
// VG
// vgssd_virt
// vg2
//
fn parse_vgso(s: &std::borrow::Cow<'_, str>) -> Result<Vec<String>, &'static str> {
    if s.len() < 1 {
        return Ok(Vec::<String>::new());
    }

    let v_vgs = s.lines().map(|line| line.trim().to_string()).collect();

    return Ok(v_vgs);
}

//
// Return all volumne groups found.
//
pub fn get_vgs() -> Vec<String> {
    let args: [&str; 11] = [
        "--headings",
        "none",
        "--separator",
        ",",
        "--reportformat",
        "basic",
        "-a",
        "--units",
        "B",
        "-o",
        "vg_name",
    ];

    match run_cmd(VGDISPLAY_BIN, &args) {
        Ok(o) => {
            let s: std::borrow::Cow<'_, str> = String::from_utf8_lossy(&o.stdout);
            match parse_vgso(&s) {
                Ok(s) => s,
                Err(e) => panic!("{e}"),
            }
        }
        Err(e) => panic!("{e}"),
    }
}

//
// PV, VG
// /dev/sda1,vg01
// /dev/sdx,
// /dev/sd1
//
fn parse_pvso(s: &std::borrow::Cow<'_, str>) -> Result<Vec<LvmPVData>, &'static str> {
    if s.len() < 1 {
        return Ok(Vec::<LvmPVData>::new());
    }

    let res: Result<Vec<LvmPVData>, &'static str> = s
        .lines()
        .filter(|&line| !line.trim().is_empty())
        .map(|line| {
            let data: Vec<&str> = line.trim().split(",").collect();
            let err = "Could not parse 'pv_name' from lines, unexpected";
            let pv_name = data.get(0).ok_or(err)?.trim().to_string();
            let vg_name = data.get(1).unwrap_or(&"").trim().to_string(); // ok, pv may not have vg
            let lvm_pv_data: LvmPVData = {
                LvmPVData {
                    pv_name: pv_name,
                    vg_name: vg_name,
                }
            };
            Ok::<LvmPVData, &'static str>(lvm_pv_data)
        })
        .collect();

    return res;
}

pub fn get_pvs() -> Vec<LvmPVData> {
    let args: [&str; 11] = [
        "--headings",
        "none",
        "--separator",
        ",",
        "--reportformat",
        "basic",
        "-a",
        "--units",
        "B",
        "-o",
        "pv_name,vg_name",
    ];

    match run_cmd(PVS_BIN, &args) {
        Ok(o) => {
            let s: std::borrow::Cow<'_, str> = String::from_utf8_lossy(&o.stdout);
            let result: Result<Vec<LvmPVData>, &'static str> = parse_pvso(&s);
            match result {
                Ok(pvs) => {
                    return pvs;
                }
                Err(e) => panic!("{e}"),
            }
        }
        Err(e) => panic!("{e}"),
    }
}

// output ex:
// LV,VG,LSize,Attr,Type,LV UUID,#Str,#DStr
// [lvpub_rmeta_3],vg04_1tbdisks,4194304B,ewi-aor---,linear,qhuhv2-Kdro-dySw-L8d4-uSLJ-8ReD-rgYbD9,1,1,parentlv,/dev/sda1:7424-9983
// lvpub,vg04_1tbdisks,536875106304B,rwi-aor---,raid5,0iPPdB-17pl-7SKc-3rwU-EiBd-10fZ-WheGSZ,4,3,,[lvpub_rimage_0]:0-42666,[lvpub_rimage_1]:0-42666,[lvpub_rimage_2]:0-42666,[lvpub_rimage_3]:0-42666
//
fn parse_lvso(s: &std::borrow::Cow<'_, str>) -> Result<Vec<LvmLvData>, &'static str> {
    if s.len() < 1 {
        return Ok(Vec::<LvmLvData>::new());
    }

    let mut lvdata_set = HashMap::<String, LvmLvData>::new();

    for line in s.lines().filter(|&line| !line.trim().is_empty()) {
        let err = "failed to parse Logical Volumne data";
        let mut data: Vec<&str> = line.trim().split(",").collect();
        let mut d_iter: std::slice::IterMut<'_, &str> = data.iter_mut();
        let lv_name = d_iter.next().ok_or_else(|| err)?.to_string();
        let vg_name = d_iter.next().ok_or_else(|| err)?.to_string();
        let size = parseu64_ds(d_iter.next().ok_or_else(|| err)?)?;
        let attr = d_iter.next().ok_or_else(|| err)?.to_string();
        let segtype = d_iter.next().ok_or_else(|| err)?.to_string();
        let uuid = d_iter.next().ok_or_else(|| err)?.to_string();
        let stripes = parseu16(
            d_iter
                .next()
                .ok_or_else(|| "failed to parse 'stripes'")?
                .trim(),
        )?;
        let data_stripes = parseu16(
            d_iter
                .next()
                .ok_or_else(|| "failed to parse 'data_stripes")?
                .trim(),
        )?;
        let parent_lv = d_iter.next().ok_or_else(|| err)?.to_string();
        let lv_data = LvmLvData {
            lv_name: lv_name.clone(),
            vg_name: vg_name,
            size: size,
            attr: attr,
            segtype: segtype,
            uuid: uuid,
            lv_segs: Vec::<LvmlvSegData>::new(),
            stripes: stripes,
            data_stripes: data_stripes,
            parent_lv: parent_lv,
        };

        // add lv to map if it does not exist before.
        if !lvdata_set.contains_key(&lv_name) {
            lvdata_set.insert(lv_name.clone(), lv_data);
        }

        // now remaining are devices
        let v_lv_segs: Vec<LvmlvSegData> = parse_lvso_segs(d_iter)?;

        // add segs to lvdata map.
        let mut lv_data = lvdata_set.get(&lv_name).unwrap().clone(); // just unwrap here, unexpected to fail here.
        for lv_segs in v_lv_segs {
            lv_data.lv_segs.push(lv_segs);
        }
        lvdata_set.insert(lv_name.clone(), lv_data);
    }

    let res: Vec<LvmLvData> = lvdata_set.values().cloned().collect::<Vec<LvmLvData>>();

    return Ok(res);
}

//
// Get '/dev/sda1:7424-9983' part from
// [lvpub_rmeta_3],vg04_1tbdisks,4194304B,ewi-aor---,linear,qhuhv2-Kdro-dySw-L8d4-uSLJ-8ReD-rgYbD9,1,1,parentlv,/dev/sda1:7424-9983
// or [lvpub_rimage_0]:0-42666,[lvpub_rimage_1]:0-42666 ...
//
fn parse_lvso_segs(
    d_iter: std::slice::IterMut<'_, &str>,
) -> Result<Vec<LvmlvSegData>, &'static str> {
    let mut v_lv_segs: Vec<LvmlvSegData> = Vec::<LvmlvSegData>::new();

    for seg in d_iter {
        let seg_data: Vec<&str> = seg.trim().split(",").collect();

        for dev_data in seg_data {
            let pos = dev_data
                .find(":")
                .ok_or_else(|| "failed to extents for pvdev")?;
            let pvdev: &str = &dev_data[0..pos];
            let remainder: &str = &dev_data[(pos + 1)..dev_data.len()];
            let extent_pos = remainder.find("-");
            let mut ext_start: u64 = 0;
            let mut ext_end: u64 = 0;
            match extent_pos {
                Some(extent_pos) => {
                    ext_start = remainder[0..(extent_pos)].parse::<u64>().unwrap_or(0);
                    ext_end = remainder[(extent_pos + 1)..remainder.len()]
                        .parse::<u64>()
                        .unwrap_or(0);
                }
                None => (), // noop, init to 0 already
            }

            v_lv_segs.push(LvmlvSegData {
                pvdev: pvdev.to_string(),
                pv_start_pe: ext_start,
                size_pe: ext_end,
            });
        }
    }

    Ok(v_lv_segs)
}

//
// lvs --headings none --separator ',' -a --units B -o lv_name,vg_name,size,attr,segtype,uuid,stripes,data_stripes
//
// ex:
// [lvdata_mpriv_rmeta_3],vgdata01,4194304B,ewi-aor---,linear,sDjQnR-1sCa-zHKG-mWQC-rO5C-pb7a-OrEjIE,1,1,lvdata_mpriv
//
pub fn get_lvs() -> Vec<LvmLvData> {
    let args: [&str; 11] = [
        "--headings",
        "none",
        "--separator",
        ",",
        "--reportformat",
        "basic",
        "-a",
        "--units",
        "B",
        "-o",
        "lv_name,vg_name,size,attr,segtype,uuid,stripes,data_stripes,lv_parent,seg_le_ranges",
    ];

    match run_cmd(LVS_BIN, &args) {
        Ok(o) => {
            let s: std::borrow::Cow<'_, str> = String::from_utf8_lossy(&o.stdout);
            let result: Result<Vec<LvmLvData>, &'static str> = parse_lvso(&s);
            match result {
                Ok(pvs) => {
                    return pvs;
                }
                Err(e) => panic!("{e}"),
            }
        }
        Err(e) => panic!("{e}"),
    }
}

//
// Create logical volumne
//
pub fn create_lv(
    lv: &String,
    vg: &String,
    size: u64,
    size_unit: &String,
    segtype: &String,
    pvl: &Vec<String>,
    extra: &Vec<LvmExtraArg>,
) -> Result<String, String> {
    let mut args = Vec::<&str>::new();
    // lvcreate --type raid1 -m 1 -L 10G -n lvvirt_archjol vg04_1tbdisks /dev/sdd1 /dev/sde1
    args.push("--type");
    args.push(segtype);
    for e in extra {
        args.push(&e.opt);
        args.push(&e.value);
    }
    args.push("-L");
    let size_str = format!("{}{}", size, size_unit);
    args.push(&size_str);
    args.push("-n");
    args.push(lv);
    args.push(vg);
    let mut pvs = String::new();
    for pvdev in pvl {
        pvs = pvdev.to_owned() + &" ".to_string();
    }
    if pvs.len() > 0 {
        args.push(pvs.trim());
    }

    match run_cmd(LVCREATE_BIN, &args) {
        Ok(o) => {
            if !o.status.success() {
                let es: std::borrow::Cow<'_, str> = String::from_utf8_lossy(&o.stderr);
                return Err(es.into_owned());
            }
            Ok("Created lv".to_string())
        }
        Err(e) => panic!("{e}"),
    }
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

    use crate::lvm::{LvmVgData, LvmlvSegData, parse_lvso, parse_pvso, parse_vgdo, parse_vgso};

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
        let s = "vgssd_virt,120028a397568B,54530146304B,1,wz--n-,ZFcFCx-fW2F-sWq6-PVy1-8PN2-2CVt-epVMVt";
        let s: std::borrow::Cow<'_, str> = std::borrow::Cow::Borrowed(s);
        let result = parse_vgdo(&s);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_vgso() {
        let s = "  vg03_backups\n  vg04_1tbdisks\n  vgdata01\n  vgroot";
        let s: std::borrow::Cow<'_, str> = std::borrow::Cow::Borrowed(s);

        let lvm_vgs = parse_vgso(&s).expect("error");
        assert_eq!(lvm_vgs.get(0).unwrap(), "vg03_backups");
        assert_eq!(lvm_vgs.get(1).unwrap(), "vg04_1tbdisks");
        assert_eq!(lvm_vgs.get(2).unwrap(), "vgdata01");
        assert_eq!(lvm_vgs.get(3).unwrap(), "vgroot");
        assert_eq!(lvm_vgs.len(), 4);

        let s = "";
        let s: std::borrow::Cow<'_, str> = std::borrow::Cow::Borrowed(s);
        let lvm_vgs = parse_vgso(&s).expect("error");
        assert_eq!(lvm_vgs.len(), 0);
    }

    #[test]
    fn test_parse_pvso() {
        ////
        // PV, VG
        // /dev/sda1,vg01
        // /dev/sdx,
        // /dev/sd1

        let s = "  /dev/sda1,vg01\n  /dev/sdx,\n  /dev/sdb2";
        let s: std::borrow::Cow<'_, str> = std::borrow::Cow::Borrowed(s);

        let lvm_pvs = parse_pvso(&s).expect("error");
        assert_eq!(lvm_pvs.get(0).unwrap().pv_name, "/dev/sda1");
        assert_eq!(lvm_pvs.get(0).unwrap().vg_name, "vg01");
        assert_eq!(lvm_pvs.get(1).unwrap().pv_name, "/dev/sdx");
        assert_eq!(lvm_pvs.get(2).unwrap().pv_name, "/dev/sdb2");
        assert_eq!(lvm_pvs.len(), 3);

        // Ngegative test
        let s = "  /dev/sda1,vg01\n  \n  /dev/sdb2";
        let s: std::borrow::Cow<'_, str> = std::borrow::Cow::Borrowed(s);

        let lvm_pvs = parse_pvso(&s).expect("error");
        assert_eq!(lvm_pvs.len(), 2);
    }

    // ex:
    // [lvpub_rmeta_3],vg04_1tbdisks,4194304B,ewi-aor---,linear,qhuhv2-Kdro-dySw-L8d4-uSLJ-8ReD-rgYbD9,1,1
    // lvpub,vg04_1tbdisks,536875106304B,rwi-aor---,raid5,0iPPdB-17pl-7SKc-3rwU-EiBd-10fZ-WheGSZ,4,3
    // [lvpub_rimage_0],vg04_1tbdisks,178958368768B,iwi-aor---,linear,pIfgYg-TSAx-zinr-EyUh-AO8D-VezQ-FUDRGR,1,1
    // [lvpub_rimage_1],vg04_1tbdisks,178958368768B,iwi-aor---,linear,CfoQsY-v1Py-SaN4-KoGF-JmDk-1aNe-Q82Np9,1,1
    // [lvpub_rimage_2],vg04_1tbdisks,178958368768B,iwi-aor---,linear,Z4fbfS-DEJy-SBaU-qo89-XIH0-oRFW-kpOgBj,1,1
    // [lvpub_rimage_3],vg04_1tbdisks,178958368768B,iwi-aor---,linear,XFDDI5-0pL2-SO6Y-NS8P-TJJv-jSGk-KRv1gz,1,1
    // [lvpub_rmeta_0],vg04_1tbdisks,4194304B,ewi-aor---,linear,Rv8iwp-YEGJ-b9V4-ekfe-blD0-5FdB-7pIrhZ,1,1
    // [lvpub_rmeta_1],vg04_1tbdisks,4194304B,ewi-aor---,linear,wgwTeO-cW8E-xHuL-Zt0j-xwAa-F5tj-WnJqm9,1,1
    // [lvpub_rmeta_2],vg04_1tbdisks,4194304B,ewi-aor---,linear,WGltv5-UiaK-n0IT-tLeO-HDyj-jZnx-w0TIrM,1,1
    // [lvpub_rmeta_3],vg04_1tbdisks,4194304B,ewi-aor---,linear,qhuhv2-Kdro-dySw-L8d4-uSLJ-8ReD-rgYbD9,1,1
    #[test]
    fn test_parse_lvso() {
        let s = "  lvbackup,vg03_backups,1073741824000B,-wi-ao----,linear,MlVT0F-L2mW-XCcY-UbJF-QUZJ-MzYe-cQYS9x,1,1,,/dev/sde2:0-255999
  lvpub,vg04_1tbdisks,536875106304B,rwi-aor---,raid5,0iPPdB-17pl-7SKc-3rwU-EiBd-10fZ-WheGSZ,4,3,,[lvpub_rimage_0]:0-42666,[lvpub_rimage_1]:0-42666,[lvpub_rimage_2]:0-42666,[lvpub_rimage_3]:0-42666
  [lvpub_rimage_0],vg04_1tbdisks,178958368768B,iwi-aor---,linear,pIfgYg-TSAx-zinr-EyUh-AO8D-VezQ-FUDRGR,1,1,lvpub,/dev/sdc1:2561-45227
  [lvpub_rimage_1],vg04_1tbdisks,178958368768B,iwi-aor---,linear,CfoQsY-v1Py-SaN4-KoGF-JmDk-1aNe-Q82Np9,1,1,lvpub,/dev/sdd1:1-42667
  [lvpub_rimage_2],vg04_1tbdisks,178958368768B,iwi-aor---,linear,Z4fbfS-DEJy-SBaU-qo89-XIH0-oRFW-kpOgBj,1,1,lvpub,/dev/sde1:1-42667
  [lvpub_rimage_3],vg04_1tbdisks,178958368768B,iwi-aor---,linear,XFDDI5-0pL2-SO6Y-NS8P-TJJv-jSGk-KRv1gz,1,1,lvpub,/dev/sdb1:1-42667
  [lvpub_rmeta_0],vg04_1tbdisks,4194304B,ewi-aor---,linear,Rv8iwp-YEGJ-b9V4-ekfe-blD0-5FdB-7pIrhZ,1,1,lvpub,/dev/sdc1:2560-2560
  [lvpub_rmeta_1],vg04_1tbdisks,4194304B,ewi-aor---,linear,wgwTeO-cW8E-xHuL-Zt0j-xwAa-F5tj-WnJqm9,1,1,lvpub,/dev/sdd1:0-0
  [lvpub_rmeta_2],vg04_1tbdisks,4194304B,ewi-aor---,linear,WGltv5-UiaK-n0IT-tLeO-HDyj-jZnx-w0TIrM,1,1,lvpub,/dev/sde1:0-0
  [lvpub_rmeta_3],vg04_1tbdisks,4194304B,ewi-aor---,linear,qhuhv2-Kdro-dySw-L8d4-uSLJ-8ReD-rgYbD9,1,1,lvpub,/dev/sdb1:0-0";

        let s: std::borrow::Cow<'_, str> = std::borrow::Cow::Borrowed(s);

        let lvm_lvs = parse_lvso(&s).expect("error");

        assert_eq!(lvm_lvs.len(), 10);
    }
}
