//! A Rust idiomatic Windows Kernel Driver KMUTEX type which protects the inner type T

use core::{ffi::c_void, fmt::Display, ops::{Deref, DerefMut}, ptr::{self, drop_in_place, null_mut}};
use wdk::println;
use wdk_sys::{ntddk::{ExAllocatePool2, ExFreePool, KeGetCurrentIrql, KeInitializeMutex, KeReleaseMutex, KeWaitForSingleObject}, APC_LEVEL, DISPATCH_LEVEL, FALSE, KMUTEX, POOL_FLAG_NON_PAGED, _KWAIT_REASON::Executive, _MODE::KernelMode};

use crate::errors::DriverMutexError;
/// A thread safe mutex implemented through acquiring a KMUTEX in the Windows kernel.
/// 
/// The type `Kmutex<T>` provides mutually exclusive access to the inner type T allocated through 
/// this crate in the non-paged pool. All data required to initialise the KMutex is allocated in the 
/// non-paged pool and as such is safe to pass stack data into the type as it will not go out of scope.
/// 
/// `KMutex` holds an inner value which is a pointer to a `KMutexInner` type which is the actual type
/// allocated in the non-paged pool, and this holds information relating to the mutex.
/// 
/// Access to the `T` within the `KMutex` can be done through calling [`Self::lock`].
/// 
/// To receive debug messages when the IRQL is too high for an operation, enable the feature flag `debug`.
/// 
/// # Lifetimes
/// 
/// As the `KMutex` is designed to be used in the Windows Kernel, with the Windows `wdk` crate, the lifetimes of 
/// the `KMutex` must be considered by the caller. See examples below for usage.
/// 
/// The KMutex can exist in a locally scoped function with little additional configuration. To use the mutex across
/// thread boundaries, or to use it in callback functions, the recommended course of action is to utilise either a 
/// globally accessible `static AtomicPtr<KMutex<T>>`; or to utilise a 
/// [Device Extension](https://learn.microsoft.com/en-us/windows-hardware/drivers/kernel/device-extensions) 
/// provided in the wdk.
/// 
/// <section class="warning">
/// If you use a `static AtomicPtr<KMutex<T>>` you MUST ensure that the memory is cleaned up when you exit the driver
/// otherwise you cause a memory leak.
/// 
/// A future addition is planned which will make the API more flexible for dynamically managing globally available 
/// mutexes to somewhat reduce the overhead required to use this crate.
/// </section>
/// 
/// # Deallocation
/// 
/// KMutex handles the deallocation of resources at the point the KMutex is dropped.
/// 
/// # Examples
/// 
/// ## Locally scoped mutex:
/// 
/// ```
/// {
///     let mtx = KMutex::new(0u32).unwrap();
///     let lock = mtx.lock().unwrap();
/// 
///     // If T implements display, you do not need to dereference the lock to print.
///     println!("The value is: {}", lock);
/// } // Mutex will become unlocked as it is managed via RAII 
/// ```
/// 
/// ## Global scope via static pointer:
/// 
/// A future release is planned to make this process more ergonomic.
/// 
/// ```
/// pub static HEAP_MTX_PTR: AtomicPtr<KMutex<u32>> = AtomicPtr::new(null_mut());
/// 
/// fn my_fn() {
///     let heap_mtx = Box::new(KMutex::new(0u32).unwrap());
///     let heap_mtx_ptr = Box::into_raw(heap_mtx);
///     HEAP_MTX_PTR.store(heap_mtx_ptr, Ordering::SeqCst);
/// 
///     // spawn some system threads
///     r _ in 0..3 {
///         let mut thread_handle: HANDLE = null_mut();
///         let status = unsafe {
///             PsCreateSystemThread(
///                 &mut thread_handle, 
///                 0, 
///                 null_mut::<OBJECT_ATTRIBUTES>(), 
///                 null_mut(),
///                 null_mut::<CLIENT_ID>(), 
///                 Some(callback_fn), 
///                 null_mut(),
///             )
///         };
///         println!("[i] Thread status: {status}");
///     }
/// }
/// 
/// unsafe extern "C" fn callback_fn(_: *mut c_void) {
///     for _ in 0..50 {
///         let p = HEAP_MTX_PTR.load(Ordering::SeqCst);
///         if !p.is_null() {
///             let p = unsafe { &*p };
///             let mut lock = p.lock().unwrap();
///             println!("Got the lock before change! {}", *lock);
///             *lock += 1;
///             println!("After the change: {}", *lock);
///         }
///     }
/// }
/// 
/// // IMPORTANT ensure the KMutex in the static is properly dropped to clean memory
/// extern "C" fn driver_exit(driver: *mut DRIVER_OBJECT) {
///     let ptr: *mut KMutex<u32> = HEAP_MTX_PTR.load(Ordering::SeqCst);
///     if !ptr.is_null() {
///         unsafe {
///             // RAII will kick in here to deallocate our memory
///             let _ = Box::from_raw(ptr);
///         }
///     }
/// }
/// ```
pub struct KMutex<T> {
    inner: *mut KMutexInner<T>
}

