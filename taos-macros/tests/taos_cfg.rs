use taos_macros::c_cfg;

#[c_cfg(v3)]
unsafe extern "C" {
    fn test_cfg(_a: usize, _b: *mut usize);
}

#[c_cfg(not(v3))]
unsafe extern "C" {
    fn test_cfg_enabled(_a: usize);
}
