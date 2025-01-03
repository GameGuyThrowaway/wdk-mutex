//! A Rust idiomatic Windows Kernel Driver FAST_MUTEX type which protects the inner type T

use alloc::boxed::Box;
use core::{
    ffi::c_void,
    fmt::Display,
    ops::{Deref, DerefMut},
    ptr::{self, drop_in_place},
};
use wdk::println;
use wdk_sys::{
    ntddk::{
        ExAcquireFastMutex, ExAllocatePool2, ExFreePool, ExReleaseFastMutex, KeGetCurrentIrql, KeInitializeEvent
    }, APC_LEVEL, DISPATCH_LEVEL, FALSE, FAST_MUTEX, FM_LOCK_BIT, POOL_FLAG_NON_PAGED, _EVENT_TYPE::SynchronizationEvent
};

extern crate alloc;

use crate::errors::DriverMutexError;

#[allow(non_snake_case)]
unsafe fn ExInitializeFastMutex(fast_mutex: *mut FAST_MUTEX) {
    // check IRQL
    let irql = unsafe { KeGetCurrentIrql() };
    assert!(irql as u32 <= DISPATCH_LEVEL);

    core::ptr::write_volatile(&mut (*fast_mutex).Count, FM_LOCK_BIT as i32);

    (*fast_mutex).Owner = core::ptr::null_mut();
    (*fast_mutex).Contention = 0;
    KeInitializeEvent(&mut (*fast_mutex).Event, SynchronizationEvent, FALSE as _)
}


/// A thread safe mutex implemented through acquiring a `FAST_MUTEX` in the Windows kernel.
///
/// The type `FastMutex<T>` provides mutually exclusive access to the inner type T allocated through
/// this crate in the non-paged pool. All data required to initialise the FastMutex is allocated in the
/// non-paged pool and as such is safe to pass stack data into the type as it will not go out of scope.
///
/// `FastMutex` holds an inner value which is a pointer to a `FastMutexInner` type which is the actual type
/// allocated in the non-paged pool, and this holds information relating to the mutex.
///
/// Access to the `T` within the `FastMutex` can be done through calling [`Self::lock`].
///
/// To receive debug messages when the IRQL is too high for an operation, enable the feature flag `debug`.
///
/// # Lifetimes
///
/// As the `FastMutex` is designed to be used in the Windows Kernel, with the Windows `wdk` crate, the lifetimes of
/// the `FastMutex` must be considered by the caller. See examples below for usage.
///
/// The `FastMutex` can exist in a locally scoped function with little additional configuration. To use the mutex across
/// thread boundaries, or to use it in callback functions, you can use the `Grt` module found in this crate. See below for 
/// details.
///
/// # Deallocation
///
/// FastMutex handles the deallocation of resources at the point the FastMutex is dropped.
///
/// # Examples
///
/// ## Locally scoped mutex:
///
/// ```
/// {
///     let mtx = FastMutex::new(0u32).unwrap();
///     let lock = mtx.lock().unwrap();
///
///     // If T implements display, you do not need to dereference the lock to print.
///     println!("The value is: {}", lock);
/// } // Mutex will become unlocked as it is managed via RAII
/// ```
///
/// ## Global scope via the `Grt` module in `wdk-mutex`:
///
/// ```
/// // Initialise the mutex on DriverEntry
/// 
/// #[export_name = "DriverEntry"]
/// pub unsafe extern "system" fn driver_entry(
///     driver: &mut DRIVER_OBJECT,
///     registry_path: PCUNICODE_STRING,
/// ) -> NTSTATUS {
///     if let Err(e) = Grt::init() {
///         println!("Error creating Grt!: {:?}", e);
///         return STATUS_UNSUCCESSFUL;
///     }
/// 
///     // ...
///     my_function();
/// }
/// 
/// 
/// // Register a new Mutex in the `Grt` of value 0u32:
/// 
/// pub fn my_function() {
///     Grt::register_mutex("my_test_mutex", 0u32);
/// }
/// 
/// unsafe extern "C" fn my_thread_fn_pointer(_: *mut c_void) {
///     let my_mutex = Grt::get_kmutex::<u32>("my_test_mutex");
///     if let Err(e) = my_mut {
///         println!("Error in thread: {:?}", e);
///         return;
///     }
/// 
///     let mut lock = my_mutex.unwrap().lock().unwrap();
///     *lock += 1;
/// }
/// 
/// 
/// // Destroy the Grt to prevent memory leak on DriverExit
/// 
/// extern "C" fn driver_exit(driver: *mut DRIVER_OBJECT) {
///     unsafe {Grt::destroy()};
/// }
/// ```
pub struct FastMutex<T> {
    inner: *mut FastMutexInner<T>,
}

/// The underlying data which is non-page pool allocated which is pointed to by the `FastMutex`.
struct FastMutexInner<T> {
    mutex: FAST_MUTEX,
    /// The data for which the mutex is protecting
    data: T,
}

unsafe impl<T> Sync for FastMutex<T> {}
unsafe impl<T> Send for FastMutex<T> {}

impl<T> FastMutex<T> {