/// The underlying data which is non-page pool allocated which is pointed to by the `KMutex`.
struct KMutexInner<T> {
    /// A KMUTEX structure allocated into KMutexInner
    mutex: KMUTEX,
    /// The data for which the mutex is protecting
    data: T,
}


unsafe impl<T: Sync> Sync for KMutex<T>{}

impl<T> KMutex<T> {

    /// Creates a new KMUTEX Windows Kernel Driver Mutex in a signaled (free) state.
    /// 
    /// # IRQL
    /// 
    /// This can be called at any IRQL.
    /// 
    /// # Examples
    /// 
    /// ```
    /// use wdk_mutex::Mutex;
    /// 
    /// let my_mutex = wdk_mutex::KMutex::new(0u32);
    /// ```
    pub fn new(data: T) -> Result<Self, DriverMutexError> {
        //
        // Non-Paged heap alloc for all struct data required for KMutexInner
        //
        let total_sz_required = size_of::<KMutexInner<T>>();
        let inner_heap_ptr: *mut c_void = unsafe {
            ExAllocatePool2(POOL_FLAG_NON_PAGED, total_sz_required as u64, u32::from_be_bytes(*b"kmtx"))
        };
        if inner_heap_ptr.is_null() {
            return Err(DriverMutexError::PagedPoolAllocFailed);
        }

        // Cast the memory allocation to a pointer to the inner
        let kmutex_inner_ptr = inner_heap_ptr as *mut KMutexInner<T>;

        // SAFETY: This raw write is safe as the pointer validity is checked above.
        unsafe {
            ptr::write(
                kmutex_inner_ptr,
                KMutexInner { 
                    mutex: KMUTEX::default(), 
                    data
                }
            );

            // Initialise the KMUTEX object via the kernel
            KeInitializeMutex(&(*kmutex_inner_ptr).mutex as *const _ as *mut _, 0);
        }

        Ok(Self { inner: kmutex_inner_ptr })
    }


    /// Acquires a mutex in a non-alertable manner. 
    /// 
    /// A future release is planned to include an alternate implementation 
    /// which will lock the mutex and become alertable if it has to wait for the mutex to become free.
    /// This function will block the local thread until it is available to acquire the mutex.
    /// 
    /// Once the thread has acquired the mutex, it will return a `KMutexGuard` which is a RAII scoped 
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
    pub fn lock(&self) -> Result<KMutexGuard<'_, T>, DriverMutexError> {

        // Check the IRQL is <= APC_LEVEL as per remarks at
        // https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-kewaitforsingleobject
        let irql = unsafe {KeGetCurrentIrql()};
        if irql > APC_LEVEL as u8 {
            if cfg!(feature = "debug") {
                println!("[wdk-mutex] [-] IRQL is too high to call .lock(). Current IRQL: {}", irql);
                return Err(DriverMutexError::IrqlTooHigh);
            }
        }

