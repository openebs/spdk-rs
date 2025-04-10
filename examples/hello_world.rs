use std::{
    ffi::{CStr, CString},
    mem::zeroed,
    os::raw::c_char,
};

use spdk_rs::libspdk::{
    spdk_env_fini, spdk_env_init, spdk_env_opts, spdk_env_opts_init, spdk_nvme_ctrlr,
    spdk_nvme_ctrlr_get_first_active_ns, spdk_nvme_ctrlr_get_next_active_ns,
    spdk_nvme_ctrlr_get_ns, spdk_nvme_ctrlr_opts, spdk_nvme_detach_async, spdk_nvme_detach_ctx,
    spdk_nvme_detach_poll, spdk_nvme_ns, spdk_nvme_ns_is_active, spdk_nvme_probe, spdk_nvme_qpair,
    spdk_nvme_transport_id, spdk_nvme_trid_populate_transport, SPDK_NVME_TRANSPORT_PCIE,
};
use std::{collections::VecDeque, ptr::addr_of_mut};

#[allow(non_camel_case_types)]
struct ctrlr_entry {
    ctrlr: *mut spdk_nvme_ctrlr,
    // name: [u8; 1024],
}

#[allow(non_camel_case_types)]
#[allow(dead_code)]
struct ns_entry {
    ctrlr: *mut spdk_nvme_ctrlr,
    ns: *mut spdk_nvme_ns,
    qpair: Option<spdk_nvme_qpair>,
}
static mut G_CONTROLLERS: VecDeque<ctrlr_entry> = VecDeque::new();
static mut G_NAMESPACES: VecDeque<ns_entry> = VecDeque::new();
static mut G_TRID: spdk_nvme_transport_id = unsafe { zeroed() };

extern "C" fn probe_cb(
    _cb_ctx: *mut ::std::os::raw::c_void,
    _trid: *const spdk_nvme_transport_id,
    _opts: *mut spdk_nvme_ctrlr_opts,
) -> bool {
    println!("Attaching to {}", unsafe {
        CStr::from_ptr((*_trid).traddr.as_ptr()).to_str().unwrap()
    });

    true
}

extern "C" fn attach_cb(
    _cb_ctx: *mut ::std::os::raw::c_void,
    _trid: *const spdk_nvme_transport_id,
    ctrlr: *mut spdk_nvme_ctrlr,
    _opts: *const spdk_nvme_ctrlr_opts,
) {
    println!("Attached to {}", unsafe {
        CStr::from_ptr((*_trid).traddr.as_ptr()).to_str().unwrap()
    });

    let mut ns: *mut spdk_nvme_ns;
    let entry: ctrlr_entry = ctrlr_entry {
        ctrlr,
        // name: [0; 1024],
    };
    unsafe {
        G_CONTROLLERS.push_back(entry);
    }

    // let cdata: *const spdk_nvme_ctrlr_data = unsafe { spdk_nvme_ctrlr_get_data(ctrlr) };
    // entry.name = unsafe {
    //     let name = CString::from_raw((*cdata).mn.as_ptr() as *mut c_char);
    //     name.into_bytes()
    // };

    unsafe {
        let mut nsid = spdk_nvme_ctrlr_get_first_active_ns(ctrlr);
        while nsid != 0 {
            ns = spdk_nvme_ctrlr_get_ns(ctrlr, nsid);
            if ns.is_null() {
                continue;
            }

            if !spdk_nvme_ns_is_active(ns) {
                nsid = spdk_nvme_ctrlr_get_next_active_ns(ctrlr, nsid);
                continue;
            }

            G_NAMESPACES.push_back(ns_entry {
                ctrlr,
                ns,
                qpair: None,
            });
            nsid = spdk_nvme_ctrlr_get_next_active_ns(ctrlr, nsid);
        }
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

        spdk_nvme_trid_populate_transport(addr_of_mut!(G_TRID), SPDK_NVME_TRANSPORT_PCIE);

        // Set default values in opts structure.
        let mut opts: spdk_env_opts = zeroed();
        spdk_env_opts_init(&mut opts as *mut _);

        let rc = spdk_env_init(&opts);
        println!("spdk_env_init rc: {}", rc);
        if rc < 0 {
            panic!("spdk_env_init failed");
        }
        let rc = spdk_nvme_probe(
            addr_of_mut!(G_TRID),
            std::ptr::null_mut(),
            Some(probe_cb),
            Some(attach_cb),
            None,
        );
        assert_eq!(rc, 0);

        if G_CONTROLLERS.is_empty() {
            // TODO: need to clear more gracefully
            panic!("no NVMe controllers found");
        }

        println!("Found {} controllers", G_CONTROLLERS.len());

        let mut detach_ctx: *mut spdk_nvme_detach_ctx = std::ptr::null_mut();
        for entry in G_CONTROLLERS.iter() {
            spdk_nvme_detach_async(entry.ctrlr, &mut detach_ctx);
        }

        if !detach_ctx.is_null() {
            spdk_nvme_detach_poll(detach_ctx);
        }
        spdk_env_fini();
    }
}
