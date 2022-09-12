//! Wrappers for SPDK `spdk_bdev` structure and the related API.
use std::{
    ffi::CString,
    fmt::{Debug, Formatter},
    marker::{PhantomData, PhantomPinned},
    os::raw::c_void,
    pin::Pin,
    ptr::{null_mut, NonNull},
};

use nix::errno::Errno;

use crate::{
    ffihelper::{
        errno_result_from_i32,
        AsStr,
        ErrnoResult,
        FfiResult,
        IntoCString,
    },
    libspdk::{
        spdk_bdev,
        spdk_bdev_alias_add,
        spdk_bdev_alias_del,
        spdk_bdev_fn_table,
        spdk_bdev_get_aliases,
        spdk_bdev_get_buf_align,
        spdk_bdev_get_by_name,
        spdk_bdev_has_write_cache,
        spdk_bdev_io_type_supported,
        spdk_bdev_module,
        spdk_bdev_module_release_bdev,
        spdk_bdev_register,
        spdk_bdev_unregister,
        SPDK_BDEV_CLAIM_EXCL_WRITE,
        SPDK_BDEV_CLAIM_NONE,
        spdk_bdev_is_zoned,
        spdk_bdev_get_zone_size,
        spdk_bdev_get_num_zones,
        spdk_bdev_get_max_zone_append_size,
        spdk_bdev_get_max_open_zones,
        spdk_bdev_get_max_active_zones,
        spdk_bdev_get_optimal_open_zones,
    },
    BdevIo,
    BdevModule,
    BdevOps,
    IoChannel,
    IoDevice,
    IoType,
    Thread,
    Uuid,
};

/// Wrapper for SPDK `spdk_bdev` structure and the related API.
/// This wrapper refers to a Bdev, it does not own it: Bdev lifecycle is managed
/// by SPDK. A single Bdev can be refer by multiple `Bdev<>` wrappers.
///
/// # Generic Arguments
///
/// * `BdevData`: TODO
#[derive(Copy)]
pub struct Bdev<BdevData>
where
    BdevData: BdevOps,
{
    /// TODO
    inner: NonNull<spdk_bdev>,
    /// TODO
    _data: PhantomData<BdevData>,
}

unsafe impl<T: BdevOps> Send for Bdev<T> {}

