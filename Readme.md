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

All tests are carried out at [wdk_mutex_tests](https://github.com/0xflux/wdk_mutex_tests), 
a separate repository which is built as a driver to test all functionality of the `wdk-mutex` crate. If you wish to run the tests
yourself, or contribute, please check that repository.

This is licenced with an MIT Licence, conditions can be found in LICENCE in the crate GitHub repository.

## Features:

**Global mutex tracking:**

The `wdk-mutex` crate allows you to easily create, track, and use Mutex's for a Windows Kernel Driver across all threads and 
callbacks. This improves developer ergonomics in creating, tracking, dropping etc Mutex's throughout your drivers codebase. 

To use it, the `Grt` (Global Reference Tracker) module will allocate a safe reference tracker, allowing you to then register a `T` to be protected by a Mutex. This
registration takes a `&'static str` which is the key for the Mutex object. At runtime, you are able to safely retrieve the mutex and operate
on the inner T across threads and callbacks.

The `Grt` will also **prevent memory leaks** as it implements a `destroy()` function which you call only once, and all data tracked by the `Grt` will be deallocated, 
making life much easier tracking global static data in a driver.

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

## Global scope via wdk-mutex Grt:

To use Mutex's across your driver at runtime with ease:

```rust
// Initialise the mutex on DriverEntry

#[export_name = "DriverEntry"]
pub unsafe extern "system" fn driver_entry(
    driver: &mut DRIVER_OBJECT,
    registry_path: PCUNICODE_STRING,
) -> NTSTATUS {
    if let Err(e) = Grt::init() {
        println!("Error creating Grt!: {:?}", e);
        return STATUS_UNSUCCESSFUL;
    }

    // ...
    my_function();
}


// Register a new Mutex in the `Grt` of value 0u32:

pub fn my_function() {
    Grt::register_mutex("my_test_mutex", 0u32);
}

unsafe extern "C" fn my_thread_fn_pointer(_: *mut c_void) {
    let my_mutex = Grt::get_kmutex::<u32>("my_test_mutex");
    if let Err(e) = my_mut {
        println!("Error in thread: {:?}", e);
        return;
    }

    let mut lock = my_mutex.unwrap().lock().unwrap();
    *lock += 1;
}


// Destroy the Grt to prevent memory leak on DriverExit

extern "C" fn driver_exit(driver: *mut DRIVER_OBJECT) {
    unsafe {Grt::destroy()};
}
```

# Stability

This crate has been built and tested with **nightly-2024-12-21**. Stability outside of nightly versions
stipulated here are considered undefined.

The crate has been tested on a Windows 11 image as a driver. No testing of other Windows versions have 
been conducted.

# Planned updates

## Critical Sections:

An idiomatic implementation for entering and leaving a mutex critical section where no underlying 
T is protected.

## FAST_MUTEX

An idiomatic implementation for FAST_MUTEX.

The next planned release will add Critical Section behaviour, where you do not want to wrap a generic T in a mutex (similar to std::mutex), but you want a section to be a critical section nonetheless.

Please note that each planned feature will be introduced gradually, and might undergo changes based on community feedback. 

I welcome any contributions or suggestions to improve functionality, documentation, and compatibility.