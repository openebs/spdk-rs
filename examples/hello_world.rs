use std::{
    ffi::CString,
    mem::zeroed,
    os::raw::{c_char, c_int, c_void},
};

use spdk_rs::libspdk::{
    size_t,
    spdk_app_fini,
    spdk_app_opts,
    spdk_app_opts_init,
    spdk_app_parse_args,
    spdk_app_start,
    spdk_app_stop,
    SPDK_APP_PARSE_ARGS_SUCCESS,
};

extern "C" fn hello_world_parse_arg(_ch: c_int, _arg: *mut c_char) -> c_int {
    0
}

extern "C" fn hello_world_usage() {}

extern "C" fn hello_world_start(_arg: *mut c_void) {
    println!("hello world started!");

    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("hello world ended");

    unsafe {
        spdk_app_stop(0);
    }
}

fn main() {
    unsafe {
        let args = std::env::args()
            .map(|arg| CString::new(arg).unwrap())
            .collect::<Vec<CString>>();

        let mut c_args = args
            .iter()
            .map(|arg| arg.as_ptr())
            .collect::<Vec<*const c_char>>();

        c_args.push(std::ptr::null());

        // Set default values in opts structure.
        let mut opts: spdk_app_opts = zeroed();
        spdk_app_opts_init(
            &mut opts as *mut _,
            std::mem::size_of::<spdk_app_opts>() as size_t,
        );

        opts.name = CString::new("hello_world_test".to_owned())
            .unwrap()
            .into_raw();

        // Parse built-in SPDK command line parameters as well
        // as our custom one(s).
        let rc = spdk_app_parse_args(
            (c_args.len() as c_int) - 1,
            c_args.as_ptr() as *mut *mut c_char,
            &mut opts as *mut _,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            Some(hello_world_parse_arg),
            Some(hello_world_usage),
        );
        assert_eq!(rc, SPDK_APP_PARSE_ARGS_SUCCESS);

        println!("spdk_app_start()");
        let rc = spdk_app_start(
            &mut opts as *mut _,
            Some(hello_world_start),
            std::ptr::null_mut(),
        );
        assert_eq!(rc, 0);

        println!("spdk_app_fini()");
        spdk_app_fini();
    }
}