impl<BdevData> Bdev<BdevData>
where
    BdevData: BdevOps,
{
    /// Registers this Bdev in SPDK.
    /// TODO: comment
    /// TODO: Error / result
    pub fn register_bdev(&mut self) -> ErrnoResult<()> {
        let errno = unsafe { spdk_bdev_register(self.as_inner_ptr()) };
        errno_result_from_i32((), errno)
    }

    /// TODO
    pub fn unregister_bdev(&mut self) {
        unsafe {
            spdk_bdev_unregister(
                self.as_inner_ptr(),
                None,
                null_mut::<c_void>(),
            );
        }
    }

    /// Returns a Bdev module for this Bdev.
    pub fn module(&self) -> BdevModule {
        BdevModule::from_ptr(self.as_inner_ref().module)
    }

    /// Returns the name of the module for thos Bdev.
    pub fn module_name(&self) -> &str {
        unsafe { (*self.as_inner_ref().module).name.as_str() }
    }

    /// TODO
    /// ... lookup a bdev by its name
    pub fn lookup_by_name(name: &str) -> Option<Self> {
        assert!(Thread::is_spdk_thread());

        let name = String::from(name).into_cstring();
        let bdev = unsafe { spdk_bdev_get_by_name(name.as_ptr()) };
        if bdev.is_null() {
            None
        } else {
            Some(Self::from_inner_ptr(bdev))
        }
    }

    /// Returns claim module raw pointer.
    #[deprecated(
        note = "Since SPDK 23.05, it is possible to have multiple claim"
    )]
    fn first_claim_module_ptr(&self) -> *mut spdk_bdev_module {
        unsafe {
            let b = self.as_inner_ref().internal;
            match b.claim_type {
                SPDK_BDEV_CLAIM_NONE => std::ptr::null_mut(),
                SPDK_BDEV_CLAIM_EXCL_WRITE => b.claim.v1.module,
                _ => {
                    let c = (*b.claim.v2.claims.tqh_first);
                    assert!(
                        c.link.tqe_next.is_null(),
                        "Multiple claims are not supported"
                    );
                    c.module
                }
            }
        }
    }

    /// Returns by a Bdev module who has claimed this Bdev.
    /// TODO: must returns a list of claims or an iterator of claims.
    #[deprecated(
        note = "Since SPDK 23.05, it is possible to have multiple claim"
    )]
    pub fn first_claim_module(&self) -> Option<BdevModule> {
        let ptr = self.first_claim_module_ptr();
        if ptr.is_null() {
            None
        } else {
            Some(BdevModule::from_ptr(ptr))
        }
    }

    /// Returns by a name of Bdev module who has claimed this Bdev.
    /// TODO: must returns a list of claims or an iterator of claims.
    #[deprecated(
        note = "Since SPDK 23.05, it is possible to have multiple claim"
    )]
    pub fn first_claim_module_name(&self) -> Option<String> {
        self.first_claim_module().map(|m| m.name().to_string())
    }

    /// Returns Bdev name.
    pub fn name(&self) -> &str {
        self.as_inner_ref().name.as_str()
    }

    /// Returns the configured product name.
    pub fn product_name(&self) -> &str {
        self.as_inner_ref().product_name.as_str()
    }

    /// Returns Bdev's raw (SPDK representation) UUID.
    pub fn raw_uuid(&self) -> Uuid {
        Uuid::new(&self.as_inner_ref().uuid)
    }

    /// Sets Bdev's raw (SPDK representation) UUID.
    pub unsafe fn set_raw_uuid(&mut self, uuid: Uuid) {
        self.as_inner_mut().uuid = uuid.into_raw();
    }

    /// Returns Bdev's UUID.
    pub fn uuid(&self) -> uuid::Uuid {
        self.raw_uuid().into()
    }

    /// Returns the UUID of this bdev as a hyphenated string.
    pub fn uuid_as_string(&self) -> String {
        self.uuid().hyphenated().to_string()
    }

    /// TODO
    /// Set a list of aliases on the bdev, used to find the bdev later
    pub fn add_aliases(&mut self, alias: &[String]) -> bool {
        alias
            .iter()
            .filter(|a| -> bool { !self.add_alias(a) })
            .count()
            == 0
    }

    /// TODO
    /// Set an alias on the bdev, this alias can be used to find the bdev later.
    /// If the alias is already present we return true
    pub fn add_alias(&mut self, alias: &str) -> bool {
        let alias = alias.into_cstring();
        let ret =
            unsafe { spdk_bdev_alias_add(self.as_inner_ptr(), alias.as_ptr()) }
                .to_result(Errno::from_i32);

        matches!(ret, Err(Errno::EEXIST) | Ok(_))
    }

    /// Removes the given alias from the Bdev.
    pub fn remove_alias(&mut self, alias: &str) {
        unsafe {
            spdk_bdev_alias_del(
                self.as_inner_ptr(),
                alias.into_cstring().as_ptr(),
            )
        };
    }

    /// Returns a list of Bdev aliases.
    pub fn aliases(&self) -> Vec<String> {
        let mut aliases = Vec::new();
        let head = unsafe { &*spdk_bdev_get_aliases(self.as_inner_ptr()) };
        let mut ent_ptr = head.tqh_first;
        while !ent_ptr.is_null() {
            let ent = unsafe { &*ent_ptr };
            let alias = ent.alias.name.as_str();
            aliases.push(alias.to_string());
            ent_ptr = ent.tailq.tqe_next;
        }
        aliases
    }

    /// Returns the block size of the underlying device.
    pub fn block_len(&self) -> u32 {
        self.as_inner_ref().blocklen
    }

    /// Sets the block size of the underlying device.
    pub unsafe fn set_block_len(&mut self, len: u32) {
        self.as_inner_mut().blocklen = len;
    }

    /// Returns number of blocks for this device.
    pub fn num_blocks(&self) -> u64 {
        self.as_inner_ref().blockcnt
    }

    /// Sets number of blocks for this device.
    pub unsafe fn set_num_blocks(&mut self, count: u64) {
        self.as_inner_mut().blockcnt = count
    }

    /// Returns the Bdev size in bytes.
    pub fn size_in_bytes(&self) -> u64 {
        self.num_blocks() * (self.block_len() as u64)
    }

    /// Returns the alignment of the Bdev.
    pub fn alignment(&self) -> u64 {
        unsafe { spdk_bdev_get_buf_align(self.as_inner_ptr()) }
    }

    /// Returns the required alignment of the Bdev.
    pub fn required_alignment(&self) -> u8 {
        self.as_inner_ref().required_alignment
    }

    /// Returns true if this Bdev is claimed by some other component.
    pub fn is_claimed(&self) -> bool {
        unsafe {
            self.as_inner_ref().internal.claim_type != SPDK_BDEV_CLAIM_NONE
        }
    }

    /// Returns true if this Bdev is claimed by the given component.
    pub fn is_claimed_by(&self, claim_name: &str) -> bool {
        // TODO: properly walk all claims for v2-type claims.
        self.first_claim_module()
            .map_or(false, |m| m.name() == claim_name)
    }

    /// Returns true if this Bdev is claimed by the given Bdev module.
    pub fn is_claimed_by_module(&self, module: &BdevModule) -> bool {
        // TODO: properly walk all claims for v2-type claims.
        self.first_claim_module()
            .map_or(false, |m| m.name() == module.name())
    }

    /// Check whether device has write cache.
    pub fn is_write_cache_enabled(&self) -> bool {
        unsafe { spdk_bdev_has_write_cache(self.as_inner_ptr()) }
    }

    /// Releases a write claim on a block device.
    pub fn release_claim(&self) {
        if self.is_claimed() {
            unsafe {
                spdk_bdev_module_release_bdev(self.as_inner_ptr());
            }
        }
    }

    /// Returns true if this Bdev supports the ZNS command set.
    pub fn is_zoned(&self) -> bool {
        unsafe { spdk_bdev_is_zoned(self.unsafe_inner_ptr()) }
    }

    /// Get device zone size in logical blocks.
    pub fn zone_size(&self) -> u64 {
        unsafe { spdk_bdev_get_zone_size(self.unsafe_inner_ptr()) }
    }

    /// Get the number of zones for the given device.
    pub fn num_zones(&self) -> u64 {
        unsafe { spdk_bdev_get_num_zones(self.unsafe_inner_ptr()) }
    }

    /// Get device maximum zone append data transfer size in logical blocks.
    pub fn max_zone_append_size(&self) -> u32 {
        unsafe { spdk_bdev_get_max_zone_append_size(self.unsafe_inner_ptr()) }
    }

    /// Get device maximum number of open zones.
    pub fn max_open_zones(&self) -> u32 {
        unsafe { spdk_bdev_get_max_open_zones(self.unsafe_inner_ptr()) }
    }

    /// Get device maximum number of active zones.
    pub fn max_active_zones(&self) -> u32 {
        unsafe { spdk_bdev_get_max_active_zones(self.unsafe_inner_ptr()) }
    }

    /// Get device optimal number of open zones.
    pub fn optimal_open_zones(&self) -> u32 {
        unsafe { spdk_bdev_get_optimal_open_zones(self.unsafe_inner_ptr()) }
    }

    /// Determines whenever the Bdev supports the requested I/O type.
    pub fn io_type_supported(&self, io_type: IoType) -> bool {
        if self.is_zoned() && io_type == IoType::ZoneAppend {
            // Always claiming to support zone append such that the exposed
            // NVMe-oF nexus is not set to read only by the kernel. The nexus
            // logic strictly rejects zone append commands.
            return true;
        }
        unsafe {
            spdk_bdev_io_type_supported(self.as_inner_ptr(), io_type.into())
        }
    }

    /// Returns a reference to a data object associated with this Bdev.
    pub fn data<'a>(&self) -> &'a BdevData {
        unsafe {
            let c = self.as_inner_ref().ctxt as *const Container<BdevData>;
            &(*c).data
        }
    }

    /// Returns a mutable reference to a data object associated with this Bdev.
    pub fn data_mut<'a>(&mut self) -> Pin<&'a mut BdevData> {
        unsafe {
            let c = self.as_inner_ref().ctxt as *mut Container<BdevData>;
            Pin::new_unchecked(&mut (*c).data)
        }
    }

    /// Returns a pointer to the underlying `spdk_bdev` structure.
    pub(crate) fn as_inner_ptr(&self) -> *mut spdk_bdev {
        self.inner.as_ptr()
    }

    /// Returns a reference to the underlying `spdk_bdev` structure.
    pub(crate) fn as_inner_ref(&self) -> &spdk_bdev {
        unsafe { self.inner.as_ref() }
    }

    /// Returns a mutable reference to the underlying `spdk_bdev` structure.
    pub(crate) fn as_inner_mut(&mut self) -> &mut spdk_bdev {
        unsafe { self.inner.as_mut() }
    }

    /// Creates a new `Bdev` wrapper from an SPDK structure pointer.
    pub(crate) fn from_inner_ptr(ptr: *mut spdk_bdev) -> Self {
        Self {
            inner: NonNull::new(ptr).unwrap(),
            _data: Default::default(),
        }
    }

    /// Public version of `as_inner_ptr()`.
    pub unsafe fn unsafe_inner_ptr(&self) -> *const spdk_bdev {
        self.as_inner_ptr()
    }

    /// Public version of `as_inner_ptr()`.
    pub unsafe fn unsafe_inner_mut_ptr(&mut self) -> *mut spdk_bdev {
        self.as_inner_ptr()
    }

    /// Public version of `from_inner_ptr()`.
    pub unsafe fn unsafe_from_inner_ptr(ptr: *mut spdk_bdev) -> Self {
        Self::from_inner_ptr(ptr)
    }
}