    /// Creates a new `FAST_MUTEX` Windows Kernel Driver Mutex.
    ///
    /// # IRQL
    ///
    /// This can be called at IRQL <= DISPATCH_LEVEL.
    ///
    /// # Examples
    ///
    /// ```
    /// use wdk_mutex::Mutex;
    ///
    /// let my_mutex = wdk_mutex::KMutex::new(0u32);
    /// ```
    pub fn new(data: T) -> Result<Self, DriverMutexError> {
        // This can only be called at a level <= DISPATCH_LEVEL; check current IRQL
        // https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-exinitializefastmutex
        if unsafe { KeGetCurrentIrql() } > DISPATCH_LEVEL as u8 {
            return Err(DriverMutexError::IrqlTooHigh)
        }

        //
        // Non-Paged heap alloc for all struct data required for FastMutexInner
        //
        let total_sz_required = size_of::<FastMutexInner<T>>();
        let inner_heap_ptr: *mut c_void = unsafe {
            ExAllocatePool2(
                POOL_FLAG_NON_PAGED,
                total_sz_required as u64,
                u32::from_be_bytes(*b"kmtx"),
            )
        };
        if inner_heap_ptr.is_null() {
            return Err(DriverMutexError::PagedPoolAllocFailed);
        }

        // Cast the memory allocation to a pointer to the inner
        let fast_mtx_inner_ptr = inner_heap_ptr as *mut FastMutexInner<T>;

        // SAFETY: This raw write is safe as the pointer validity is checked above.
        unsafe {
            ptr::write(
                fast_mtx_inner_ptr,
                FastMutexInner {
                    mutex: FAST_MUTEX::default(),
                    data,
                },
            );

            // Initialise the FastMutex object via the kernel
            ExInitializeFastMutex(&mut (*fast_mtx_inner_ptr).mutex);
        }

        Ok(Self { inner: fast_mtx_inner_ptr })
    }


    /// Acquires the mutex.
    ///
    /// Once the thread has acquired the mutex, it will return a `FastMutexGuard` which is a RAII scoped
    /// guard allowing exclusive access to the inner T.
    ///
    /// # Errors
    ///
    /// If the IRQL is too high, this function will return an error and will not acquire a lock. To prevent
    /// a kernel panic, the caller should match the return value rather than just unwrapping the value.
    ///
    /// # IRQL
    ///
    /// This function must be called at IRQL `<= APC_LEVEL`, if the IRQL is higher than this,
    /// the function will return an error.
    ///
    /// It is the callers responsibility to ensure the IRQL is sufficient to call this function and it
    /// will not alter the IRQL for the caller, as this may introduce undefined behaviour elsewhere in the
    /// driver / kernel.
    ///
    /// # Examples
    ///
    /// ```
    /// let mtx = KMutex::new(0u32).unwrap();
    /// let lock = mtx.lock().unwrap();
    /// ```
    pub fn lock(&self) -> Result<FastMutexGuard<'_, T>, DriverMutexError> {
        // Check the IRQL is <= APC_LEVEL as per remarks at
        // https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-exacquirefastmutex
        let irql = unsafe { KeGetCurrentIrql() };
        if irql > APC_LEVEL as u8 {
            return Err(DriverMutexError::IrqlTooHigh);
        }

        // SAFETY: RAII manages pointer validity and IRQL checked.
        unsafe { ExAcquireFastMutex(&mut (*self.inner).mutex as *mut _ as *mut _) };

        Ok(FastMutexGuard { fast_mutex: self })
    }

    // todo docs
    pub unsafe fn to_owned(self) -> T {
        let data_read = unsafe { ptr::read(&(*self.inner).data) };
        data_read
    }

    // todo docs
    pub unsafe fn to_owned_box(self) -> Box<T> {
        let data_read = unsafe { ptr::read(&(*self.inner).data) };
        Box::new(data_read)
    }
}

impl<T> Drop for FastMutex<T> {
    fn drop(&mut self) {
        unsafe {
            // Drop the underlying data and run destructors for the data, this would be relevant in the
            // case where Self contains other heap allocated types which have their own deallocation
            // methods.
            drop_in_place(&mut (*self.inner).data);

            // Free the memory we allocated
            ExFreePool(self.inner as *mut _);
        }
    }
}


// todo docs
pub struct FastMutexGuard<'a, T> {
    fast_mutex: &'a FastMutex<T>,
}

impl<T> Display for FastMutexGuard<'_, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // SAFETY: Dereferencing the inner data is safe as RAII controls the memory allocations.
        write!(f, "{}", unsafe { &(*self.fast_mutex.inner).data })
    }
}


impl<T> Deref for FastMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Dereferencing the inner data is safe as RAII controls the memory allocations.
        unsafe { &(*self.fast_mutex.inner).data }
    }
}

impl<T> DerefMut for FastMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Dereferencing the inner data is safe as RAII controls the memory allocations.
        // Mutable access is safe due to Self only being given out whilst a mutex is held from the
        // kernel.
        unsafe { &mut (*self.fast_mutex.inner).data }
    }
}

impl<T> Drop for FastMutexGuard<'_, T> {
    fn drop(&mut self) {
        // NOT SAFE AT A IRQL TOO HIGH
        unsafe { ExReleaseFastMutex(&mut (*self.fast_mutex.inner).mutex) }; 
    }
}

impl<T> FastMutexGuard<'_, T> {
    // todo docs
    pub fn drop_safe(&mut self) -> Result<(), DriverMutexError> {
        let irql = unsafe { KeGetCurrentIrql() };
        if irql > DISPATCH_LEVEL as u8 {
            if cfg!(feature = "debug") {
                println!("[wdk-mutex] [-] Unable to safely drop the KMUTEX. Calling IRQL is too high: {}", irql);
            }
            return Err(DriverMutexError::IrqlTooHigh);
        }

        unsafe { ExReleaseFastMutex(&mut (*self.fast_mutex.inner).mutex) }; 

        Ok(())
    }
}