
pub struct NameValue {
   pub name: String,
   pub value: String,
}

pub fn get_vg_info(vg_name: &String) -> Vec<NameValue> {
    vec![
        NameValue {
            name: "VG Name".to_string(),
            value: vg_name.to_string(),
        },
        NameValue {
            name: "Format".to_string(),
            value: "lvm2".to_string(),
        },
    ]
}