impl<BdevData> Clone for Bdev<BdevData>
where
    BdevData: BdevOps,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            _data: Default::default(),
        }
    }
}

impl<BdevData> Debug for Bdev<BdevData>
where
    BdevData: BdevOps,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            f.debug_struct("Bdev")
                .field("module", &self.module_name())
                .field("product_name", &self.product_name())
                .field("name", &self.name())
                .field("uuid", &self.uuid_as_string())
                .field("aliases", &self.aliases())
                .field("num_blocks", &self.num_blocks())
                .field("block_len", &self.block_len())
                .field("claimed_by", &self.first_claim_module_name())
                .field("ptr", &unsafe { self.unsafe_inner_ptr() })
                .finish()
        } else {
            write!(
                f,
                "{m}::{name} {sz}x{blk} bytes",
                m = self.module_name(),
                name = self.name(),
                sz = self.num_blocks(),
                blk = self.block_len(),
            )
        }
    }
}

/// Container for the data associated with a `Bdev` instance.
/// This container stores the `spdk_bdev` structure itself,
/// its associated function table and user-defined data structure provided upon
/// Bdev creation.
///
/// When SPDK destructs a BDEV, this container is dropped,
/// automatically freeing all the resources allocated during BDEV creation.
///
/// # Generic Arguments
///
/// * `BdevData`: TODO
#[repr(C)]
pub(crate) struct Container<BdevData>
where
    BdevData: BdevOps,
{
    /// TODO
    pub(crate) bdev: spdk_bdev,
    /// TODO
    pub(crate) fn_table: spdk_bdev_fn_table,
    /// TODO
    pub(crate) data: BdevData,
    /// Prevent auto-Unpin.
    pub(crate) _pin: PhantomPinned,
}