        // Discard the return value; the status code does not represent an error or contain information 
        // relevant to the context of no timeout.
        let _ = unsafe { 
            // SAFETY: The IRQL is sufficient for the operation as checked above, and we know our pointer
            // is valid as RAII manages the lifetime of the heap allocation, ensuring it will only be deallocated
            // once Self gets dropped.
            KeWaitForSingleObject(
                &mut (*self.inner).mutex as *mut _ as *mut _,
                Executive, 
                KernelMode as i8,
                FALSE as u8, 
                null_mut(),
            ) 
        };
         
        Ok(KMutexGuard {
            kmutex: self
        })
    }
}

impl<T> Drop for KMutex<T> {
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


/// A RAII scoped guard for the inner data protected by the mutex. Once this guard is given out, the protected data
/// may be safely mutated by the caller as we guarantee exclusive access via Windows Kernel Mutex primitives.
/// 
/// When this structure is dropped (falls out of scope), the lock will be unlocked.
/// 
/// # IRQL
/// 
/// Access to the data within this guard must be done at <= APC_LEVEL if a non-alertable lock was acquired, or <= 
/// DISPATCH_LEVEL if an alertable lock was acquired. It is the callers responsible to manage APC levels whilst
/// using the KMutex.
/// 
/// If you wish to manually drop the lock with a safety check, call the function [`Self::drop_safe`].
/// 
/// # Kernel panic
/// 
/// Raising the IRQL above safe limits whilst using the mutex will cause a Kernel Panic if not appropriately handled.
/// When RAII drops this type, the mutex is released, if the mutex goes out of scope whilst you hold an IRQL that
/// is too high, you will receive a kernel panic.
///
pub struct KMutexGuard<'a, T> {
    kmutex: &'a KMutex<T>,
}

impl<'a, T> Display for KMutexGuard<'a, T> 
where T: Display
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // SAFETY: Dereferencing the inner data is safe as RAII controls the memory allocations.
        write!(f, "{}", unsafe { &(*self.kmutex.inner).data })
    }
}


impl<'a, T> Deref for KMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Dereferencing the inner data is safe as RAII controls the memory allocations.
        unsafe { &(*self.kmutex.inner).data }
    }
}

impl<'a, T> DerefMut for KMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Dereferencing the inner data is safe as RAII controls the memory allocations.
        // Mutable access is safe due to Self only being given out whilst a mutex is held from the
        // kernel.
        unsafe { &mut (*self.kmutex.inner).data }
    }
}

impl<'a, T> Drop for KMutexGuard<'a, T> {
    fn drop(&mut self) {
        // NOT SAFE AT A IRQL TOO HIGH
        unsafe { KeReleaseMutex(&mut (*self.kmutex.inner).mutex, FALSE as u8) }; 
    }
}

impl<'a, T> KMutexGuard<'a, T> {
    /// Safely drop the KMutexGuard, an alternative to RAII.
    /// 
    /// This function checks the IRQL before attempting to drop the guard. 
    /// 
    /// # Errors
    /// 
    /// If the IRQL > DISPATCH_LEVEL, no unlock will occur and a DriverMutexError will be returned to the 
    /// caller.
    /// 
    /// # IRQL
    /// 
    /// This function is safe to call at any IRQL, but it will not release the mutex if IRQL > DISPATCH_LEVEL
    pub fn drop_safe(&mut self) -> Result<(), DriverMutexError> {

        let irql = unsafe {KeGetCurrentIrql()};
        if irql > DISPATCH_LEVEL as u8 {
            if cfg!(feature = "debug") {
                println!("[wdk-mutex] [-] Unable to safely drop the KMUTEX. Calling IRQL is too high: {}", irql);
                return Err(DriverMutexError::IrqlTooHigh);
            }
        }

        unsafe { KeReleaseMutex(&mut (*self.kmutex.inner).mutex, FALSE as u8) }; 

        Ok(())
    }
}