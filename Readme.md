# wdk-mutex

An idiomatic Rust mutex type for Windows kernel driver development.

To use this crate, simply:

```
cargo add wdk-mutex
```

**See the sections below for examples**. This crate assumes you already have a Rust Windows Driver Kit project
using the Microsoft [windows-drivers-rs](https://github.com/microsoft/windows-drivers-rs) crate. You will
not be able to use this crate unless you are in a wdk development environment, setup steps outlined on
their repository.

This crate checks IRQL when acquiring and releasing the mutex, returning an error if IRQL is too high. See the crate documentation for details.

`wdk-mutex` is a work in progress to implement additional mutex types and functionality as required. Contributions, issues, and discussions are welcome. This crate is **NOT** affiliated with the WDK crates provided by Microsoft, but is designed to work with them and their ecosystem for Windows Rust 
Kernel Driver development.

As this crate integrates into the wdk ecosystem, Microsoft stipulate: This project is still in early stages of development and is not yet recommended for production use.

Tests have been conducted on public modules, but are not included in the crate due to the complexity of 
deploying kernel level tests. A new repo will be created specifically for testing this crate which 
can be built as a driver.

This is licenced with an MIT Licence, conditions can be found in LICENCE in the crate GitHub repository.

## Stability

This crate has been built and tested with **nightly-2024-12-21**. Stability outside of nightly versions
stipulated here are considered undefined.

The crate has been tested on a Windows 11 image as a driver. No testing of other Windows versions have 
been conducted.

## Features:

**KMUTEX:** 

The `wdk-mutex` crate supports acquiring a KMUTEX. The type `Kmutex<T>` provides mutually exclusive access
to the inner type T allocated through this crate in the non-paged pool. All data required to initialise the 
KMutex is allocated in the non-paged pool and as such is safe to pass stack data into the type as it will not go out of scope.

Access to the `T` within the `KMutex` can be done through calling `lock`, similar to the Rust std Mutex.

As the `KMutex` is designed to be used in the Windows Kernel, with the Windows `wdk` crate, the lifetimes of 
the `KMutex` must be considered by the caller. See **examples** below for usage.

# Examples

## Locally scoped mutex:

```rust
{
    let mtx = KMutex::new(0u32).unwrap();
    let lock = mtx.lock().unwrap();

    // If T implements display, you do not need to dereference the lock to print.
    println!("The value is: {}", lock);
} // Mutex will become unlocked as it is managed via RAII 
```

## Global scope via static pointer:

A future release is planned to make this process more ergonomic.

```rust
pub static HEAP_MTX_PTR: AtomicPtr<KMutex<u32>> = AtomicPtr::new(null_mut());

fn my_fn() {
    let heap_mtx = Box::new(KMutex::new(0u32).unwrap());
    let heap_mtx_ptr = Box::into_raw(heap_mtx);
    HEAP_MTX_PTR.store(heap_mtx_ptr, Ordering::SeqCst);

    // spawn some system threads
    r _ in 0..3 {
        let mut thread_handle: HANDLE = null_mut();
        let status = unsafe {
            PsCreateSystemThread(
                &mut thread_handle, 
                0, 
                null_mut::<OBJECT_ATTRIBUTES>(), 
                null_mut(),
                null_mut::<CLIENT_ID>(), 
                Some(callback_fn), 
                null_mut(),
            )
        };
        println!("[i] Thread status: {status}");
    }
}

unsafe extern "C" fn callback_fn(_: *mut c_void) {
    for _ in 0..50 {
        let p = HEAP_MTX_PTR.load(Ordering::SeqCst);
        if !p.is_null() {
            let p = unsafe { &*p };
            let mut lock = p.lock().unwrap();
            println!("Got the lock before change! {}", *lock);
            *lock += 1;
            println!("After the change: {}", *lock);
        }
    }
}

// IMPORTANT ensure the KMutex in the static is properly dropped to clean memory
extern "C" fn driver_exit(driver: *mut DRIVER_OBJECT) {
    let ptr: *mut KMutex<u32> = HEAP_MTX_PTR.load(Ordering::SeqCst);
    if !ptr.is_null() {
        unsafe {
            // RAII will kick in here to deallocate our memory
            let _ = Box::from_raw(ptr);
        }
    }
}
```

# Planned updates

## Global interface:

A future addition is planned which will make the API more flexible for dynamically managing globally 
available mutexes to somewhat reduce the overhead required to use this crate.

## Critical Sections:

An idiomatic implementation for entering and leaving a mutex critical section where no underlying 
T is protected.

## FAST_MUTEX

An idiomatic implementation for FAST_MUTEX.

The next planned release will add Critical Section behaviour, where you do not want to wrap a generic T in a mutex (similar to std::mutex), but you want a section to be a critical section nonetheless.

Please note that each planned feature will be introduced gradually, and might undergo changes based on community feedback. 

I welcome any contributions or suggestions to improve functionality, documentation, and compatibility.