impl<BdevData> Drop for Container<BdevData>
where
    BdevData: BdevOps,
{
    fn drop(&mut self) {
        // Tell the Bdev data object to be cleaned up.
        let pinned_data = unsafe { Pin::new_unchecked(&mut self.data) };
        pinned_data.destruct();

        // Drop the associated strings.
        unsafe {
            CString::from_raw(self.bdev.name);
            CString::from_raw(self.bdev.product_name);
        }
    }
}

impl<BdevData> Container<BdevData>
where
    BdevData: BdevOps,
{
    /// Creates a new container reference from an SPDK Bdev context
    /// pointer.
    ///
    /// # Safety
    ///
    /// TODO
    pub(crate) fn from_ptr<'a>(ctx: *const c_void) -> Pin<&'a Self> {
        unsafe { Pin::new_unchecked(&*(ctx as *const Self)) }
    }

    /// Creates a new mutable container reference from an SPDK Bdev context
    /// pointer.
    ///
    /// # Safety
    ///
    /// TODO
    pub(crate) fn from_ptr_mut<'a>(ctx: *mut c_void) -> Pin<&'a mut Self> {
        unsafe { Pin::new_unchecked(&mut *(ctx as *mut Self)) }
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct BdevZoneInfo {
    /// Indicated if the device to which this ZoneInfo is linked to is a
    /// zoned block device (ZBD) or not. If true, the following fields are
    /// also relavant.
    pub zoned: bool,
    /// Number of zones available on the device.
    pub num_zones: u64,
    /// Size of each zone (in blocks). Typically alligned to a power of 2.
    /// In SPDK the actuall writable zone capacity has to be queried for each
    /// individual zone through a zone report.
    /// zone_capacity <= zone_size.
    /// zone_capacity * num_zones = device capacity
    pub zone_size: u64,
    /// Maximum data transfer size for a single zone append command (in blocks).
    /// Normal (seq) writes must respect the device's general max transfer size.
    pub max_zone_append_size: u32,
    /// Maximum number of open zones for a given device.
    /// This essentially limits the amount of parallel open zones that can be written to.
    /// Refere to NVMe ZNS specification (Figure 7 Zone State Machine) for more details.
    /// https://nvmexpress.org/wp-content/uploads/NVM-Express-Zoned-Namespace-Command-Set-Specification-1.1d-2023.12.28-Ratified.pdf
    pub max_open_zones: u32,
    /// Maximum number of active zones for a given device.
    /// max_open_zones is a subset of max_active_zones. Closed zones are still active until they
    /// get finished (finished zones are in effect immutabel until reset).
    /// Refere to NVMe ZNS specification (Figure 7 Zone State Machine) for more details.
    /// https://nvmexpress.org/wp-content/uploads/NVM-Express-Zoned-Namespace-Command-Set-Specification-1.1d-2023.12.28-Ratified.pdf
    pub max_active_zones: u32,
    /// The drives prefered number of open zones.
    pub optimal_open_zones: u32,
}
